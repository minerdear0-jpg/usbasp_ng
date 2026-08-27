use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod analyze;
mod caps;
mod capture;
mod correlate;
mod decoder;
mod demo;
mod evidence;
mod jsonl;
mod protocol;
mod scene;
mod snapshot;
mod state;
mod tui;
mod usb;
mod version;

use caps::CapsAdvert;
use capture::{write_header, CaptureFile, CaptureRecord};
use decoder::{
    format_frame, reassemble_caps, reassemble_enableprog, reassemble_fault_snapshot,
    reassemble_trace_end,
};
use state::AppState;
use jsonl::{
    emit_jsonl_frame, emit_jsonl_semantic, enableprog_failed, snapshot_failed, FaultStats,
};
use protocol::*;
use tui::WatchSource;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutMode {
    Human,
    Json,
    Jsonl,
    Faults,
}

#[derive(Parser, Debug)]
#[command(
    name = "diagplane",
    about = "USBasp NG Diagnostics Plane host",
    version = concat!(env!("CARGO_PKG_VERSION"), "  protocol ", "1"),
    long_version = version::VERSION_LONG
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Decode a capture (.bin with optional USBDIAGv header)
    Decode {
        file: PathBuf,
        #[arg(long, conflicts_with_all = ["jsonl", "faults"])]
        json: bool,
        /// JSON Lines for lnav (pipe or redirect)
        #[arg(long, conflicts_with_all = ["json", "faults"])]
        jsonl: bool,
        /// Fault-oriented human view + summary
        #[arg(long, conflicts_with_all = ["json", "jsonl"])]
        faults: bool,
    },
    /// Replay a capture with timing; no hardware
    Replay {
        file: PathBuf,
        #[arg(long, default_value = "1.0")]
        speed: f64,
        #[arg(long)]
        step: bool,
        #[arg(long, conflicts_with_all = ["jsonl", "faults"])]
        json: bool,
        #[arg(long, conflicts_with_all = ["json", "faults"])]
        jsonl: bool,
        #[arg(long, conflicts_with_all = ["json", "jsonl"])]
        faults: bool,
    },
    /// Synthetic scenarios (no stick)
    Demo {
        scenario: Option<String>,
        #[arg(long)]
        list: bool,
        /// Write capture.bin (USBDIAGv header)
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, conflicts_with_all = ["jsonl", "faults"])]
        json: bool,
        /// Emit JSONL to stdout (lnav-ready): demo X --jsonl | lnav
        #[arg(long, conflicts_with_all = ["json", "faults"])]
        jsonl: bool,
        #[arg(long, conflicts_with_all = ["json", "jsonl"])]
        faults: bool,
    },
    /// Live EP2 → stdout
    Monitor {
        #[arg(default_value = "")]
        serial: String,
        #[arg(long)]
        json: bool,
    },
    /// TUI watch: file / demo / live EP2 / JSONL
    Watch {
        /// Live USB iSerial (empty = first composite)
        #[arg(long, default_value = "")]
        serial: String,
        /// Load a capture.bin
        #[arg(long, conflicts_with_all = ["demo", "diag"])]
        file: Option<PathBuf>,
        /// Load a demo scenario
        #[arg(long, conflicts_with_all = ["file", "diag"])]
        demo: Option<String>,
        /// Programmer JSONL (`decode FILE --jsonl`)
        #[arg(long, conflicts_with_all = ["file", "demo"])]
        diag: Option<PathBuf>,
        /// Target UART log — dual-column event order (RELEASE ↔ READY, not µs)
        #[arg(long)]
        uart: Option<PathBuf>,
    },
    /// Record EP2 → .bin (writes USBDIAGv header)
    Record {
        #[arg(default_value = "")]
        serial: String,
        out: PathBuf,
    },
    /// Print firmware + board capability map (gate on caps, not version)
    Capabilities {
        /// Live USB iSerial (empty = first composite)
        #[arg(long, default_value = "")]
        serial: String,
        /// Load a capture.bin
        #[arg(long, conflicts_with = "demo")]
        file: Option<PathBuf>,
        /// Load a demo scenario
        #[arg(long, conflicts_with = "file")]
        demo: Option<String>,
        /// Seconds to wait for ISP CONNECT → DIAG_CAPS (not USB plug-in)
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    /// One coherent instrument dump (USB/ISP/TRACE/MEMOP) — Analyze, no extra wire
    Snapshot {
        /// Live USB iSerial (empty = first composite)
        #[arg(long, default_value = "")]
        serial: String,
        /// Load a capture.bin
        #[arg(long, conflicts_with_all = ["demo", "diag"])]
        file: Option<PathBuf>,
        /// Load a demo scenario
        #[arg(long, conflicts_with_all = ["file", "diag"])]
        demo: Option<String>,
        /// Programmer JSONL (`decode FILE --jsonl`)
        #[arg(long, conflicts_with_all = ["file", "demo"])]
        diag: Option<PathBuf>,
        /// Seconds to wait for SESSION_END on live EP2
        #[arg(long, default_value = "30")]
        timeout: u64,
        #[arg(long)]
        json: bool,
    },
    /// Frozen diagnostic evidence (expected/observed/verdict) — host container, not EP2
    Evidence {
        /// Live USB iSerial (empty = first composite)
        #[arg(long, default_value = "")]
        serial: String,
        #[arg(long, conflicts_with_all = ["demo", "diag"])]
        file: Option<PathBuf>,
        #[arg(long, conflicts_with_all = ["file", "diag"])]
        demo: Option<String>,
        #[arg(long, conflicts_with_all = ["file", "demo"])]
        diag: Option<PathBuf>,
        #[arg(long, default_value = "30")]
        timeout: u64,
        #[arg(long)]
        json: bool,
    },
    /// Offline analysis: Evidence → Findings → Verdict (no firmware philosophy)
    Analyze {
        #[arg(long, default_value = "")]
        serial: String,
        #[arg(long, conflicts_with_all = ["demo", "diag"])]
        file: Option<PathBuf>,
        #[arg(long, conflicts_with_all = ["file", "diag"])]
        demo: Option<String>,
        #[arg(long, conflicts_with_all = ["file", "demo"])]
        diag: Option<PathBuf>,
        #[arg(long, default_value = "30")]
        timeout: u64,
        #[arg(long)]
        json: bool,
        /// Write `.usbasp2e` (evidence + analysis JSON)
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Dual-truth event order: EP2 JSONL ↔ oracle UART (not absolute µs)
    Correlate {
        /// Programmer capture as JSONL (`decode FILE --jsonl`)
        #[arg(long)]
        diag: PathBuf,
        /// Target UART log (`@TTTTTTTT EVENT,...` lines)
        #[arg(long)]
        uart: PathBuf,
        /// JSON Lines instead of human table
        #[arg(long)]
        jsonl: bool,
    },
}

fn out_mode(json: bool, jsonl: bool, faults: bool) -> OutMode {
    if jsonl {
        OutMode::Jsonl
    } else if faults {
        OutMode::Faults
    } else if json {
        OutMode::Json
    } else {
        OutMode::Human
    }
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e:#}");
        std::process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Decode {
            file,
            json,
            jsonl,
            faults,
        } => {
            let cap = CaptureFile::load(&file)?;
            print_capture(&cap, out_mode(json, jsonl, faults), None, false)
        }
        Cmd::Replay {
            file,
            speed,
            step,
            json,
            jsonl,
            faults,
        } => {
            let cap = CaptureFile::load(&file)?;
            print_capture(&cap, out_mode(json, jsonl, faults), Some(speed), step)
        }
        Cmd::Demo {
            scenario,
            list,
            out,
            json,
            jsonl,
            faults,
        } => cmd_demo(
            scenario.as_deref(),
            list,
            out.as_ref(),
            out_mode(json, jsonl, faults),
        ),
        Cmd::Monitor { serial, json } => cmd_monitor(&serial, json),
        Cmd::Watch {
            serial,
            file,
            demo,
            diag,
            uart,
        } => {
            let src = if let Some(path) = diag {
                WatchSource::Jsonl(path)
            } else if let Some(path) = file {
                WatchSource::File(path)
            } else if let Some(name) = demo {
                WatchSource::Demo(name)
            } else {
                WatchSource::Live { serial }
            };
            tui::run(src, uart)
        }
        Cmd::Record { serial, out } => cmd_record(&serial, &out),
        Cmd::Capabilities {
            serial,
            file,
            demo,
            timeout,
        } => cmd_capabilities(&serial, file.as_ref(), demo.as_deref(), timeout),
        Cmd::Snapshot {
            serial,
            file,
            demo,
            diag,
            timeout,
            json,
        } => cmd_snapshot(
            &serial,
            file.as_ref(),
            demo.as_deref(),
            diag.as_ref(),
            timeout,
            json,
        ),
        Cmd::Evidence {
            serial,
            file,
            demo,
            diag,
            timeout,
            json,
        } => cmd_evidence(
            &serial,
            file.as_ref(),
            demo.as_deref(),
            diag.as_ref(),
            timeout,
            json,
        ),
        Cmd::Analyze {
            serial,
            file,
            demo,
            diag,
            timeout,
            json,
            out,
        } => cmd_analyze(
            &serial,
            file.as_ref(),
            demo.as_deref(),
            diag.as_ref(),
            timeout,
            json,
            out.as_ref(),
        ),
        Cmd::Correlate { diag, uart, jsonl } => {
            let (events, sync) = correlate::correlate_files(&diag, &uart)?;
            if jsonl {
                correlate::emit_jsonl(&events, &sync);
            } else {
                correlate::emit_human(&events, &sync);
            }
            Ok(())
        }
    }
}

