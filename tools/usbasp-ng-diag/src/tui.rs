//! Ratatui watch UI: diagnosis + phases + instruments + dual-column timeline.

use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use ratatui::Terminal;
use std::io::{self, IsTerminal, Stdout};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
            let handle = crate::usb::open_composite(&serial)?;
            (
                format!("live:{}", handle.serial),
                AppState::default(),
                Some(handle),
            )
        }
    };

    let mut ui = Ui::new(label, state, uart);
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
    loop {
        terminal.draw(|f| draw(f, ui))?;

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
                }
            }
        } else if ui.uart_path.is_some() {
            ui.refresh_timeline();
        }

        let wait = if live.is_some() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(200)
        };
        if event::poll(wait)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
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
    let inst_h = if !show_inst {
        0
    } else if flash_below {
        10
    } else {
        7
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_header(f, chunks[0], ui);
    draw_banner(f, chunks[1], ui);
    draw_phases(f, chunks[2], ui);
    let body = chunks[3];
    draw_footer(f, chunks[4], ui);

    if flash_right {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(72), Constraint::Length(32)])
            .split(body);
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(inst_h), Constraint::Min(5)])
            .split(split[0]);
        draw_instruments(f, left[0], ui);
        draw_body_log(f, left[1], ui);
        draw_flash_map(f, split[1], ui);
    } else {
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(inst_h), Constraint::Min(5)])
            .split(body);
        if inst_h > 0 {
            draw_instruments_with_optional_flash(f, left[0], ui, flash_below);
        }
        draw_body_log(f, left[1], ui);
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
    let bezel = Style::default().fg(Color::DarkGray);
    let face = match lamp {
        None => Style::default().fg(Color::DarkGray),
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

fn draw_header(f: &mut Frame, area: Rect, ui: &Ui) {
    let sck = match (ui.state.sck_id, ui.state.sck_sw) {
        (Some(id), Some(true)) => format!("SCK SW {id}"),
        (Some(id), _) => format!("SCK HW {id}"),
        _ => "SCK —".into(),
    };
    let mut spans = vec![
        Span::styled(
            crate::version::banner_short(),
            Style::default().fg(Color::White),
        ),
        sep(),
        Span::raw(ui.source_label.clone()),
        sep(),
        Span::styled(sck, Style::default().fg(Color::DarkGray)),
        sep(),
    ];
    spans.extend(dash_key("RUN", ui.follow.then(lamp_run)));
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
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_footer(f: &mut Frame, area: Rect, ui: &Ui) {
    let n = ui.rows().len();
    let usb = if ui.status.starts_with("USB ") {
        format!("  {}", ui.status)
    } else {
        String::new()
    };
    let line = format!("n={n}{usb}    q  w wire  f fault  c caps  j/k  g/G  Space HOLD");
    let style = if ui.status.starts_with("USB ") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(Paragraph::new(line).style(style), area);
}

fn draw_banner(f: &mut Frame, area: Rect, ui: &Ui) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let (tone, line) = diagnosis_at(&ui.state, Some(now));
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
    f.render_widget(
        Paragraph::new(Span::styled(format!(" {line} "), style)),
        area,
    );
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

fn draw_instruments_with_optional_flash(
    f: &mut Frame,
    area: Rect,
    ui: &Ui,
    flash_below: bool,
) {
    let bus = if flash_below && area.height >= 10 {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(7), Constraint::Length(3)])
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
        Some(n) => format!("TRACE {n}"),
        None => "TRACE".into(),
    };
    f.render_widget(
        Paragraph::new(trace_lines(&ui.state)).block(dim_block(&trace_title, trace_lit)),
        cols[2],
    );
}

fn draw_flash_map(f: &mut Frame, area: Rect, ui: &Ui) {
    let inner_w = area.width.saturating_sub(2);
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
            .block(dim_block(flash_title, lit)),
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

fn recent_ns(event: Option<u64>, now: Option<u64>, window_ns: u64) -> bool {
    match (event, now) {
        (Some(t), Some(n)) => n.saturating_sub(t) <= window_ns,
        (Some(_), None) => true,
        _ => false,
    }
}

/// Semantic bus LED — EP2 has frames, not an SCK/MOSI/MISO oscillogram.
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

fn pin_after_disc(st: &AppState, bit: u8) -> Option<(&'static str, Style)> {
    // bit: 2 RST, 3 MOSI, 4 MISO, 5 SCK
    let ddr = st.pins_ddr?;
    let driving = ddr & (1 << bit) != 0;
    Some(if driving {
        ("DRIVE", Style::default().fg(Color::White).bg(Color::Red))
    } else {
        ("Hi-Z", Style::default().fg(Color::Black).bg(Color::Green))
    })
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
    let mosi = if st.mosi_ns.is_some() {
        ("TX".into(), Style::default().fg(Color::White))
    } else {
        ("—".into(), Style::default().fg(Color::DarkGray))
    };
    let miso = if st.miso_silent {
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
    let disc = match (st.pins_ok, st.pins_ddr, st.pins_pin) {
        (Some(true), ddr, pin) => Span::styled(
            format!(
                "Hi-Z  d={:02X} p={:02X}",
                ddr.unwrap_or(0),
                pin.unwrap_or(0)
            ),
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        (Some(false), ddr, pin) => Span::styled(
            format!(
                "DRIVE d={:02X} p={:02X}",
                ddr.unwrap_or(0),
                pin.unwrap_or(0)
            ),
            Style::default().fg(Color::White).bg(Color::Red),
        ),
        (None, _, _) => Span::styled("—", Style::default().fg(Color::DarkGray)),
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
            format!("KIND {kind}  POST {}", st.trace_post.unwrap_or(0)),
            kv,
        )),
    ]
}

fn draw_caps(f: &mut Frame, area: Rect, ui: &Ui) {
    let text = match &ui.state.caps {
        Some(adv) => adv.format_report(&crate::version::banner_short()),
        None => "CAPS: wait CONNECT (not USB plug-in)".into(),
    };
    f.render_widget(
        Paragraph::new(text).block(dim_block("CAPS", ui.state.caps.is_some())),
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

    let header = if dual {
        Row::new(["Δt", "PROGRAMMER", "TARGET"]).style(Style::default().add_modifier(Modifier::BOLD))
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
            Constraint::Percentage(48),
            Constraint::Percentage(48),
        ]
    } else {
        vec![Constraint::Length(9), Constraint::Min(20)]
    };
    let title = if dual {
        format!(" P | T   n={n}   ORDER RELEASE↔READY ")
    } else {
        format!(" LOG   n={n} ")
    };
    let table = Table::new(table_rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    f.render_stateful_widget(table, area, &mut state);
}

#[cfg(test)]
mod tests {
    use super::HEADLESS_WATCH_HINT;

    #[test]
    fn headless_hint_points_at_jsonl() {
        assert!(HEADLESS_WATCH_HINT.contains("demo --jsonl"));
        assert!(HEADLESS_WATCH_HINT.contains("decode FILE --jsonl"));
        assert!(HEADLESS_WATCH_HINT.contains("interactive terminal"));
    }
}

