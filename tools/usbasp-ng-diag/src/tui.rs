//! Ratatui watch: faceplate + log + host VERDICT rail. TUI is a viewer, not the analyzer.

use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Padding, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use ratatui::Terminal;
use std::io::{self, IsTerminal, Stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::capture::CaptureFile;
use crate::correlate::{self, TimelineEvent};
use crate::demo;
use crate::decoder::type_name;
use crate::protocol::{DiagFrame, EP2_IN, MEM_EEPROM, MEM_FLASH, MEM_READFLASH};
use crate::scene::{
    diagnosis_at, dual_rows, is_wire_fragment, phases, programmer_rows, rel_label, DiagTone,
    PhaseMark, ViewRow,
};
use crate::state::{AppState, Level};
use crate::usb::CompositeHandle;

pub enum WatchSource {
    File(PathBuf),
    Demo(String),
    Live { serial: String },
    Jsonl(PathBuf),
}

struct Ui {
    state: AppState,
    source_label: String,
    faults_only: bool,
    show_caps: bool,
    wire: bool,
    table_state: TableState,
    follow: bool,
    status: String,
    uart_path: Option<PathBuf>,
    timeline: Vec<TimelineEvent>,
    confirm_clear: bool,
    usb_want: Option<String>,
    usb_path: String,
    usb_serial: String,
    usb_link: UsbLink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UsbLink {
    Offline,
    Awaiting,
    Connected,
    Dropped,
}

impl Ui {
    fn new(source_label: String, state: AppState, uart_path: Option<PathBuf>) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        let mut ui = Self {
            state,
            source_label,
            faults_only: false,
            show_caps: false,
            wire: false,
            table_state,
            follow: true,
            status: String::new(),
            uart_path,
            timeline: Vec::new(),
            confirm_clear: false,
            usb_want: None,
            usb_path: String::new(),
            usb_serial: String::new(),
            usb_link: UsbLink::Offline,
        };
        ui.refresh_timeline();
        ui.jump_bot();
        ui
    }

    fn refresh_timeline(&mut self) {
        let Some(path) = &self.uart_path else {
            self.timeline.clear();
            return;
        };
        if self.state.events.is_empty() {
            self.timeline.clear();
            return;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let prog: Vec<(u64, String, String)> = self
            .state
            .events
            .iter()
            .filter(|e| self.wire || !is_wire_fragment(e))
            .map(|e| (e.host_ns, type_name(e.ty).to_string(), e.text.clone()))
            .collect();
        if let Ok(ev) = correlate::merge_programmer_and_uart(prog, &text) {
            self.timeline = ev;
        }
    }

    fn rows(&self) -> Vec<ViewRow> {
        if self.uart_path.is_some() && !self.timeline.is_empty() {
            let t0 = self.state.events.first().map(|e| e.host_ns);
            dual_rows(&self.timeline, t0, self.faults_only)
        } else {
            programmer_rows(&self.state, self.wire, self.faults_only)
        }
    }

    fn scroll_rel(&mut self, delta: isize) {
        self.follow = false;
        let n = self.rows().len();
        if n == 0 {
            self.table_state.select(None);
            return;
        }
        let cur = self.table_state.selected().unwrap_or(0);
        let next = cur.saturating_add_signed(delta).min(n.saturating_sub(1));
        self.table_state.select(Some(next));
    }

    fn jump_top(&mut self) {
        self.follow = false;
        if self.rows().is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    fn jump_bot(&mut self) {
        self.follow = true;
        let n = self.rows().len();
        if n == 0 {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(n - 1));
        }
    }

    fn on_new_events(&mut self) {
        self.refresh_timeline();
        if self.follow {
            self.jump_bot();
        }
    }

    fn clear_screen(&mut self) {
        self.state = AppState::default();
        self.timeline.clear();
        self.show_caps = false;
        self.status.clear();
        self.follow = true;
        self.jump_bot();
    }
}

pub fn run(source: WatchSource, uart: Option<PathBuf>) -> Result<()> {
    require_interactive_tty()?;
    let (label, state, live) = match source {
        WatchSource::File(path) => {
            let cap = CaptureFile::load(&path)?;
            let mut st = AppState::default();
            st.ingest_capture(&cap);
            (format!("file:{}", path.display()), st, None)
        }
        WatchSource::Demo(name) => {
            let cap = demo::build_scenario(&name)?;
            let mut st = AppState::default();
            st.ingest_capture(&cap);
            (format!("demo:{name}"), st, None)
        }
        WatchSource::Jsonl(path) => {
            let text = std::fs::read_to_string(&path)?;
            let mut st = AppState::default();
            st.ingest_jsonl(&text)?;
            (format!("jsonl:{}", path.display()), st, None)
        }
        WatchSource::Live { serial } => {
            let live = crate::usb::try_open_composite(&serial)?;
            let (label, link, path, ser) = match &live {
                Some(h) => (
                    format!("live:{}", h.serial),
                    UsbLink::Connected,
                    h.path.clone(),
                    h.serial.clone(),
                ),
                None => (
                    format!("live:{serial}"),
                    UsbLink::Awaiting,
                    "—".into(),
                    serial.clone(),
                ),
            };
            let mut ui = Ui::new(label, AppState::default(), uart);
            ui.usb_want = Some(serial);
            ui.usb_path = path;
            ui.usb_serial = ser;
            ui.usb_link = link;
            return run_tui(ui, live);
        }
    };

    let ui = Ui::new(label, state, uart);
    run_tui(ui, live)
}

fn run_tui(mut ui: Ui, live: Option<CompositeHandle>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut ui, live);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

pub(crate) const HEADLESS_WATCH_HINT: &str = "\
watch requires an interactive terminal (pty)
for headless use:  demo --jsonl  |  decode FILE --jsonl  |  snapshot";

pub(crate) fn require_interactive_tty() -> Result<()> {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        return Ok(());
    }
    bail!("{}", HEADLESS_WATCH_HINT);
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ui: &mut Ui,
    mut live: Option<CompositeHandle>,
) -> Result<()> {
    let mut buf = [0u8; 8];
    let mut last_probe = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .unwrap_or_else(Instant::now);
    loop {
        terminal.draw(|f| draw(f, ui))?;

        if live.is_none() {
            if let Some(want) = ui.usb_want.clone() {
                if last_probe.elapsed() >= Duration::from_millis(400) {
                    last_probe = Instant::now();
                    match crate::usb::try_open_composite(&want) {
                        Ok(Some(h)) => {
                            ui.usb_path = h.path.clone();
                            ui.usb_serial = h.serial.clone();
                            ui.source_label = format!("live:{}", h.serial);
                            ui.usb_link = UsbLink::Connected;
                            ui.status.clear();
                            live = Some(h);
                        }
                        Ok(None) => {
                            ui.usb_link = UsbLink::Awaiting;
                            if ui.usb_path.is_empty() {
                                ui.usb_path = "—".into();
                            }
                        }
                        Err(e) => {
                            ui.usb_link = UsbLink::Awaiting;
                            ui.status = format!("USB {e}");
                        }
                    }
                }
            }
        }

        if let Some(h) = live.as_mut() {
            match h
                .handle
                .read_interrupt(EP2_IN, &mut buf, Duration::from_millis(40))
            {
                Ok(n) if n >= 6 => {
                    if let Some(frame) = DiagFrame::from_report(&buf[..n]) {
                        if frame.ty != 0 {
                            let ns = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos() as u64;
                            ui.state.push_frame(ns, frame);
                            ui.on_new_events();
                        }
                    }
                }
                Ok(_) | Err(rusb::Error::Timeout) => {}
                Err(e) => {
                    ui.status = format!("USB {e}");
                    ui.usb_link = UsbLink::Dropped;
                    live = None;
                }
            }
        } else if ui.uart_path.is_some() {
            ui.refresh_timeline();
        }

        let wait = if live.is_some() || ui.usb_want.is_some() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(200)
        };
        if event::poll(wait)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if ui.confirm_clear {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                            ui.clear_screen();
                            ui.confirm_clear = false;
                        }
                        KeyCode::Char('n')
                        | KeyCode::Char('N')
                        | KeyCode::Esc
                        | KeyCode::Char('q') => {
                            ui.confirm_clear = false;
                        }
                        _ => {}
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        ui.confirm_clear = true;
                    }
                    KeyCode::Char('x') => {
                        ui.confirm_clear = true;
                    }
                    KeyCode::Char('f') => {
                        ui.faults_only = !ui.faults_only;
                        ui.on_new_events();
                    }
                    KeyCode::Char('w') => {
                        ui.wire = !ui.wire;
                        ui.on_new_events();
                    }
                    KeyCode::Char('c') => {
                        ui.show_caps = !ui.show_caps;
                    }
                    KeyCode::Char('j') | KeyCode::Down => ui.scroll_rel(1),
                    KeyCode::Char('k') | KeyCode::Up => ui.scroll_rel(-1),
                    KeyCode::PageDown => ui.scroll_rel(10),
                    KeyCode::PageUp => ui.scroll_rel(-10),
                    KeyCode::Char('g') => ui.jump_top(),
                    KeyCode::Char('G') => ui.jump_bot(),
                    KeyCode::Char(' ') => {
                        ui.follow = !ui.follow;
                        if ui.follow {
                            ui.jump_bot();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn draw(f: &mut Frame, ui: &Ui) {
    let w = f.area().width;
    let h = f.area().height;
    let wide = w >= 110;
    let show_inst = h >= 22 && w >= 80;
    // 16:9: FLASH is a full-height rail (instruments + log). Tall column,
    // one page per row — no wrapping tiles.
    // 80×24: rail does not fit; FLASH stays a strip under the bus.
    let flash_right = wide && show_inst;
    let flash_below = show_inst && !flash_right && h >= 24;
    let show_verdict = h >= 22 && w >= 60;
    // Full-width host Verdict (evidence viewer). Not firmware. Grows with analyzers.
    let verdict_h = if !show_verdict {
        0
    } else if h >= 36 {
        9
    } else if h >= 28 {
        6
    } else {
        4
    };
    let inst_h = if !show_inst {
        0
    } else if flash_below {
        if h < 28 {
            10
        } else {
            11
        }
    } else {
        7
    };

    // Faceplate flush on top. Final band: gap · VERDICT · gap · keys.
    let chunks = if verdict_h > 0 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(5),
                Constraint::Length(1),
                Constraint::Length(verdict_h),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(5),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(f.area())
    };

    let top = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(chunks[0]);

    draw_header(f, top[0], ui);
    draw_banner(f, top[1], ui);
    // top[2] empty: diagnosis lamp must not sit on the phase keys
    draw_phases(f, top[3], ui);
    let body = chunks[1];
    if verdict_h > 0 {
        draw_verdict(f, chunks[3], ui);
        draw_footer(f, chunks[5], ui);
    } else {
        draw_footer(f, chunks[3], ui);
    }

    if flash_right {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(72), Constraint::Length(32)])
            .spacing(1)
            .split(body);
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(inst_h), Constraint::Min(5)])
            .spacing(1)
            .split(split[0]);
        draw_instruments(f, left[0], ui);
        draw_body_log(f, left[1], ui);
        draw_flash_map(f, split[1], ui);
    } else {
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(inst_h), Constraint::Min(5)])
            .spacing(1)
            .split(body);
        if inst_h > 0 {
            draw_instruments_with_optional_flash(f, left[0], ui, flash_below);
        }
        draw_body_log(f, left[1], ui);
    }
    if ui.confirm_clear {
        draw_clear_confirm(f);
    }
}