fn cmd_capabilities(
    serial: &str,
    file: Option<&PathBuf>,
    demo_name: Option<&str>,
    timeout_s: u64,
) -> Result<()> {
    let mut st = AppState::default();
    let device_label;
    if let Some(path) = file {
        let cap = CaptureFile::load(path)?;
        st.ingest_capture(&cap);
        device_label = format!("file:{}", path.display());
    } else if let Some(name) = demo_name {
        let cap = demo::build_scenario(name)?;
        st.ingest_capture(&cap);
        device_label = format!("demo:{name}");
    } else {
        let h = usb::open_composite(serial)?;
        device_label = h.serial.clone();
        eprintln!("{}", version::banner_short());
        eprintln!();
        eprintln!("device:    {device_label}");
        eprintln!("transport: HIDUART (EP2)");
        eprintln!();
        eprintln!("waiting for ISP session…");
        eprintln!("  (DIAG_CAPS is emitted on avrdude CONNECT, not on USB plug-in)");
        eprintln!("  start ISP within {timeout_s}s, e.g.:");
        eprintln!("    avrdude -c usbasp -P usb:{device_label} -p m8 -B 8");
        eprintln!();
        let deadline = SystemTime::now() + Duration::from_secs(timeout_s);
        let mut buf = [0u8; 8];
        let mut saw_frame = false;
        while SystemTime::now() < deadline && st.caps.is_none() {
            match h
                .handle
                .read_interrupt(EP2_IN, &mut buf, Duration::from_millis(200))
            {
                Ok(n) if n >= 6 => {
                    if let Some(f) = DiagFrame::from_report(&buf[..n]) {
                        if f.ty != 0 {
                            saw_frame = true;
                            if f.ty == HELLO {
                                eprintln!("CONNECT / HELLO");
                            }
                            let ns = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos() as u64;
                            st.push_frame(ns, f);
                            if st.caps.is_some() {
                                eprintln!("CAPABILITIES received");
                                eprintln!();
                            }
                        }
                    }
                }
                Ok(_) | Err(rusb::Error::Timeout) => {}
                Err(e) => bail!("USB read: {e}"),
            }
        }
        if st.caps.is_none() {
            eprintln!("NO DIAG SESSION");
            eprintln!("device present, ISP CONNECT not observed");
            if saw_frame {
                eprintln!("(saw other EP2 frames, but no DIAG_CAPS advertisement)");
            }
            eprintln!();
            eprintln!("hint: keep this command running, then start avrdude;");
            eprintln!("      or: --demo capabilities_yel0");
            bail!("no DIAG_CAPS (no ISP diagnostic session)");
        }
    }

    let Some(adv) = st.caps else {
        bail!("no DIAG_CAPS advertisement seen");
    };
    let schema = st.hello_schema.unwrap_or(SCHEMA_V1);
    println!("{}", version::banner_short());
    println!("device schema: {schema} (HELLO)");
    if !device_label.is_empty() {
        println!();
        println!("device: {device_label}");
    }
    println!();
    println!("CAPABILITIES");
    println!("  firmware: 0x{:08x}", adv.firmware.0);
    println!("  board:    0x{:08x}", adv.board.0);
    println!();
    // Checklist view (gate on bits, never version)
    let f = adv.firmware;
    let mark = |b: bool| if b { "✓" } else { "✗" };
    println!("  TRACE       {}", mark(f.contains(caps::DiagCaps::TRACE)));
    println!("  TRIGGER     {}", mark(f.contains(caps::DiagCaps::TRIGGER)));
    println!(
        "  PRETRIGGER  {}",
        mark(f.contains(caps::DiagCaps::PRETRIGGER))
    );
    println!(
        "  TIMESTAMP   {}",
        mark(f.contains(caps::DiagCaps::TIMESTAMP))
    );
    println!(
        "  SESSION     {}",
        mark(f.contains(caps::DiagCaps::SESSION))
    );
    println!(
        "  SNAPSHOT    {}",
        mark(f.contains(caps::DiagCaps::SNAPSHOT))
    );
    println!(
        "  LINE_FAULT  {}",
        mark(f.contains(caps::DiagCaps::LINE_FAULT))
    );
    println!();
    let b = adv.board;
    println!("BOARD");
    println!(
        "  target UART       {}",
        mark(b.contains(caps::BoardCaps::TARGET_UART))
    );
    println!(
        "  sck jumper        {}",
        mark(b.contains(caps::BoardCaps::SCK_JUMPER))
    );
    println!(
        "  physical capture  {}",
        mark(b.contains(caps::BoardCaps::PHYSICAL_CAPTURE))
    );
    if let Some(flags) = st.hello_flags {
        println!();
        println!("HELLO.flags=0x{flags:02x}  (legacy compact; prefer DIAG_CAPS)");
    }
    Ok(())
}

