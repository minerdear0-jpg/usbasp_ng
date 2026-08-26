use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use rusb::{Direction, TransferType};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod capture;
mod decoder;
mod demo;
mod protocol;

use capture::{write_header, CaptureFile, CaptureRecord};
use decoder::{format_frame, reassemble_enableprog, reassemble_fault_snapshot};
use protocol::*;

#[derive(Parser, Debug)]
#[command(name = "usbasp-ng-diag", about = "USBasp NG Diagnostics Plane (DIAG v1)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Decode a capture (.bin with optional USBDIAGv header)
    Decode {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Replay a capture with timing (--speed / --step); no hardware
    Replay {
        file: PathBuf,
        /// Playback rate (1.0 = realtime host_ns deltas; 0 = as fast as possible)
        #[arg(long, default_value = "1.0")]
        speed: f64,
        /// Wait for Enter between records
        #[arg(long)]
        step: bool,
        #[arg(long)]
        json: bool,
    },
    /// Synthetic scenarios (no stick)
    Demo {
        /// Scenario name; omit with --list
        scenario: Option<String>,
        #[arg(long)]
        list: bool,
        /// Write capture.bin (with header) instead of printing
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Live EP2 → stdout
    Monitor {
        #[arg(default_value = "")]
        serial: String,
        #[arg(long)]
        json: bool,
    },
    /// Record EP2 → .bin (writes USBDIAGv header)
    Record {
        #[arg(default_value = "")]
        serial: String,
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Decode { file, json } => {
            let cap = CaptureFile::load(&file)?;
            print_capture(&cap, json, None, false)
        }
        Cmd::Replay {
            file,
            speed,
            step,
            json,
        } => {
            let cap = CaptureFile::load(&file)?;
            print_capture(&cap, json, Some(speed), step)
        }
        Cmd::Demo {
            scenario,
            list,
            out,
        } => cmd_demo(scenario.as_deref(), list, out.as_ref()),
        Cmd::Monitor { serial, json } => cmd_monitor(&serial, json),
        Cmd::Record { serial, out } => cmd_record(&serial, &out),
    }
}

fn cmd_demo(scenario: Option<&str>, list: bool, out: Option<&PathBuf>) -> Result<()> {
    if list || scenario.is_none() && out.is_none() {
        for s in demo::list_scenarios() {
            println!("{s}");
        }
        if scenario.is_none() && !list {
            eprintln!("usage: demo <scenario> [--out file.bin] | demo --list");
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
        return Ok(());
    }
    print_capture(&cap, false, None, false)
}

fn print_capture(
    cap: &CaptureFile,
    json: bool,
    speed: Option<f64>,
    step: bool,
) -> Result<()> {
    if let Some(h) = &cap.header {
        if !json {
            eprintln!(
                "capture header: format={} schema={} record={}",
                h.format_version, h.diag_schema, h.record_size
            );
        }
    } else if !json {
        eprintln!("capture header: (legacy, no USBDIAGv)");
    }

    let mut ep_buf = Vec::new();
    let mut snap_buf = Vec::new();
    let mut prev_ns: Option<u64> = None;

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

        if json {
            let v = serde_json::json!({
                "host_ns": rec.host_ns,
                "t_tick": f.timestamp,
                "type": type_name_owned(f.ty),
                "flags": f.flags,
                "a": f.a,
                "b": f.b,
            });
            println!("{v}");
        } else {
            println!("{}  {}", rec.host_ns, format_frame(&f));
        }

        match f.ty {
            ENABLEPROG => {
                snap_buf.clear();
                ep_buf.push(f);
                if ep_buf.len() == 4 {
                    if let Some(line) = reassemble_enableprog(&ep_buf) {
                        if !json {
                            println!("{:20}>> {line}", "");
                        }
                    }
                    ep_buf.clear();
                }
            }
            FAULT_SNAPSHOT => {
                ep_buf.clear();
                snap_buf.push(f);
                if snap_buf.len() == 4 {
                    if let Some(line) = reassemble_fault_snapshot(&snap_buf) {
                        if !json {
                            println!("{:20}>> {line}", "");
                        }
                    }
                    snap_buf.clear();
                }
            }
            _ => {
                ep_buf.clear();
                snap_buf.clear();
            }
        }
    }
    Ok(())
}

fn open_composite(want_serial: &str) -> Result<(rusb::DeviceHandle<rusb::GlobalContext>, String)> {
    for dev in rusb::devices().context("list USB")?.iter() {
        let desc = match dev.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };
        if desc.vendor_id() != VID || desc.product_id() != PID {
            continue;
        }
        let ver = desc.device_version();
        if ver.major() == 2 && ver.minor() == 3 {
            continue;
        }
        let Ok(handle) = dev.open() else {
            continue;
        };
        let ser = handle
            .read_serial_number_string_ascii(&desc)
            .unwrap_or_default();
        if !want_serial.is_empty() && ser != want_serial {
            continue;
        }
        let _ = handle.detach_kernel_driver(IF_MONITOR);
        handle
            .claim_interface(IF_MONITOR)
            .context("claim IF2")?;
        let config = dev.active_config_descriptor().context("config")?;
        let mut found = false;
        for iface in config.interfaces() {
            for id in iface.descriptors() {
                if id.interface_number() != IF_MONITOR {
                    continue;
                }
                for ep in id.endpoint_descriptors() {
                    if ep.address() == EP2_IN
                        && ep.transfer_type() == TransferType::Interrupt
                        && ep.direction() == Direction::In
                    {
                        found = true;
                    }
                }
            }
        }
        if !found {
            bail!("no interrupt IN 0x82 on IF2");
        }
        return Ok((handle, ser));
    }
    bail!("no composite USBasp 16c0:05dc (diag) found");
}

fn cmd_monitor(serial: &str, json: bool) -> Result<()> {
    let (handle, ser) = open_composite(serial)?;
    if !json {
        println!("USBasp NG Diagnostics  serial={ser:?}  schema=DIAG v1");
    }
    let mut buf = [0u8; 8];
    let mut ep_buf = Vec::new();
    let mut snap_buf = Vec::new();
    loop {
        match handle.read_interrupt(EP2_IN, &mut buf, Duration::from_millis(1000)) {
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
                            snap_buf.push(f);
                            if snap_buf.len() == 4 {
                                if let Some(line) = reassemble_fault_snapshot(&snap_buf) {
                                    println!("         >> {line}");
                                }
                                snap_buf.clear();
                            }
                        }
                        _ => {
                            ep_buf.clear();
                            snap_buf.clear();
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
    let (handle, ser) = open_composite(serial)?;
    eprintln!("recording serial={ser:?} → {out:?} (USBDIAGv header)");
    let mut file = File::create(out).with_context(|| format!("create {out:?}"))?;
    write_header(&mut file)?;
    let mut buf = [0u8; 8];
    let mut n = 0u64;
    loop {
        match handle.read_interrupt(EP2_IN, &mut buf, Duration::from_millis(1000)) {
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