fn draw_body_log(f: &mut Frame, area: Rect, ui: &Ui) {
    if ui.show_caps {
        draw_caps(f, area, ui);
    } else {
        draw_timeline(f, area, ui);
    }
}

fn sep() -> Span<'static> {
    Span::styled(" │ ", Style::default().fg(Color::DarkGray))
}

/// 80s dash key: bezel always; lamp only when pressed. Missing control → omit.
fn dash_key(label: &str, lamp: Option<(Color, Color)>) -> Vec<Span<'static>> {
    let bezel = Style::default().fg(Color::Gray);
    let face = match lamp {
        None => Style::default().fg(Color::Gray),
        Some((fg, bg)) => Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    };
    vec![
        Span::styled("[", bezel),
        Span::styled(format!(" {label} "), face),
        Span::styled("]", bezel),
    ]
}

fn lamp_ok() -> (Color, Color) {
    (Color::Black, Color::Green)
}
fn lamp_ng() -> (Color, Color) {
    (Color::White, Color::Red)
}
fn lamp_run() -> (Color, Color) {
    (Color::Black, Color::Yellow)
}

/// Host-alive lamp: cyan, not the yellow bus keys.
fn dash_key_alive(on: bool) -> Vec<Span<'static>> {
    let bezel = Style::default().fg(Color::Cyan);
    let face = if on {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    vec![
        Span::styled("[", bezel),
        Span::styled(" RUN ", face),
        Span::styled("]", bezel),
    ]
}