fn ingest_state(
    serial: &str,
    file: Option<&PathBuf>,
    demo_name: Option<&str>,
    diag: Option<&PathBuf>,
    timeout_s: u64,
    wait_session_end: bool,
) -> Result<(AppState, String)> {
    let mut st = AppState::default();
    let source;
    if let Some(path) = file {
        let cap = CaptureFile::load(path)?;
        st.ingest_capture(&cap);
        source = format!("file:{}", path.display());
    } else if let Some(name) = demo_name {
        let cap = demo::build_scenario(name)?;
        st.ingest_capture(&cap);
        source = format!("demo:{name}");
    } else if let Some(path) = diag {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        st.ingest_jsonl(&text)?;
        source = format!("jsonl:{}", path.display());
    } else {
        let h = usb::open_composite(serial)?;
        source = format!("live:{}", h.serial);
        eprintln!("{}", version::banner_short());
        eprintln!("device:    {}", h.serial);
        if wait_session_end {
            eprintln!("waiting for SESSION_END (ISP) within {timeout_s}s…");
        }
        let deadline = SystemTime::now() + Duration::from_secs(timeout_s);
        let mut buf = [0u8; 8];
        while SystemTime::now() < deadline && !(wait_session_end && st.saw_session_end) {
            match h
                .handle
                .read_interrupt(EP2_IN, &mut buf, Duration::from_millis(200))
            {
                Ok(n) if n >= 6 => {
                    if let Some(f) = DiagFrame::from_report(&buf[..n]) {
                        if f.ty != 0 {
                            let ns = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos() as u64;
                            st.push_frame(ns, f);
                        }
                    }
                }
                Ok(_) | Err(rusb::Error::Timeout) => {}
                Err(e) => bail!("USB read: {e}"),
            }
        }
        if st.events.is_empty() {
            bail!("no EP2 frames (start avrdude while this waits, or use --demo)");
        }
    }
    Ok((st, source))
}

