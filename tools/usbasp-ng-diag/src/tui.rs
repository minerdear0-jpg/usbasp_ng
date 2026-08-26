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
use crate::protocol::{DiagFrame, EP2_IN, MEM_EEPROM, MEM_FLASH, MEM_READFLASH};
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
    let wide = f.area().width >= 100;
    let tall = f.area().height >= 24;
    let inst_h = if wide && tall { 8 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(inst_h),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_header(f, chunks[0], ui);
    draw_diag(f, chunks[1], ui);
    if inst_h > 0 {
        draw_instruments(f, chunks[2], ui);
    }
    if ui.show_caps {
        draw_caps(f, chunks[3], ui);
    } else {
        draw_timeline(f, chunks[3], ui);
    }
    let help = Paragraph::new(ui.status.as_str()).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[4]);
}

fn draw_header(f: &mut Frame, area: Rect, ui: &Ui) {
    let sck = match (ui.state.sck_id, ui.state.sck_sw) {
        (Some(id), Some(true)) => format!("SW SCK id={id}"),
        (Some(id), _) => format!("HW SCK id={id}"),
        _ => "SCK —".into(),
    };
    let dual = if ui.uart_path.is_some() { " DUAL " } else { "" };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " diagplane ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  {}  {sck} ", ui.source_label)),
        if ui.faults_only {
            Span::styled(" FAULTS ", Style::default().fg(Color::Black).bg(Color::Red))
        } else {
            Span::styled(" ALL ", Style::default().fg(Color::Black).bg(Color::Green))
        },
        if ui.follow {
            Span::styled(
                " FOLLOW ",
                Style::default().fg(Color::Black).bg(Color::Yellow),
            )
        } else {
            Span::raw(" paused ")
        },
        if ui.wire {
            Span::styled(" WIRE ", Style::default().fg(Color::Black).bg(Color::Magenta))
        } else {
            Span::raw("")
        },
        Span::styled(dual, Style::default().fg(Color::Cyan)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("watch"));
    f.render_widget(title, area);
}

fn draw_diag(f: &mut Frame, area: Rect, ui: &Ui) {
    let (tone, line) = diagnosis(&ui.state);
    let (fg, bg) = match tone {
        DiagTone::Bad => (Color::White, Color::Red),
        DiagTone::Ok => (Color::Black, Color::Green),
        DiagTone::Warn => (Color::Black, Color::Yellow),
        DiagTone::Info => (Color::Cyan, Color::Reset),
    };
    let mut spans = vec![Span::styled(
        format!(" {line} "),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )];
    if ui.state.trace_triggered {
        spans.push(Span::styled(
            "  FROZEN",
            Style::default().fg(Color::Yellow),
        ));
    }
    let phase_line = Line::from(
        phases(&ui.state)
            .iter()
            .enumerate()
            .flat_map(|(i, (name, mark))| {
                let (sym, style) = match mark {
                    PhaseMark::Ok => ("✓", Style::default().fg(Color::Green)),
                    PhaseMark::Fail => {
                        ("×", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                    }
                    PhaseMark::Active => ("▶", Style::default().fg(Color::Yellow)),
                    PhaseMark::Idle => ("·", Style::default().fg(Color::DarkGray)),
                };
                let mut v = Vec::new();
                if i > 0 {
                    v.push(Span::raw("  "));
                }
                v.push(Span::styled(format!("{name} {sym}"), style));
                v
            })
            .collect::<Vec<_>>(),
    );
    let p = Paragraph::new(vec![Line::from(spans), Line::from(""), phase_line]).block(
        Block::default()
            .borders(Borders::ALL)
            .title("diagnosis"),
    );
    f.render_widget(p, area);
}

fn draw_instruments(f: &mut Frame, area: Rect, ui: &Ui) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),
            Constraint::Min(28),
            Constraint::Length(28),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(isp_text(&ui.state))
            .block(Block::default().borders(Borders::ALL).title("ISP")),
        cols[0],
    );

    let (mid, mid_title) = if ui.state.ep_fail == Some(true) || ui.state.memop_kind.is_none() {
        (enableprog_text(&ui.state), "ENABLEPROG")
    } else {
        (flash_text(&ui.state), "FLASH")
    };
    f.render_widget(
        Paragraph::new(mid).block(Block::default().borders(Borders::ALL).title(mid_title)),
        cols[1],
    );

    f.render_widget(
        Paragraph::new(trace_text(&ui.state))
            .block(Block::default().borders(Borders::ALL).title("TRACE")),
        cols[2],
    );
}