fn draw_header(f: &mut Frame, area: Rect, ui: &Ui) {
    let (hud, hud_w) = usb_hud(ui);
    let hud_w = hud_w.min(area.width.saturating_sub(8)).max(12);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(8), Constraint::Length(hud_w)])
        .split(area);

    let mut spans = vec![
        Span::styled(
            crate::version::banner_short(),
            Style::default().fg(Color::White),
        ),
        sep(),
        Span::raw(ui.source_label.clone()),
        sep(),
    ];
    spans.extend(dash_key_alive(ui.follow && heartbeat()));
    spans.push(Span::raw(" "));
    spans.extend(dash_key("HOLD", (!ui.follow).then(lamp_run)));
    if ui.uart_path.is_some() {
        spans.push(Span::raw(" "));
        spans.extend(dash_key("DUAL", Some(lamp_run())));
    }
    spans.push(Span::raw(" "));
    spans.extend(dash_key("WIRE", ui.wire.then(|| (Color::Black, Color::White))));
    spans.push(Span::raw(" "));
    spans.extend(dash_key("FAULT", ui.faults_only.then(lamp_ng)));
    spans.push(Span::raw(" "));
    spans.extend(dash_key("CAPS", ui.show_caps.then(lamp_run)));
    f.render_widget(Paragraph::new(Line::from(spans)), cols[0]);
    f.render_widget(Paragraph::new(hud).alignment(Alignment::Right), cols[1]);
}

fn spans_width(spans: &[Span]) -> u16 {
    spans
        .iter()
        .map(|s| s.content.chars().count() as u16)
        .sum()
}

fn usb_hud(ui: &Ui) -> (Line<'static>, u16) {
    match ui.usb_link {
        UsbLink::Offline => {
            let spans = dash_key("OFFLINE", None);
            let w = spans_width(&spans);
            (Line::from(spans), w)
        }
        UsbLink::Awaiting => {
            let lamp = if heartbeat() {
                Some(lamp_run())
            } else {
                None
            };
            let id = if ui.usb_serial.is_empty() {
                "USBasp2".to_string()
            } else {
                ui.usb_serial.clone()
            };
            let path = if ui.usb_path.is_empty() {
                "—"
            } else {
                ui.usb_path.as_str()
            };
            let mut spans = dash_key("AWAITING", lamp);
            spans.push(Span::raw(" "));
            spans.extend(dash_key(path, None));
            spans.push(Span::raw(" "));
            spans.extend(dash_key(&id, None));
            let w = spans_width(&spans);
            (Line::from(spans), w)
        }
        UsbLink::Connected => {
            let mut spans = dash_key("CONNECTED", Some(lamp_ok()));
            spans.push(Span::raw(" "));
            // Bus address is a readout plate, not a third status lamp.
            spans.extend(dash_key(
                &ui.usb_path,
                Some((Color::Black, Color::White)),
            ));
            spans.push(Span::raw(" "));
            spans.extend(dash_key(&ui.usb_serial, None));
            let w = spans_width(&spans);
            (Line::from(spans), w)
        }
        UsbLink::Dropped => {
            let mut spans = dash_key("DROPPED", Some(lamp_ng()));
            spans.push(Span::raw(" "));
            spans.extend(dash_key(&ui.usb_path, None));
            spans.push(Span::raw(" "));
            spans.extend(dash_key(&ui.usb_serial, None));
            let w = spans_width(&spans);
            (Line::from(spans), w)
        }
    }
}

fn draw_footer(f: &mut Frame, area: Rect, ui: &Ui) {
    let n = ui.rows().len();
    let usb = if ui.status.starts_with("USB ") {
        format!("  {}", ui.status)
    } else {
        String::new()
    };
    let line = if ui.confirm_clear {
        "CLEAR screen?   y yes    n / Esc cancel".into()
    } else {
        format!(
            "  n={n}{usb}    q quit    x clear    w wire    f fault    c caps    j/k    g/G    Space HOLD"
        )
    };
    let style = if ui.confirm_clear {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if ui.status.starts_with("USB ") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(Paragraph::new(line).style(style), area);
}

/// Host Verdict rail — session face, not “any red finding = FAIL”.
fn draw_verdict(f: &mut Frame, area: Rect, ui: &Ui) {
    let face = session_face(&ui.state);
    let inner_w = area.width.saturating_sub(4);
    let inner_h = area.height.saturating_sub(2);
    let lit = !matches!(face, SessionFace::Open);
    f.render_widget(
        Paragraph::new(verdict_lines(&ui.state, inner_w, inner_h)).block(
            dim_block("VERDICT", lit).padding(Padding::horizontal(1)),
        ),
        area,
    );
}

/// Watch face only. Engine correlator is not this module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionFace {
    Pass,
    PassAnomaly,
    FailUnconfirmed,
    Inconclusive,
    Open,
}

fn session_face(st: &AppState) -> SessionFace {
    let line_fail = st.line_ok == Some(false);
    let poll_fail = st.flash_poll_failed || st.memop_pages.iter().any(|(_, ok)| !*ok);
    let stalled = st.memop_stalled;
    let ep_fail = st.ep_fail == Some(true);
    let ep_pass = st.ep_fail == Some(false);
    let memop_open = st.memop_kind.is_some() && st.memop_end_ok.is_none();
    let memop_fail = st.memop_end_ok == Some(false) || st.last_flash_ok == Some(false);
    let done = st.saw_session_end;

    if ep_fail || poll_fail || stalled || memop_fail {
        SessionFace::FailUnconfirmed
    } else if memop_open {
        // CONT pages without MEMOP END: still writing, or aborted mid-write.
        if done {
            SessionFace::FailUnconfirmed
        } else {
            SessionFace::Open
        }
    } else if ep_pass && done && line_fail {
        SessionFace::PassAnomaly
    } else if ep_pass && done {
        SessionFace::Pass
    } else if ep_pass {
        // ENABLEPROG ok, session still open — not a final PASS.
        SessionFace::Open
    } else if line_fail {
        SessionFace::Inconclusive
    } else {
        SessionFace::Open
    }
}

fn face_lamp(face: SessionFace) -> (&'static str, Style) {
    match face {
        SessionFace::FailUnconfirmed => (
            " FAIL UNCONFIRMED ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        SessionFace::Inconclusive => (
            " INCONCLUSIVE ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        SessionFace::PassAnomaly => (
            " PASS WITH ANOMALY ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        SessionFace::Pass => (
            " PASS ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        SessionFace::Open => (" OPEN ", Style::default().fg(Color::Gray)),
    }
}

fn rst_name(st: &AppState) -> &'static str {
    match st.line_bit.unwrap_or(0) {
        2 => "RST",
        3 => "MOSI",
        5 => "SCK",
        _ => "LINE",
    }
}