fn cmd_snapshot(
    serial: &str,
    file: Option<&PathBuf>,
    demo_name: Option<&str>,
    diag: Option<&PathBuf>,
    timeout_s: u64,
    json: bool,
) -> Result<()> {
    let (st, source) = ingest_state(serial, file, demo_name, diag, timeout_s, true)?;
    let complete = st.saw_session_end;
    let snap = snapshot::from_state(&source, &st, complete);
    if json {
        snap.emit_json()
    } else {
        snap.emit_human();
        Ok(())
    }
}

fn cmd_evidence(
    serial: &str,
    file: Option<&PathBuf>,
    demo_name: Option<&str>,
    diag: Option<&PathBuf>,
    timeout_s: u64,
    json: bool,
) -> Result<()> {
    let (st, source) = ingest_state(serial, file, demo_name, diag, timeout_s, true)?;
    let complete = st.saw_session_end;
    let ev = evidence::from_state(&source, &st, complete);
    if json {
        ev.emit_json()
    } else {
        ev.emit_human();
        Ok(())
    }
}

fn cmd_analyze(
    serial: &str,
    file: Option<&PathBuf>,
    demo_name: Option<&str>,
    diag: Option<&PathBuf>,
    timeout_s: u64,
    json: bool,
    out: Option<&PathBuf>,
) -> Result<()> {
    let (st, source) = ingest_state(serial, file, demo_name, diag, timeout_s, true)?;
    let complete = st.saw_session_end;
    let ev = evidence::from_state(&source, &st, complete);
    let pack = analyze::Usbasp2eFile::from_evidence(ev);
    if let Some(path) = out {
        pack.write_path(path)?;
        eprintln!("wrote {}", path.display());
    }
    if json {
        pack.emit_json()
    } else {
        pack.emit_human();
        Ok(())
    }
}

