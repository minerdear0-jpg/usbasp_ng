//! Ratatui watch UI for captures / demo / live EP2.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::capture::CaptureFile;
use crate::demo;
use crate::protocol::{DiagFrame, EP2_IN};
use crate::state::{AppState, Level};
use crate::usb::CompositeHandle;

pub enum WatchSource {
    File(PathBuf),
    Demo(String),
    Live { serial: String },
}

struct Ui {
    state: AppState,
    source_label: String,
    faults_only: bool,
    show_caps: bool,
    list_state: ListState,
    follow: bool,
    status: String,
}

impl Ui {
    fn new(source_label: String, state: AppState) -> Self {
        let mut list_state = ListState::default();
        if !state.events.is_empty() {
            list_state.select(Some(state.events.len().saturating_sub(1)));
        }
        Self {
            state,
            source_label,
            faults_only: false,
            show_caps: false,
            list_state,
            follow: true,
            status: "q quit  f faults  c caps  j/k scroll  g/G top/bot  Space follow".into(),
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        self.state
            .events
            .iter()
            .enumerate()
            .filter(|(_, e)| !self.faults_only || e.is_fault)
            .map(|(i, _)| i)
            .collect()
    }

    fn scroll_rel(&mut self, delta: isize) {
        self.follow = false;
        let vis = self.visible_indices();
        if vis.is_empty() {
            self.list_state.select(None);
            return;
        }
        let cur = self.list_state.selected().unwrap_or(0);
        let cur_vis = vis.iter().position(|&i| i == cur).unwrap_or(0);
        let next = cur_vis
            .saturating_add_signed(delta)
            .min(vis.len().saturating_sub(1));
        self.list_state.select(Some(vis[next]));
    }

    fn jump_top(&mut self) {
        self.follow = false;
        let vis = self.visible_indices();
        self.list_state.select(vis.first().copied());
    }

    fn jump_bot(&mut self) {
        self.follow = true;
        let vis = self.visible_indices();
        self.list_state.select(vis.last().copied());
    }

    fn on_new_events(&mut self) {
        if self.follow {
            let vis = self.visible_indices();
            self.list_state.select(vis.last().copied());
        }
    }
}

pub fn run(source: WatchSource) -> Result<()> {
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
        WatchSource::Live { serial } => {
            let handle = crate::usb::open_composite(&serial)?;
            (
                format!("live:{}", handle.serial),
                AppState::default(),
                Some(handle),
            )
        }
    };

    let mut ui = Ui::new(label, state);
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

        // Live USB poll (short)
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

fn draw(f: &mut ratatui::Frame, ui: &Ui) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(if ui.show_caps { 12 } else { 4 }),
            Constraint::Length(2),
        ])
        .split(f.area());

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " usbasp-ng-diag ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  {}  ", ui.source_label)),
        if ui.faults_only {
            Span::styled(" FAULTS ", Style::default().fg(Color::Black).bg(Color::Red))
        } else {
            Span::styled(" ALL ", Style::default().fg(Color::Black).bg(Color::Green))
        },
        if ui.follow {
            Span::styled(" FOLLOW ", Style::default().fg(Color::Black).bg(Color::Yellow))
        } else {
            Span::raw(" paused ")
        },
        if ui.show_caps {
            Span::styled(" CAPS ", Style::default().fg(Color::Black).bg(Color::Magenta))
        } else {
            Span::raw("")
        },
    ]))
    .block(Block::default().borders(Borders::ALL).title("watch"));
    f.render_widget(title, chunks[0]);

    let vis = ui.visible_indices();
    let items: Vec<ListItem> = vis
        .iter()
        .map(|&i| {
            let e = &ui.state.events[i];
            let style = match e.level {
                Level::Error => Style::default().fg(Color::Red),
                Level::Warn => Style::default().fg(Color::Yellow),
                Level::Info => Style::default().fg(Color::Cyan),
                Level::Debug => Style::default().fg(Color::DarkGray),
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}  {}", e.host_ns, e.text),
                style,
            )))
        })
        .collect();

    // Map selected absolute index → visible row for ListState
    let mut list_state = ListState::default();
    if let Some(sel) = ui.list_state.selected() {
        if let Some(row) = vis.iter().position(|&i| i == sel) {
            list_state.select(Some(row));
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("events ({})", vis.len())),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    let summary = if ui.show_caps {
        match &ui.state.caps {
            Some(adv) => adv.format_report("USBASP-NG DIAG v1"),
            None => "Capabilities: (waiting for DIAG_CAPS — re-plug or use demo capabilities_yel0)"
                .into(),
        }
    } else {
        let s = &ui.state.stats;
        let caps_line = ui
            .state
            .caps
            .as_ref()
            .map(|c| c.summary_line())
            .unwrap_or_else(|| "caps: —".into());
        let slots = ui
            .state
            .trace_slots
            .map(|s| s.to_string())
            .unwrap_or_else(|| "—".into());
        let ov = if ui.state.trace_overflow { "YES" } else { "no" };
        format!(
            "ENABLEPROG PASS={} FAIL={}   SNAPSHOT FAIL={}   ERROR={}   OVERFLOW={} dropped={}\n{caps_line}\nTRACE slots={slots}  overflow={ov}",
            s.enableprog_pass, s.enableprog_fail, s.snapshot_fail, s.errors, s.overflows, s.dropped
        )
    };
    let summary = Paragraph::new(summary)
        .block(Block::default().borders(Borders::ALL).title(if ui.show_caps {
            "capabilities"
        } else {
            "summary"
        }));
    f.render_widget(summary, chunks[2]);

    let help = Paragraph::new(ui.status.as_str())
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[3]);
}