fn clip(s: &str, w: u16) -> String {
    let n = w as usize;
    if n == 0 {
        return String::new();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= n {
            break;
        }
        out.push(ch);
    }
    out
}

fn kv_line(key: &'static str, val: String, val_style: Style, inner_w: u16) -> Line<'static> {
    let kv = Style::default().fg(Color::DarkGray);
    Line::from(vec![
        Span::styled(format!("{key:<10}"), kv),
        Span::styled(clip(&val, inner_w.saturating_sub(10)), val_style),
    ])
}

fn verdict_lines(st: &AppState, inner_w: u16, inner_h: u16) -> Vec<Line<'static>> {
    let face = session_face(st);
    let (word, lamp) = face_lamp(face);
    let kv = Style::default().fg(Color::DarkGray);
    let ok = Style::default().fg(Color::Green);
    let ng = Style::default().fg(Color::Red);
    let note = Style::default().fg(Color::Yellow);
    let white = Style::default().fg(Color::White);

    let echo = st.ep_rx.map(|b| b.contains(&0x53)).unwrap_or(false);
    let pages_ok = st.memop_pages.iter().filter(|(_, ok)| *ok).count();
    let pages_fail = st.memop_pages.iter().filter(|(_, ok)| !*ok).count();
    let sticky_fail = st.flash_poll_failed;
    let stalled = st.memop_stalled;
    let proto = match (
        st.ep_fail,
        echo,
        sticky_fail || pages_fail > 0,
        stalled,
        st.last_flash_ok,
        st.memop_end_ok,
        pages_ok,
        pages_fail,
    ) {
        (Some(false), _, true, _, _, _, ok_n, fail_n) => {
            let addr = st
                .flash_poll_fail_addr
                .map(|a| format!(" @{a:#06x}"))
                .unwrap_or_default();
            format!("✓ ENABLEPROG  FLASH POLL FAIL{addr}  CONT ok={ok_n} fail={fail_n}")
        }
        (Some(false), _, _, true, _, _, _, _) => {
            let gap_ms = st
                .memop_stall_gap_ns
                .unwrap_or(crate::state::MEMOP_GAP_STALL_NS)
                / 1_000_000;
            format!(
                "✓ ENABLEPROG  FLASH STALL ≥{gap_ms}ms  END {} pages (not success)",
                st.last_flash_pages.unwrap_or(0)
            )
        }
        (Some(false), true, false, false, Some(true), _, n, _)
            if n > 0 || st.last_flash_pages.is_some() =>
        {
            format!(
                "✓ ENABLEPROG 0x53  ✓ FLASH END {} pages",
                st.last_flash_pages.unwrap_or(n as u8)
            )
        }
        (Some(false), true, false, false, Some(false), _, _, _) => {
            "✓ ENABLEPROG 0x53  FLASH END FAIL".into()
        }
        (Some(false), true, false, false, None, None, n, _) if n > 0 => {
            format!("✓ ENABLEPROG 0x53  FLASH {n} pages — no MEMOP END")
        }
        (Some(false), true, false, false, None, None, _, _) if st.memop_kind.is_some() => {
            "✓ ENABLEPROG 0x53  FLASH — no MEMOP END".into()
        }
        (Some(false), true, false, false, _, _, _, _) => "✓ ENABLEPROG 0x53".into(),
        (Some(true), _, _, _, _, _, _, _) => "ENABLEPROG FAIL  (primary)".into(),
        _ => "—".into(),
    };
    let pin = st.line_pin.unwrap_or(0);
    let phys = if st.line_ok == Some(false) {
        format!(
            "! {} echo  PINx={pin:#04x}  conf=LOW  capture=no",
            rst_name(st)
        )
    } else if st.line_ok == Some(true) {
        format!("✓ {} echo follows PORT", rst_name(st))
    } else {
        "—".into()
    };
    let cause = match face {
        SessionFace::PassAnomaly => {
            format!(
                "{} anomaly → programming failure   NOT ESTABLISHED",
                rst_name(st)
            )
        }
        SessionFace::FailUnconfirmed if stalled => {
            "MEMOP stall  host USB drop plausible  RST NOT ESTABLISHED".into()
        }
        SessionFace::FailUnconfirmed if st.memop_kind.is_some() && st.memop_end_ok.is_none() => {
            "MEMOP incomplete  host USB drop plausible  RST NOT ESTABLISHED".into()
        }
        SessionFace::FailUnconfirmed if st.ep_fail == Some(true) && st.line_ok == Some(false) => {
            format!("{} path plausible  physical {} NOT PROVEN", rst_name(st), rst_name(st))
        }
        SessionFace::Inconclusive => {
            "GPIO PINB ≠ connector  PHYSICAL_CAPTURE unavailable".into()
        }
        SessionFace::FailUnconfirmed => {
            "protocol path failed  pin edges NOT PROVEN".into()
        }
        SessionFace::Pass => "physical cause  NOT PROVEN".into(),
        SessionFace::Open => "—".into(),
    };

    let mut lines = vec![Line::from(vec![
        Span::styled("SESSION   ", kv),
        Span::styled(word, lamp),
    ])];
    if inner_h >= 2 {
        let proto_style = if st.ep_fail == Some(true) { ng } else if st.ep_fail == Some(false) { ok } else { kv };
        lines.push(kv_line("Protocol", proto, proto_style, inner_w));
    }
    if inner_h >= 3 {
        let phys_style = if st.line_ok == Some(false) { ng } else { kv };
        lines.push(kv_line("Physical", phys, phys_style, inner_w));
    }
    if inner_h >= 4 {
        lines.push(kv_line("Causality", cause, note, inner_w));
    }
    if inner_h >= 5 && st.line_ok == Some(false) {
        let drive = if st.line_drive_high == Some(true) {
            "HIGH"
        } else {
            "LOW"
        };
        lines.push(kv_line(
            "  expected",
            format!("{drive} on {}  (MCU PINB, not ISP jack)", rst_name(st)),
            white,
            inner_w,
        ));
    }
    if inner_h >= 6 {
        lines.push(kv_line(
            "  source",
            "USBASP_INTERNAL  PHYSICAL_CAPTURE unavailable".into(),
            kv,
            inner_w,
        ));
    }
    lines
}