fn cmd_demo(
    scenario: Option<&str>,
    list: bool,
    out: Option<&PathBuf>,
    mode: OutMode,
) -> Result<()> {
    if list || scenario.is_none() && out.is_none() {
        for s in demo::list_scenarios() {
            println!("{s}");
        }
        if scenario.is_none() && !list {
            eprintln!(
                "usage: demo <scenario> [--out file.bin] [--jsonl|--faults] | demo --list"
            );
        }
        return Ok(());
    }
    let name = scenario.unwrap();
    let cap = demo::build_scenario(name)?;
    if let Some(path) = out {
        cap.write(path, true)?;
        eprintln!(
            "wrote {path:?} ({} records, USBDIAGv header)",
            cap.records.len()
        );
        if mode == OutMode::Human {
            return Ok(());
        }
        // allow --out together with --jsonl/--faults (print after write)
    }
    print_capture(&cap, mode, None, false)
}

fn print_capture(
    cap: &CaptureFile,
    mode: OutMode,
    speed: Option<f64>,
    step: bool,
) -> Result<()> {
    if mode == OutMode::Human || mode == OutMode::Faults {
        if let Some(h) = &cap.header {
            eprintln!(
                "capture header: format={} schema={} record={}",
                h.format_version, h.diag_schema, h.record_size
            );
        } else {
            eprintln!("capture header: (legacy, no USBDIAGv)");
        }
    }
    if mode == OutMode::Faults {
        println!("=== FAULT VIEW ===");
    }

    let mut ep_buf: Vec<DiagFrame> = Vec::new();
    let mut snap_buf: Vec<DiagFrame> = Vec::new();
    let mut caps_buf: Vec<DiagFrame> = Vec::new();
    let mut te_buf: Vec<DiagFrame> = Vec::new();
    let mut ep_ns: Vec<u64> = Vec::new();
    let mut snap_ns: Vec<u64> = Vec::new();
    let mut prev_ns: Option<u64> = None;
    let mut stats = FaultStats::default();

    for rec in &cap.records {
        if let Some(rate) = speed {
            if step {
                eprint!("[Enter] ");
                let _ = io::stderr().flush();
                let mut line = String::new();
                let _ = io::stdin().read_line(&mut line);
            } else if rate > 0.0 {
                if let Some(prev) = prev_ns {
                    let delta_ns = rec.host_ns.saturating_sub(prev);
                    let wait_ns = (delta_ns as f64 / rate) as u64;
                    if wait_ns > 0 {
                        thread::sleep(Duration::from_nanos(wait_ns.min(5_000_000_000)));
                    }
                }
            }
            prev_ns = Some(rec.host_ns);
        }

        let Some(f) = rec.frame() else {
            continue;
        };
        if f.ty == 0 {
            continue;
        }

        stats.note_frame(&f);

        match mode {
            OutMode::Json => {
                let v = serde_json::json!({
                    "host_ns": rec.host_ns,
                    "t_tick": f.timestamp,
                    "type": type_name_owned(f.ty),
                    "flags": f.flags,
                    "a": f.a,
                    "b": f.b,
                });
                println!("{v}");
            }
            OutMode::Jsonl => {
                emit_jsonl_frame(rec.host_ns, &f);
            }
            OutMode::Human => {
                println!("{}  {}", rec.host_ns, format_frame(&f));
            }
            OutMode::Faults => {
                if matches!(f.ty, ERROR | TRACE_OVERFLOW) {
                    println!("{}  {}", rec.host_ns, format_frame(&f));
                }
            }
        }

        match f.ty {
            ENABLEPROG => {
                snap_buf.clear();
                snap_ns.clear();
                caps_buf.clear();
                te_buf.clear();
                ep_buf.push(f);
                ep_ns.push(rec.host_ns);
                if ep_buf.len() == 4 {
                    let fail = enableprog_failed(&ep_buf);
                    stats.note_enableprog(fail);
                    if let Some(line) = reassemble_enableprog(&ep_buf) {
                        match mode {
                            OutMode::Human => println!("{:20}>> {line}", ""),
                            OutMode::Jsonl => {
                                let level = if fail { "error" } else { "info" };
                                emit_jsonl_semantic(
                                    *ep_ns.last().unwrap_or(&rec.host_ns),
                                    "enableprog",
                                    level,
                                    &line,
                                );
                            }
                            OutMode::Faults if fail => {
                                for (ns, fr) in ep_ns.iter().zip(ep_buf.iter()) {
                                    println!("{ns}  {}", format_frame(fr));
                                }
                                println!("{:20}>> {line}", "");
                            }
                            _ => {}
                        }
                    }
                    ep_buf.clear();
                    ep_ns.clear();
                }
            }
            FAULT_SNAPSHOT => {
                ep_buf.clear();
                ep_ns.clear();
                caps_buf.clear();
                te_buf.clear();
                snap_buf.push(f);
                snap_ns.push(rec.host_ns);
                if snap_buf.len() == 4 {
                    let fail = snapshot_failed(&snap_buf);
                    if fail {
                        stats.note_snapshot_fail();
                    }
                    if let Some(line) = reassemble_fault_snapshot(&snap_buf) {
                        match mode {
                            OutMode::Human => println!("{:20}>> {line}", ""),
                            OutMode::Jsonl => {
                                let level = if fail { "error" } else { "info" };
                                emit_jsonl_semantic(
                                    *snap_ns.last().unwrap_or(&rec.host_ns),
                                    "fault_snapshot",
                                    level,
                                    &line,
                                );
                            }
                            OutMode::Faults if fail => {
                                for (ns, fr) in snap_ns.iter().zip(snap_buf.iter()) {
                                    println!("{ns}  {}", format_frame(fr));
                                }
                                println!("{:20}>> {line}", "");
                            }
                            _ => {}
                        }
                    }
                    snap_buf.clear();
                    snap_ns.clear();
                }
            }
            CAPS => {
                ep_buf.clear();
                ep_ns.clear();
                snap_buf.clear();
                snap_ns.clear();
                te_buf.clear();
                caps_buf.push(f);
                if caps_buf.len() == 4 {
                    if let Some(line) = reassemble_caps(&caps_buf) {
                        match mode {
                            OutMode::Human => println!("{:20}>> {line}", ""),
                            OutMode::Jsonl => {
                                emit_jsonl_semantic(rec.host_ns, "caps", "info", &line);
                            }
                            _ => {}
                        }
                    }
                    let _ = CapsAdvert::from_frames(&caps_buf);
                    caps_buf.clear();
                }
            }
            TRACE_END => {
                ep_buf.clear();
                ep_ns.clear();
                snap_buf.clear();
                snap_ns.clear();
                caps_buf.clear();
                te_buf.push(f);
                if te_buf.len() == 4 {
                    if let Some(line) = reassemble_trace_end(&te_buf) {
                        match mode {
                            OutMode::Human => println!("{:20}>> {line}", ""),
                            OutMode::Jsonl => {
                                let level = if te_buf[1].flags & 0x80 != 0
                                    || te_buf[2].flags & 0x80 != 0
                                {
                                    "warning"
                                } else {
                                    "info"
                                };
                                emit_jsonl_semantic(rec.host_ns, "trace_end", level, &line);
                            }
                            OutMode::Faults
                                if te_buf[1].flags & 0x80 != 0
                                    || te_buf[2].flags & 0x80 != 0 =>
                            {
                                println!("{:20}>> {line}", "");
                            }
                            _ => {}
                        }
                    }
                    te_buf.clear();
                }
            }
            _ => {
                ep_buf.clear();
                ep_ns.clear();
                snap_buf.clear();
                snap_ns.clear();
                caps_buf.clear();
                te_buf.clear();
            }
        }
    }

    if mode == OutMode::Faults {
        println!();
        stats.print_summary();
    }
    Ok(())
}

