use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use rusb::{Direction, TransferType};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod decoder;
mod protocol;

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
    /// Decode a lab capture (host_ns u64 LE + 8-byte report per record)
    Decode {
        file: PathBuf,
    },
    /// Live EP2 → stdout (detach IF2 briefly)
    Monitor {
        /// Match USB iSerial (e.g. YEL0); empty = first composite
        #[arg(default_value = "")]
        serial: String,
        #[arg(long)]
        json: bool,
    },
    /// Raw record: host_ns + 8-byte reports
    Record {
        #[arg(default_value = "")]
        serial: String,
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Decode { file } => cmd_decode(&file),
        Cmd::Monitor { serial, json } => cmd_monitor(&serial, json),
        Cmd::Record { serial, out } => cmd_record(&serial, &out),
    }
}

fn cmd_decode(path: &PathBuf) -> Result<()> {
    let mut blob = Vec::new();
    File::open(path)
        .with_context(|| format!("open {path:?}"))?
        .read_to_end(&mut blob)?;
    const REC: usize = 8 + 8;
    if blob.len() % REC != 0 {
        eprintln!("warning: trailing {} bytes", blob.len() % REC);
    }
    let mut ep_buf = Vec::new();
    let mut snap_buf = Vec::new();
    for chunk in blob.chunks_exact(REC) {
        let host_ns = u64::from_le_bytes(chunk[0..8].try_into().unwrap());
        let report = &chunk[8..16];
        let Some(f) = DiagFrame::from_report(report) else {
            continue;
        };
        println!("{host_ns}  {}", format_frame(&f));
        match f.ty {
            ENABLEPROG => {
                snap_buf.clear();
                ep_buf.push(f);
                if ep_buf.len() == 4 {
                    if let Some(line) = reassemble_enableprog(&ep_buf) {
                        println!("{:20}>> {line}", "");
                    }
                    ep_buf.clear();
                }
            }
            FAULT_SNAPSHOT => {
                ep_buf.clear();
                snap_buf.push(f);
                if snap_buf.len() == 4 {
                    if let Some(line) = reassemble_fault_snapshot(&snap_buf) {
                        println!("{:20}>> {line}", "");
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
            // classic bcdDevice 2.03 — no diag EP2
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
        // Claim monitor interface
        let _ = handle.detach_kernel_driver(IF_MONITOR);
        handle
            .claim_interface(IF_MONITOR)
            .context("claim IF2")?;
        // Sanity: find EP 0x82
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
    eprintln!("recording serial={ser:?} → {out:?}");
    let mut file = File::create(out).with_context(|| format!("create {out:?}"))?;
    let mut buf = [0u8; 8];
    let mut n = 0u64;
    loop {
        match handle.read_interrupt(EP2_IN, &mut buf, Duration::from_millis(1000)) {
            Ok(k) if k >= 6 => {
                let ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                file.write_all(&ns.to_le_bytes())?;
                let mut report = [0u8; 8];
                report[..k.min(8)].copy_from_slice(&buf[..k.min(8)]);
                file.write_all(&report)?;
                file.flush()?;
                n += 1;
                if let Some(f) = DiagFrame::from_report(&report) {
                    eprintln!("{n:5} {}", format_frame(&f));
                }
            }
            Ok(_) | Err(rusb::Error::Timeout) => {}
            Err(e) => bail!("USB read: {e}"),
        }
    }
}