fn draw_banner(f: &mut Frame, area: Rect, ui: &Ui) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let face = session_face(&ui.state);
    let (tone, line) = match face {
        SessionFace::PassAnomaly => (
            DiagTone::Warn,
            format!(
                "{} GPIO echo ANOMALY  PINx={:#04x}  not a connector fact — session is PASS WITH ANOMALY",
                rst_name(&ui.state),
                ui.state.line_pin.unwrap_or(0)
            ),
        ),
        SessionFace::Inconclusive => (
            DiagTone::Warn,
            format!(
                "{} GPIO echo ANOMALY  PINx={:#04x}  PHYSICAL_CAPTURE no — not session FAIL",
                rst_name(&ui.state),
                ui.state.line_pin.unwrap_or(0)
            ),
        ),
        _ => diagnosis_at(&ui.state, Some(now)),
    };
    let (fg, bg, lit) = match tone {
        DiagTone::Bad => (Color::White, Color::Red, true),
        DiagTone::Ok => (Color::Black, Color::Green, true),
        DiagTone::Warn => (Color::Black, Color::Yellow, true),
        DiagTone::Info => (Color::DarkGray, Color::Reset, false),
    };
    let style = if lit {
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(fg)
    };
    let text = format!("  {line}  ");
    let para = if lit {
        Paragraph::new(Span::styled(text, style)).style(Style::default().bg(bg))
    } else {
        Paragraph::new(Span::styled(text, style))
    };
    f.render_widget(para, area);
}

fn draw_phases(f: &mut Frame, area: Rect, ui: &Ui) {
    let mut spans = Vec::new();
    for (i, (name, mark)) in phases(&ui.state).iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        let lamp = match mark {
            PhaseMark::Idle => None,
            PhaseMark::Ok => Some(lamp_ok()),
            PhaseMark::Fail => Some(lamp_ng()),
            PhaseMark::Active => Some(lamp_run()),
        };
        spans.extend(dash_key(name, lamp));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn dim_block(title: &str, lit: bool) -> Block<'_> {
    let style = if lit {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(style)
        .title_style(style)
}

fn log_block(title: String) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .padding(Padding::horizontal(1))
}

fn draw_instruments_with_optional_flash(
    f: &mut Frame,
    area: Rect,
    ui: &Ui,
    flash_below: bool,
) {
    let bus = if flash_below && area.height >= 10 {
        let flash_h = if area.height >= 11 { 4 } else { 3 };
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(7), Constraint::Length(flash_h)])
            .split(area);
        draw_flash_map(f, split[1], ui);
        split[0]
    } else {
        area
    };
    draw_instruments(f, bus, ui);
}

fn draw_instruments(f: &mut Frame, area: Rect, ui: &Ui) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(32),
            Constraint::Length(28),
        ])
        .split(area);

    let isp_lit = ui.state.saw_reset || ui.state.pins_ddr.is_some() || ui.state.sck_id.is_some();
    f.render_widget(
        Paragraph::new(isp_pin_lines(&ui.state, blink_now())).block(dim_block("ISP", isp_lit)),
        cols[0],
    );
    let ep_lit = ui.state.ep_fail.is_some();
    f.render_widget(
        Paragraph::new(enableprog_lines(&ui.state)).block(dim_block("ENABLEPROG", ep_lit)),
        cols[1],
    );
    let trace_lit = ui.state.trace_triggered
        || ui.state.trace_overflow
        || (ui.state.saw_session && !ui.state.saw_session_end)
        || ui.state.trace_write_index.is_some();
    let trace_title = match ui.state.trace_slots {
        Some(n) => format!("TRACE LOG {n}"),
        None => "TRACE LOG".into(),
    };
    f.render_widget(
        Paragraph::new(trace_lines(&ui.state)).block(dim_block(&trace_title, trace_lit)),
        cols[2],
    );
}

fn draw_flash_map(f: &mut Frame, area: Rect, ui: &Ui) {
    let inner_w = area.width.saturating_sub(4);
    let inner_h = area.height.saturating_sub(2);
    let lit = ui.state.memop_kind.is_some() || !ui.state.memop_pages.is_empty();
    let flash_title = match ui.state.memop_kind {
        Some(MEM_READFLASH) => "FLASH READ",
        Some(MEM_FLASH) => "FLASH WRITE",
        Some(MEM_EEPROM) => "EEPROM",
        _ => "FLASH",
    };
    f.render_widget(
        Paragraph::new(flash_lines(&ui.state, inner_w, inner_h, blink_now()))
            .block(dim_block(flash_title, lit).padding(Padding::horizontal(1))),
        area,
    );
}

fn blink_now() -> bool {
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        / 350)
        % 2
        == 0
}

/// RUN heartbeat: lamp blinks so a live watch is visibly not frozen.
fn heartbeat() -> bool {
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        / 600)
        % 2
        == 0
}

fn draw_clear_confirm(f: &mut Frame) {
    let area = f.area();
    let w = 44u16.min(area.width.saturating_sub(2));
    let h = 5u16.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let pop = Rect {
        x,
        y,
        width: w,
        height: h,
    };
    f.render_widget(Clear, pop);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                " CLEAR session? ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " y yes     n / Esc cancel ",
                Style::default().fg(Color::White),
            )),
        ])
        .block(dim_block("CONFIRM", true)),
        pop,
    );
}

fn recent_ns(event: Option<u64>, now: Option<u64>, window_ns: u64) -> bool {
    match (event, now) {
        (Some(t), Some(n)) => n.saturating_sub(t) <= window_ns,
        (Some(_), None) => true,
        _ => false,
    }
}

/// Semantic lamp for EP2 frames (ENABLEPROG TX/RX, MEMOP). Not SCK edges.
fn vled(on: bool, hot: bool, silent: bool, blink: bool) -> Span<'static> {
    if !on && !silent {
        return Span::styled(" · ", Style::default().fg(Color::DarkGray));
    }
    if silent {
        return Span::styled(" · ", Style::default().fg(Color::Red));
    }
    let hot = hot && blink;
    let style = if hot {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Black).bg(Color::Green)
    };
    Span::styled(" * ", style)
}

fn pin_pair(name: &'static str, val: String, style: Style) -> (Span<'static>, Span<'static>) {
    let name_s = Span::styled(format!("{name:<10}"), Style::default().fg(Color::DarkGray));
    let val_s = Span::styled(format!("{val:<10}"), style);
    (name_s, val_s)
}