fn isp_text(st: &AppState) -> String {
    let rst = if st.reset_asserted {
        "ASSERT"
    } else if st.saw_release {
        "RELEASE"
    } else {
        "—"
    };
    let sck = match (st.sck_id, st.sck_sw) {
        (Some(id), Some(true)) => format!("SW {id}"),
        (Some(id), _) => format!("HW {id}"),
        _ => "—".into(),
    };
    let pins = match st.pins_ok {
        Some(true) => "Hi-Z OK",
        Some(false) => "STILL DRIVING",
        None => "MISO ?",
    };
    format!("RST  {rst}\nSCK  {sck}\nDISC {pins}")
}

fn enableprog_text(st: &AppState) -> String {
    let fmt = |b: [u8; 4]| format!("{:02X} {:02X} {:02X} {:02X}", b[0], b[1], b[2], b[3]);
    match (st.ep_tx, st.ep_rx, st.ep_fail) {
        (Some(tx), Some(rx), Some(fail)) => {
            let res = if fail { "FAIL" } else { "PASS" };
            let echo = if rx[0] == 0x53 {
                "echo 53 ok"
            } else {
                "echo 53 missing"
            };
            let delay = st
                .snap_delay
                .or(st.err_delay)
                .map(|d| format!("  delay={d}"))
                .unwrap_or_default();
            format!(
                "TX  {}\nRX  {}\n{res}  {echo}{delay}",
                fmt(tx),
                fmt(rx)
            )
        }
        _ => "waiting for ENABLEPROG".into(),
    }
}

fn flash_text(st: &AppState) -> String {
    let mem = match st.memop_kind {
        Some(MEM_FLASH) => "FLASH",
        Some(MEM_EEPROM) => "EEPROM",
        Some(MEM_READFLASH) => "READFLASH",
        _ => "—",
    };
    let psz = st.memop_pagesize.unwrap_or(0);
    let end = st
        .memop_end_pages
        .map(|n| n.to_string())
        .unwrap_or_else(|| "…".into());
    let obs = st.memop_pages.len();
    let mut bar = String::new();
    for &(_, ok) in st.memop_pages.iter().take(24) {
        bar.push(if ok { '#' } else { 'X' });
    }
    if st.memop_pages.len() > 24 {
        bar.push('…');
    }
    if bar.is_empty() {
        bar.push_str("(no CONT yet — subsample)");
    }
    format!("{mem} pagesize={psz}  END={end}  observed={obs}\n{bar}")
}

fn trace_text(st: &AppState) -> String {
    let slots = st
        .trace_slots
        .map(|s| s.to_string())
        .unwrap_or_else(|| "—".into());
    let life = if st.trace_triggered {
        "FROZEN"
    } else if st.saw_session && !st.saw_session_end {
        "ARMED"
    } else {
        "IDLE"
    };
    let ov = if st.trace_overflow { "YES" } else { "no" };
    let kind = match st.trace_kind {
        Some(3) => "ENABLEPROG_FAIL",
        Some(4) => "TRACE_OVERFLOW",
        Some(0) | None => "NONE",
        Some(k) => {
            return format!("slots={slots}  {life}\noverflow={ov}  kind={k}");
        }
    };
    let post = st.trace_post.unwrap_or(0);
    format!("slots={slots}  {life}\noverflow={ov}  kind={kind}  post={post}")
}

fn draw_caps(f: &mut Frame, area: Rect, ui: &Ui) {
    let text = match &ui.state.caps {
        Some(adv) => adv.format_report("USBASP-NG DIAG v1"),
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
