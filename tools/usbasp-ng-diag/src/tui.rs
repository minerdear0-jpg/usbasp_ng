//! Ratatui watch UI: diagnosis + phases + instruments + dual-column timeline.

use anyhow::Result;
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
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::capture::CaptureFile;
use crate::correlate::{self, TimelineEvent};
use crate::demo;
use crate::decoder::type_name;
use crate::protocol::{DiagFrame, EP2_IN};
use crate::scene::{
    diagnosis, dual_rows, is_wire_fragment, phases, programmer_rows, rel_label, DiagTone,
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
            status: "q quit  w wire  f faults  c caps  j/k  g/G  Space follow".into(),
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
                    ui.status = format!("USB error: {e} (q to quit)");
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
    // Mockup: instruments stay visible on 80×24, not only 100×24.
    // Bus faceplate stays put. FLASH rails on the right on 16:9 (~110+ cols).
    let inst_h = if h >= 22 && w >= 80 { 7 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(inst_h),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_header(f, chunks[0], ui);
    draw_banner(f, chunks[1], ui);
    draw_phases(f, chunks[2], ui);
    if inst_h > 0 {
        draw_instruments(f, chunks[3], ui);
    }
    if ui.show_caps {
        draw_caps(f, chunks[4], ui);
    } else {
        draw_timeline(f, chunks[4], ui);
    }
    let help = Paragraph::new(ui.status.as_str()).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[5]);
}