fn isp_pin_lines(st: &AppState, blink: bool) -> Vec<Line<'static>> {
    // EP2 faceplate: MCU intent + sampled frames. Not a probe.
    // RESET = drive intent. SCK = id + HW/SW, not Hz.
    // MOSI/MISO lamps = ENABLEPROG TX/RX and MEMOP direction, not edges.
    // DISC = ISP_PINS one-shot DDR/PIN after disconnect.
    let now = st.events.last().map(|e| e.host_ns);
    const WIN_NS: u64 = 400_000_000;
    let rst = if st.reset_asserted {
        (
            "DRV".into(),
            Style::default().fg(Color::Black).bg(Color::Yellow),
        )
    } else if st.saw_release {
        ("REL".into(), Style::default().fg(Color::White))
    } else {
        ("—".into(), Style::default().fg(Color::DarkGray))
    };
    let mosi: (String, Style) = if st.mosi_ns.is_some() {
        ("TX".into(), Style::default().fg(Color::White))
    } else {
        ("—".into(), Style::default().fg(Color::DarkGray))
    };
    let miso: (String, Style) = if st.miso_silent {
        ("SILENT".into(), Style::default().fg(Color::Red))
    } else if st.miso_ns.is_some() {
        ("RX".into(), Style::default().fg(Color::White))
    } else {
        ("—".into(), Style::default().fg(Color::DarkGray))
    };
    let sck = match (st.sck_id, st.sck_sw) {
        (Some(id), Some(true)) => (
            format!("SW {id}"),
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ),
        (Some(id), _) => (format!("HW {id}"), Style::default().fg(Color::White)),
        _ => ("—".into(), Style::default().fg(Color::DarkGray)),
    };
    let mosi_led = vled(
        st.mosi_ns.is_some(),
        recent_ns(st.mosi_ns, now, WIN_NS),
        false,
        blink,
    );
    let miso_led = vled(
        st.miso_ns.is_some(),
        recent_ns(st.miso_ns, now, WIN_NS),
        st.miso_silent,
        blink,
    );
    let (rst_n, rst_v) = pin_pair("RST", rst.0, rst.1);
    let (mosi_n, _) = pin_pair("MOSI", mosi.0.clone(), mosi.1);
    let (miso_n, _) = pin_pair("MISO", miso.0.clone(), miso.1);
    let (sck_n, sck_v) = pin_pair("SCK", sck.0, sck.1);
    let mosi_txt = Span::styled(format!("{:<6}", mosi.0), mosi.1);
    let miso_txt = Span::styled(format!("{:<6}", miso.0), miso.1);
    let disc = match st.pins_ok {
        Some(true) => Span::styled(
            "Hi-Z",
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Some(false) => Span::styled(
            "DRIVE",
            Style::default().fg(Color::White).bg(Color::Red),
        ),
        None => Span::styled("—", Style::default().fg(Color::DarkGray)),
    };
    vec![
        Line::from(vec![rst_n, Span::raw(" "), mosi_n]),
        Line::from(vec![rst_v, Span::raw(" "), mosi_led, mosi_txt]),
        Line::from(vec![miso_n, Span::raw(" "), sck_n]),
        Line::from(vec![miso_led, miso_txt, Span::raw(" "), sck_v]),
        Line::from(vec![
            Span::styled("SNAP ", Style::default().fg(Color::DarkGray)),
            disc,
        ]),
    ]
}

fn hex_cell(b: u8, accent: bool, fail: bool) -> Span<'static> {
    let style = if fail {
        Style::default()
            .fg(Color::White)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else if accent {
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    };
    Span::styled(format!(" {b:02X} "), style)
}

/// Same footprint as a live hex cube; gray = slot exists, wait for ISP.
fn wait_cell() -> Span<'static> {
    Span::styled(" -- ", Style::default().fg(Color::DarkGray))
}

const EP_CUBES: usize = 4;

fn cube_row(label: &'static str, bytes: Option<[u8; 4]>, fail: Option<bool>) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{label:<3} "),
        Style::default().fg(Color::DarkGray),
    )];
    for i in 0..EP_CUBES {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        match bytes {
            Some(b) => {
                let tx_cmd = label == "TX" && i == 1;
                let is_echo = label == "RX" && b[i] == 0x53;
                let rx_fail = label == "RX" && fail == Some(true) && i == 0;
                spans.push(hex_cell(
                    b[i],
                    tx_cmd || (is_echo && fail != Some(true)),
                    rx_fail,
                ));
            }
            None => spans.push(wait_cell()),
        }
    }
    Line::from(spans)
}