fn cmd_monitor(serial: &str, json: bool) -> Result<()> {
    let h = usb::open_composite(serial)?;
    if !json {
        println!("USBasp NG Diagnostics  serial={:?}  schema=DIAG v1", h.serial);
    }
    let mut buf = [0u8; 8];
    let mut ep_buf = Vec::new();
    let mut snap_buf = Vec::new();
    let mut caps_buf = Vec::new();
    let mut te_buf = Vec::new();
    loop {
        match h
            .handle
            .read_interrupt(EP2_IN, &mut buf, Duration::from_millis(1000))
        {
            Ok(n) if n >= 6 => {
                let Some(f) = DiagFrame::from_report(&buf[..n]) else {
                    continue;
                };
                if f.ty == 0 {
                    continue;
                }
                if json {
                    let v = serde_json::json!({
                        "t_tick": f.timestamp,
                        "type": type_name_owned(f.ty),
                        "flags": f.flags,
                        "a": f.a,
                        "b": f.b,
                    });
                    println!("{v}");
                } else {
                    let wall = humantime_now();
                    println!("[{wall}] {}", format_frame(&f));
                    match f.ty {
                        ENABLEPROG => {
                            snap_buf.clear();
                            caps_buf.clear();
                            te_buf.clear();
                            ep_buf.push(f);
                            if ep_buf.len() == 4 {
                                if let Some(line) = reassemble_enableprog(&ep_buf) {
                                    println!("         >> {line}");
                                }
                                ep_buf.clear();
                            }
                        }
                        FAULT_SNAPSHOT => {
                            ep_buf.clear();
                            caps_buf.clear();
                            te_buf.clear();
                            snap_buf.push(f);
                            if snap_buf.len() == 4 {
                                if let Some(line) = reassemble_fault_snapshot(&snap_buf) {
                                    println!("         >> {line}");
                                }
                                snap_buf.clear();
                            }
                        }
                        CAPS => {
                            ep_buf.clear();
                            snap_buf.clear();
                            te_buf.clear();
                            caps_buf.push(f);
                            if caps_buf.len() == 4 {
                                if let Some(line) = reassemble_caps(&caps_buf) {
                                    println!("         >> {line}");
                                }
                                caps_buf.clear();
                            }
                        }
                        TRACE_END => {
                            ep_buf.clear();
                            snap_buf.clear();
                            caps_buf.clear();
                            te_buf.push(f);
                            if te_buf.len() == 4 {
                                if let Some(line) = reassemble_trace_end(&te_buf) {
                                    println!("         >> {line}");
                                }
                                te_buf.clear();
                            }
                        }
                        _ => {
                            ep_buf.clear();
                            snap_buf.clear();
                            caps_buf.clear();
                            te_buf.clear();
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(rusb::Error::Timeout) => {}
            Err(e) => bail!("USB read: {e}"),
        }
    }
}

fn type_name_owned(ty: u8) -> String {
    decoder::type_name(ty).to_ascii_lowercase()
}

fn humantime_now() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs() % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

fn cmd_record(serial: &str, out: &PathBuf) -> Result<()> {
    let h = usb::open_composite(serial)?;
    eprintln!(
        "recording serial={:?} → {out:?} (USBDIAGv header)",
        h.serial
    );
    let mut file = File::create(out).with_context(|| format!("create {out:?}"))?;
    write_header(&mut file)?;
    let mut buf = [0u8; 8];
    let mut n = 0u64;
    loop {
        match h
            .handle
            .read_interrupt(EP2_IN, &mut buf, Duration::from_millis(1000))
        {
            Ok(k) if k >= 6 => {
                let ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                let mut report = [0u8; 8];
                report[..k.min(8)].copy_from_slice(&buf[..k.min(8)]);
                let rec = CaptureRecord {
                    host_ns: ns,
                    report,
                };
                file.write_all(&rec.to_bytes())?;
                file.flush()?;
                n += 1;
                if let Some(f) = rec.frame() {
                    eprintln!("{n:5} {}", format_frame(&f));
                }
            }
            Ok(_) | Err(rusb::Error::Timeout) => {}
            Err(e) => bail!("USB read: {e}"),
        }
    }
}