fn draw_header(f: &mut Frame, area: Rect, ui: &Ui) {
    let sck = match (ui.state.sck_id, ui.state.sck_sw) {
        (Some(id), Some(true)) => format!("SW SCK id={id}"),
        (Some(id), _) => format!("HW SCK id={id}"),
        _ => "SCK —".into(),
    };
    let mut spans = vec![
        Span::styled(
            format!(" {} ", crate::version::banner_short()),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  {}  {sck}", ui.source_label)),
    ];
    if ui.uart_path.is_some() {
        spans.push(Span::styled(
            "  DUAL",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }
    if ui.follow {
        spans.push(Span::styled("  follow", Style::default().fg(Color::DarkGray)));
    } else {
        spans.push(Span::styled("  paused", Style::default().fg(Color::Yellow)));
    }
    if ui.wire {
        spans.push(Span::styled("  WIRE", Style::default().fg(Color::Magenta)));
    }
    if ui.faults_only {
        spans.push(Span::styled("  FAULTS", Style::default().fg(Color::Red)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_banner(f: &mut Frame, area: Rect, ui: &Ui) {
    let (tone, line) = diagnosis(&ui.state);
    let (fg, bg) = match tone {
        DiagTone::Bad => (Color::White, Color::Red),
        DiagTone::Ok => (Color::Black, Color::Green),
        DiagTone::Warn => (Color::Black, Color::Yellow),
        DiagTone::Info => (Color::Cyan, Color::Reset),
    };
    let text = line;
    f.render_widget(
        Paragraph::new(Span::styled(
            format!(" {text} "),
            Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
        )),
        area,
    );
}

fn draw_phases(f: &mut Frame, area: Rect, ui: &Ui) {
    let mut spans = Vec::new();
    for (i, (name, mark)) in phases(&ui.state).iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        let (sym, fg, bg) = match mark {
            PhaseMark::Ok => ("✓", Color::Black, Color::Green),
            PhaseMark::Fail => ("×", Color::White, Color::Red),
            PhaseMark::Active => ("▶", Color::Black, Color::Yellow),
            PhaseMark::Idle => ("·", Color::DarkGray, Color::Reset),
        };
        spans.push(Span::styled(
            format!(" {name} {sym} "),
            Style::default().fg(fg).bg(bg),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_instruments(f: &mut Frame, area: Rect, ui: &Ui) {
    let show_flash = ui.state.memop_kind.is_some() && ui.state.ep_fail != Some(true);
    // 16:9 terminal ≈ 3.3 cells/row (cell ~1:2). 120×36 is already that.
    let bus = if show_flash && area.width >= 110 {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(72), Constraint::Length(32)])
            .split(area);
        draw_flash_map(f, split[1], ui);
        split[0]
    } else {
        area
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(32),
            Constraint::Length(28),
        ])
        .split(bus);

    f.render_widget(
        Paragraph::new(isp_pin_lines(&ui.state)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("ISP"),
        ),
        cols[0],
    );
    f.render_widget(
        Paragraph::new(enableprog_lines(&ui.state)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("ENABLEPROG"),
        ),
        cols[1],
    );
    let trace_title = match ui.state.trace_slots {
        Some(n) => format!("TRACE {n}"),
        None => "TRACE".into(),
    };
    f.render_widget(
        Paragraph::new(trace_lines(&ui.state)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(trace_title),
        ),
        cols[2],
    );
}

fn draw_flash_map(f: &mut Frame, area: Rect, ui: &Ui) {
    let inner_w = area.width.saturating_sub(2);
    f.render_widget(
        Paragraph::new(flash_lines(&ui.state, inner_w)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("FLASH"),
        ),
        area,
    );
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

fn isp_pin_lines(st: &AppState) -> Vec<Line<'static>> {
    // Faceplate is the PROG bus, not the trailing RELEASE line.
    // After-disc DDR (if present) overlays Hi-Z vs still-driving.
    let in_prog = st.reset_asserted || st.ep_fail.is_some() || st.saw_reset;
    let rst = if let Some(v) = pin_after_disc(st, 2) {
        (v.0.to_string(), v.1)
    } else if st.reset_asserted || (st.ep_fail == Some(true) && in_prog) {
        (
            "ASSERT".into(),
            Style::default().fg(Color::Black).bg(Color::Yellow),
        )
    } else if st.saw_release {
        ("RELEASE".into(), Style::default().fg(Color::Cyan))
    } else {
        ("—".into(), Style::default().fg(Color::DarkGray))
    };
    let mosi = if let Some(v) = pin_after_disc(st, 3) {
        (v.0.to_string(), v.1)
    } else if in_prog {
        ("out".into(), Style::default().fg(Color::Cyan))
    } else {
        ("—".into(), Style::default().fg(Color::DarkGray))
    };
    let miso = if let Some(v) = pin_after_disc(st, 4) {
        (v.0.to_string(), v.1)
    } else if in_prog {
        ("float".into(), Style::default().fg(Color::DarkGray))
    } else {
        ("—".into(), Style::default().fg(Color::DarkGray))
    };
    let sck = match (st.sck_id, st.sck_sw) {
        (Some(id), Some(true)) => {
            (
                format!("SW {id}"),
                Style::default().fg(Color::Black).bg(Color::Yellow),
            )
        }
        (Some(id), _) => (format!("HW {id}"), Style::default().fg(Color::Cyan)),
        _ => ("—".into(), Style::default().fg(Color::DarkGray)),
    };
    let (rst_n, rst_v) = pin_pair("RST", rst.0, rst.1);
    let (mosi_n, mosi_v) = pin_pair("MOSI", mosi.0, mosi.1);
    let (miso_n, miso_v) = pin_pair("MISO", miso.0, miso.1);
    let (sck_n, sck_v) = pin_pair("SCK", sck.0, sck.1);
    let disc = match st.pins_ok {
        Some(true) => Span::styled(
            "Hi-Z OK",
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Some(false) => Span::styled(
            "STILL DRIVING",
            Style::default().fg(Color::White).bg(Color::Red),
        ),
        None => Span::styled("in session", Style::default().fg(Color::DarkGray)),
    };
    vec![
        Line::from(vec![rst_n, Span::raw(" "), mosi_n]),
        Line::from(vec![rst_v, Span::raw(" "), mosi_v]),
        Line::from(vec![miso_n, Span::raw(" "), sck_n]),
        Line::from(vec![miso_v, Span::raw(" "), sck_v]),
        Line::from(vec![
            Span::styled("DISC ", Style::default().fg(Color::DarkGray)),
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
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    };
    Span::styled(format!(" {b:02X} "), style)
}

fn enableprog_lines(st: &AppState) -> Vec<Line<'static>> {
    match (st.ep_tx, st.ep_rx, st.ep_fail) {
        (Some(tx), Some(rx), Some(fail)) => {
            let echo_ok = rx[0] == 0x53;
            let mut tx_spans = vec![Span::styled("TX  ", Style::default().fg(Color::DarkGray))];
            for (i, b) in tx.iter().enumerate() {
                tx_spans.push(hex_cell(*b, i == 1, false));
                if i + 1 != tx.len() {
                    tx_spans.push(Span::raw(" "));
                }
            }
            let mut rx_spans = vec![Span::styled("RX  ", Style::default().fg(Color::DarkGray))];
            for (i, b) in rx.iter().enumerate() {
                rx_spans.push(hex_cell(*b, !fail && i == 1, fail && i == 0));
                if i + 1 != rx.len() {
                    rx_spans.push(Span::raw(" "));
                }
            }
            let res = if fail {
                Span::styled(" FAIL ", Style::default().fg(Color::White).bg(Color::Red))
            } else {
                Span::styled(" PASS ", Style::default().fg(Color::Black).bg(Color::Green))
            };
            let echo = if echo_ok {
                Span::raw("  echo 53 ok")
            } else {
                Span::styled(
                    "  echo 53 missing",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )
            };
            let delay = st
                .snap_delay
                .or(st.err_delay)
                .map(|d| format!("  delay={d}"))
                .unwrap_or_default();
            vec![
                Line::from(tx_spans),
                Line::from(rx_spans),
                Line::from(vec![res, echo, Span::raw(delay)]),
                Line::from(Span::styled(
                    "expect RX[0]=53  (programming enable)",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        _ => vec![Line::from(Span::styled(
            "waiting for ENABLEPROG",
            Style::default().fg(Color::DarkGray),
        ))],
    }
}

fn flash_lines(st: &AppState, inner_w: u16) -> Vec<Line<'static>> {
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
            "no CONT yet (END is authority)",
            Style::default().fg(Color::DarkGray),
        )));
        return lines;
    }
    // Right rail is ~30 cols → one cell per row. Wider → pack.
    let cell_w = 13u16;
    let per_row = (inner_w / cell_w).max(1) as usize;
    let mut row: Vec<Span> = Vec::new();
    let shown = st.memop_pages.len().min(12);
    for (i, &(addr, ok)) in st.memop_pages.iter().take(12).enumerate() {
        let style = if ok {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::White).bg(Color::Red)
        };
        row.push(Span::styled(
            format!(" {addr:#06x} {} ", if ok { "OK" } else { "FAIL" }),
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
    if st.memop_pages.len() > shown {
        lines.push(Line::from(Span::raw("…")));
    }
    lines
}

fn trace_lines(st: &AppState) -> Vec<Line<'static>> {
    let slots = st.trace_slots.unwrap_or(0);
    let life = if st.trace_triggered {
        "FROZEN"
    } else if st.saw_session && !st.saw_session_end {
        "ARMED"
    } else {
        "IDLE"
    };
    let n = 16u16;
    let t_at = if st.trace_triggered { 9 } else { n };
    let mut bar = String::new();
    for i in 0..n {
        if i == t_at {
            bar.push('T');
        } else if i < t_at {
            bar.push('█');
        } else {
            bar.push('·');
        }
    }
    let ov = if st.trace_overflow { "YES" } else { "no" };
    let kind = match st.trace_kind {
        Some(3) => "ENABLEPROG_FAIL",
        Some(4) => "TRACE_OVERFLOW",
        Some(0) | None => "NONE",
        Some(_) => "?",
    };
    vec![
        Line::from(vec![
            Span::raw("["),
            Span::styled(bar, Style::default().fg(Color::Cyan)),
            Span::raw("]  "),
            Span::styled(
                format!(" {life} "),
                if st.trace_triggered {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ]),
        Line::from(Span::styled(
            format!("slots={slots}  overflow={ov}  kind={kind}  post={}", st.trace_post.unwrap_or(0)),
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

fn draw_caps(f: &mut Frame, area: Rect, ui: &Ui) {
    let text = match &ui.state.caps {
        Some(adv) => adv.format_report(&crate::version::banner_short()),
        None => "Capabilities: (waiting for DIAG_CAPS — ISP CONNECT, not USB plug-in)".into(),
    };
    f.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("capabilities")),
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
                    Level::Info => Style::default().fg(Color::Cyan),
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
        format!("timeline dual ({n})  yellow = RELEASE↔READY")
    } else {
        format!("timeline ({n})")
    };
    let table = Table::new(table_rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(table, area, &mut state);
}