fn enableprog_lines(st: &AppState) -> Vec<Line<'static>> {
    let tx = cube_row("TX", st.ep_tx, st.ep_fail);
    let rx = cube_row("RX", st.ep_rx, st.ep_fail);
    let mut lines = vec![tx, rx];
    match st.ep_fail {
        Some(fail) => {
            let echo_ok = st
                .ep_rx
                .map(|b| b.iter().any(|&x| x == 0x53))
                .unwrap_or(false);
            let res = if fail {
                Span::styled(" FAIL ", Style::default().fg(Color::White).bg(Color::Red))
            } else {
                Span::styled(" PASS ", Style::default().fg(Color::Black).bg(Color::Green))
            };
            let echo = if echo_ok {
                Span::styled("  ECHO 53", Style::default().fg(Color::Black).bg(Color::Green))
            } else {
                Span::styled(
                    "  ECHO 53 MISS",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                )
            };
            let delay = st
                .snap_delay
                .or(st.err_delay)
                .map(|d| format!("  t={d}"))
                .unwrap_or_default();
            lines.push(Line::from(vec![res, echo, Span::raw(delay)]));
        }
        None => {
            lines.push(Line::from(Span::styled(
                " --    --",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
    lines
}

fn flash_lines(st: &AppState, inner_w: u16, inner_h: u16, blink: bool) -> Vec<Line<'static>> {
    if st.memop_kind.is_none() && st.memop_pages.is_empty() {
        return vec![Line::from(Span::styled(
            "—",
            Style::default().fg(Color::DarkGray),
        ))];
    }
    let psz = st.memop_pagesize.unwrap_or(0) as u16;
    let lo = st.memop_pages.first().map(|(a, _)| *a);
    let hi = st
        .memop_pages
        .last()
        .map(|(a, _)| a.saturating_add(psz.saturating_sub(1)));
    let range = match (lo, hi) {
        (Some(a), Some(b)) => format!("{a:#06x}–{b:#06x}"),
        _ => "—".into(),
    };
    let end_n = st
        .memop_end_pages
        .map(|n| n as usize)
        .unwrap_or(st.memop_pages.len());
    let mut lines = vec![Line::from(Span::styled(
        format!("{range}  CONT {}/{end_n}", st.memop_pages.len()),
        Style::default().fg(Color::DarkGray),
    ))];
    if st.memop_pages.is_empty() {
        lines.push(Line::from(Span::styled(
            "CONT —  END is authority",
            Style::default().fg(Color::DarkGray),
        )));
        return lines;
    }
    let cell_w = 12u16;
    let per_row = if inner_h >= 8 {
        1
    } else {
        (inner_w / cell_w).max(1) as usize
    };
    let n = st.memop_pages.len();
    let room = inner_h.saturating_sub(1) as usize;
    let mut cap = room.max(1).saturating_mul(per_row);
    if n > cap {
        cap = cap.saturating_sub(per_row).max(1);
    }
    let start = n.saturating_sub(cap);
    if start > 0 {
        lines.push(Line::from(Span::styled(
            "…",
            Style::default().fg(Color::DarkGray),
        )));
    }
    let slice = &st.memop_pages[start..n];
    let last_i = slice.len().saturating_sub(1);
    let mut row: Vec<Span> = Vec::new();
    for (i, &(addr, ok)) in slice.iter().enumerate() {
        let sync = i == last_i;
        let style = if !ok {
            if blink {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD)
            }
        } else if sync {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Black).bg(Color::Green)
        };
        let mark = if sync { '>' } else { ' ' };
        row.push(Span::styled(
            format!("{mark}{addr:#06x} {} ", if ok { "OK" } else { "NG" }),
            style,
        ));
        if (i + 1) % per_row == 0 {
            lines.push(Line::from(std::mem::take(&mut row)));
        } else {
            row.push(Span::raw(" "));
        }
    }
    if !row.is_empty() {
        lines.push(Line::from(row));
    }
    lines
}

fn trace_lines(st: &AppState) -> Vec<Line<'static>> {
    let life = if st.trace_triggered {
        "FROZEN"
    } else if st.saw_session && !st.saw_session_end {
        "ARMED"
    } else if st.trace_write_index.is_some() {
        "END"
    } else {
        "IDLE"
    };
    let life_style = match life {
        "FROZEN" => Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        "ARMED" => Style::default().fg(Color::Black).bg(Color::Yellow),
        "END" => Style::default().fg(Color::White),
        _ => Style::default().fg(Color::DarkGray),
    };
    let kv = Style::default().fg(Color::DarkGray);
    let slots = st.trace_slots.map(|n| n as u16);
    let occ = match (st.trace_write_index, slots, st.trace_valid) {
        (Some(w), Some(s), Some(v)) => format!("{w}/{s}  V={v}"),
        (Some(w), Some(s), None) => format!("{w}/{s}"),
        (Some(w), None, _) => format!("{w}/—"),
        _ => "—".into(),
    };
    let ov = if st.trace_overflow {
        Span::styled(
            "YES",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )
    } else if st.trace_write_index.is_some() || st.saw_session {
        Span::styled("no", Style::default().fg(Color::White))
    } else {
        Span::styled("—", kv)
    };
    let kind = match st.trace_kind {
        Some(3) => "ENABLEPROG_FAIL",
        Some(4) => "TRACE_OVERFLOW",
        Some(0) => "NONE",
        Some(_) => "?",
        None => "—",
    };
    vec![
        Line::from(vec![
            Span::styled("ST   ", kv),
            Span::styled(format!(" {life} "), life_style),
        ]),
        Line::from(vec![
            Span::styled("OCC  ", kv),
            Span::styled(occ, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![Span::styled("OV   ", kv), ov]),
        Line::from(Span::styled(
            format!("KIND {kind}  POST {}  LOG", st.trace_post.unwrap_or(0)),
            kv,
        )),
    ]
}

fn draw_caps(f: &mut Frame, area: Rect, ui: &Ui) {
    let mut text = match &ui.state.caps {
        Some(adv) => adv.format_report(&crate::version::banner_short()),
        None => "CAPS: wait CONNECT (not USB plug-in)".into(),
    };
    if let (Some(ddr), Some(pin)) = (ui.state.pins_ddr, ui.state.pins_pin) {
        let hz = match ui.state.pins_ok {
            Some(true) => "Hi-Z",
            Some(false) => "DRIVE",
            None => "?",
        };
        text.push_str(&format!(
            "\n\nISP_PINS  {hz}  DDR=0x{ddr:02X}  PIN=0x{pin:02X}\n"
        ));
    }
    f.render_widget(
        Paragraph::new(text)
            .block(dim_block("CAPS", ui.state.caps.is_some()).padding(Padding::horizontal(1))),
        area,
    );
}

fn draw_timeline(f: &mut Frame, area: Rect, ui: &Ui) {
    let rows_data = ui.rows();
    let dual = ui.uart_path.is_some();
    let n = rows_data.len();

    let mut state = ui.table_state.clone();
    if let Some(sel) = state.selected() {
        if n == 0 {
            state.select(None);
        } else if sel >= n {
            state.select(Some(n - 1));
        }
    }

    let gutter = "│";
    let header = if dual {
        Row::new(["Δt", "PROGRAMMER", gutter, "TARGET"])
            .style(Style::default().add_modifier(Modifier::BOLD))
    } else {
        Row::new(["Δt", "EVENT"]).style(Style::default().add_modifier(Modifier::BOLD))
    };

    let table_rows: Vec<Row> = rows_data
        .iter()
        .map(|r| {
            let style = if r.is_anchor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if r.is_fault {
                Style::default().fg(Color::Red)
            } else {
                match r.level {
                    Level::Error => Style::default().fg(Color::Red),
                    Level::Warn => Style::default().fg(Color::Yellow),
                    Level::Info => Style::default(),
                    Level::Debug => Style::default().fg(Color::DarkGray),
                }
            };
            let dt = rel_label(r.rel_ms);
            if dual {
                Row::new(vec![
                    Cell::from(dt),
                    Cell::from(r.prog.clone()),
                    Cell::from(Span::styled(
                        gutter,
                        Style::default().fg(Color::DarkGray),
                    )),
                    Cell::from(r.target.clone()),
                ])
                .style(style)
            } else {
                Row::new(vec![Cell::from(dt), Cell::from(r.prog.clone())]).style(style)
            }
        })
        .collect();

    let widths = if dual {
        vec![
            Constraint::Length(9),
            Constraint::Percentage(46),
            Constraint::Length(1),
            Constraint::Percentage(46),
        ]
    } else {
        vec![Constraint::Length(9), Constraint::Min(20)]
    };
    let title = if dual {
        format!(" P  |  T    n={n}    ORDER RELEASE↔READY ")
    } else {
        format!(" LOG   n={n} ")
    };
    let table = Table::new(table_rows, widths)
        .header(header)
        .column_spacing(2)
        .block(log_block(title))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    f.render_stateful_widget(table, area, &mut state);
}

#[cfg(test)]
mod tests {
    use super::{
        isp_pin_lines, session_face, usb_hud, verdict_lines, SessionFace, Ui, UsbLink,
        HEADLESS_WATCH_HINT,
    };
    use crate::state::AppState;

    fn line_text(line: &ratatui::text::Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn headless_hint_points_at_jsonl() {
        assert!(HEADLESS_WATCH_HINT.contains("demo --jsonl"));
        assert!(HEADLESS_WATCH_HINT.contains("decode FILE --jsonl"));
        assert!(HEADLESS_WATCH_HINT.contains("interactive terminal"));
    }

    #[test]
    fn snap_omits_ddr_pin_hex() {
        let mut st = AppState::default();
        st.pins_ok = Some(true);
        st.pins_ddr = Some(0x12);
        st.pins_pin = Some(0x34);
        let text = line_text(isp_pin_lines(&st, false).last().unwrap());
        assert!(text.contains("Hi-Z"), "{text}");
        assert!(!text.contains("d="), "{text}");
        assert!(!text.contains("p="), "{text}");
    }

    #[test]
    fn verdict_idle_is_open_not_fail() {
        let st = AppState::default();
        let lines = verdict_lines(&st, 80, 4);
        let all = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(all.contains("OPEN"), "{all}");
        assert!(!all.contains(" FAIL "), "{all}");
    }

    #[test]
    fn line_fault_with_enableprog_pass_is_anomaly_not_session_fail() {
        let mut st = AppState::default();
        st.line_ok = Some(false);
        st.line_bit = Some(2);
        st.line_drive_high = Some(true);
        st.line_pin = Some(0x14);
        st.ep_fail = Some(false);
        st.ep_rx = Some([0xff, 0xff, 0x53, 0x00]);
        st.memop_pages = vec![(0, true), (0x200, true), (0x400, true)];
        st.memop_kind = Some(1); // MEM_FLASH
        st.memop_end_ok = Some(true);
        st.memop_end_pages = Some(3);
        st.saw_session_end = true;
        assert_eq!(session_face(&st), SessionFace::PassAnomaly);
        let all = verdict_lines(&st, 96, 4)
            .iter()
            .map(line_text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all.contains("PASS WITH ANOMALY"), "{all}");
        assert!(all.contains("ENABLEPROG"), "{all}");
        assert!(all.contains("RST"), "{all}");
        assert!(all.contains("NOT ESTABLISHED"), "{all}");
        assert!(!all.contains(" FAIL "), "{all}");
    }

    #[test]
    fn line_fault_without_enableprog_is_inconclusive() {
        let mut st = AppState::default();
        st.line_ok = Some(false);
        st.line_bit = Some(2);
        st.line_pin = Some(0x14);
        assert_eq!(session_face(&st), SessionFace::Inconclusive);
        let all = verdict_lines(&st, 80, 4)
            .iter()
            .map(line_text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all.contains("INCONCLUSIVE"), "{all}");
        assert!(!all.contains(" FAIL "), "{all}");
    }

    #[test]
    fn flash_cont_without_end_is_not_pass() {
        let mut st = AppState::default();
        st.line_ok = Some(false);
        st.line_bit = Some(2);
        st.line_pin = Some(0x14);
        st.ep_fail = Some(false);
        st.ep_rx = Some([0xff, 0xff, 0x53, 0x00]);
        st.memop_kind = Some(1);
        st.memop_pages = (0..23).map(|i| ((i * 0x40) as u16, true)).collect();
        st.memop_end_ok = None;
        st.saw_session_end = true;
        assert_eq!(session_face(&st), SessionFace::FailUnconfirmed);
        let all = verdict_lines(&st, 96, 4)
            .iter()
            .map(line_text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all.contains("FAIL UNCONFIRMED"), "{all}");
        assert!(all.contains("no MEMOP END"), "{all}");
        assert!(!all.contains("PASS WITH ANOMALY"), "{all}");
    }

    #[test]
    fn flash_poll_fail_survives_readflash_and_is_not_pass() {
        let mut st = AppState::default();
        st.line_ok = Some(false);
        st.line_bit = Some(2);
        st.line_pin = Some(0x14);
        st.ep_fail = Some(false);
        st.ep_rx = Some([0xff, 0xff, 0x53, 0x00]);
        st.flash_poll_failed = true;
        st.flash_poll_fail_addr = Some(0x11c0);
        st.last_flash_ok = Some(false);
        st.last_flash_pages = Some(128);
        // After READFLASH START, current memop_pages may be all OK.
        st.memop_kind = Some(2); // READFLASH
        st.memop_pages = vec![(0, true)];
        st.memop_end_ok = Some(true);
        st.saw_session_end = true;
        assert_eq!(session_face(&st), SessionFace::FailUnconfirmed);
        let all = verdict_lines(&st, 96, 4)
            .iter()
            .map(line_text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all.contains("FAIL UNCONFIRMED"), "{all}");
        assert!(all.contains("POLL FAIL"), "{all}");
        assert!(!all.contains("PASS WITH ANOMALY"), "{all}");
    }

    #[test]
    fn flash_stall_end_ok_is_not_pass_anomaly() {
        let mut st = AppState::default();
        st.line_ok = Some(false);
        st.line_bit = Some(2);
        st.line_pin = Some(0x14);
        st.ep_fail = Some(false);
        st.ep_rx = Some([0xff, 0xff, 0x53, 0x00]);
        st.memop_stalled = true;
        st.memop_stall_gap_ns = Some(17_000_000_000);
        st.last_flash_ok = Some(false);
        st.last_flash_pages = Some(68);
        st.memop_end_ok = Some(true);
        st.saw_session_end = true;
        assert_eq!(session_face(&st), SessionFace::FailUnconfirmed);
        let all = verdict_lines(&st, 96, 4)
            .iter()
            .map(line_text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all.contains("FAIL UNCONFIRMED"), "{all}");
        assert!(all.contains("STALL"), "{all}");
        assert!(!all.contains("PASS WITH ANOMALY"), "{all}");
    }

    #[test]
    fn usb_hud_awaiting_names_the_stick() {
        let mut ui = Ui::new("live:YEL0".into(), AppState::default(), None);
        ui.usb_link = UsbLink::Awaiting;
        ui.usb_serial = "YEL0".into();
        ui.usb_path = "—".into();
        let (line, _) = usb_hud(&ui);
        let t = line_text(&line);
        assert!(t.contains("AWAITING"), "{t}");
        assert!(t.contains("YEL0"), "{t}");
        assert!(t.contains("["), "{t}");
    }

    #[test]
    fn usb_hud_connected_path_is_a_key() {
        let mut ui = Ui::new("live:YEL0".into(), AppState::default(), None);
        ui.usb_link = UsbLink::Connected;
        ui.usb_serial = "YEL0".into();
        ui.usb_path = "/dev/bus/usb/003/010".into();
        let t = line_text(&usb_hud(&ui).0);
        assert!(t.contains("CONNECTED"), "{t}");
        assert!(t.contains("[ /dev/bus/usb/003/010 ]"), "{t}");
        assert!(t.contains("[ YEL0 ]"), "{t}");
    }
}

