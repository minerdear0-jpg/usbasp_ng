//! Dual-truth timeline: PROGRAMMER (EP2) ↔ TARGET (oracle UART).
//!
//! Two time bases:
//! - **event_order** — UART log has no host receive stamps. READY is glued to
//!   RESET RELEASE. Order only; no ±doubt.
//! - **host_observer** — UART lines prefixed with host_ns (NTP/Cristian wire
//!   is the host). READY sits at its receive time. `dt_ready_host_ns` is the
//!   measured RELEASE→READY interval on that common clock; `doubt_ns` is
//!   |dt|/2 (RFC 5905-style half round-trip of the observation).
//!
//! Target `tcnt1` on READY is the canary's Timer1 (sub-ms); it does not by
//! itself sync to the programmer crystal — the host stamps do.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Source {
    Programmer,
    Target,
}

#[derive(Clone, Debug, Serialize)]
pub struct TimelineEvent {
    pub source: Source,
    pub host_ns: Option<u64>,
    /// Firmware Timer1 wire16 when known (programmer only).
    pub t_fw: Option<u16>,
    /// Target ms since boot (`@TTTTTTTT`).
    pub t_ms: Option<u32>,
    pub kind: String,
    pub msg: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ProgRow {
    host_ns: u64,
    t_fw: Option<u16>,
    kind: String,
    msg: String,
}

#[derive(Clone, Debug)]
pub(crate) struct UartRow {
    t_ms: u32,
    kind: String,
    raw: String,
    host_ns: Option<u64>,
    tcnt1: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeBasis {
    EventOrder,
    HostObserver,
}

#[derive(Clone, Debug, Serialize)]
pub struct ClockSync {
    pub time_basis: TimeBasis,
    /// host_ns(READY) − host_ns(RELEASE) when both known (host observer).
    pub dt_ready_host_ns: Option<i64>,
    /// Cristian bound |dt|/2; None if event_order (unbounded).
    pub doubt_ns: Option<u64>,
    pub tcnt1_ready: Option<u16>,
}

fn parse_fw_t(msg: &str) -> Option<u16> {
    // format_frame: "t= 5885 HELLO ..." or "t=5885 "
    let idx = msg.find("t=")?;
    let rest = msg[idx + 2..].trim_start();
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if num.is_empty() {
        None
    } else {
        num.parse().ok()
    }
}

fn kind_from_diag_msg(msg: &str) -> String {
    // "... t=123 HELLO ..." or semantic lines
    for tok in ["ENABLEPROG", "MEMOP", "ISP_PINS", "LINE_FAULT", "RESET", "SESSION_END", "SESSION_BEGIN", "TRACE_END", "TRACE_BEGIN", "SCK_CONFIG", "HELLO", "CAPS", "TRACE_OVERFLOW", "FAULT_SNAPSHOT", "ERROR"] {
        if msg.contains(tok) {
            return tok.to_string();
        }
    }
    "FRAME".to_string()
}

pub(crate) fn parse_diag_jsonl(text: &str) -> Result<Vec<ProgRow>> {
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let obj: serde_json::Value =
            serde_json::from_str(line).with_context(|| format!("diag jsonl line {}", lineno + 1))?;
        let host_ns = obj
            .get("host_ns")
            .and_then(|v| v.as_u64())
            .context("diag row missing host_ns")?;
        let msg = obj
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let kind = obj
            .get("kind")
            .and_then(|v| v.as_str())
            .filter(|k| *k != "frame" && *k != "semantic")
            .map(|s| s.to_string())
            .unwrap_or_else(|| kind_from_diag_msg(&msg));
        out.push(ProgRow {
            host_ns,
            t_fw: parse_fw_t(&msg),
            kind,
            msg,
        });
    }
    Ok(out)
}

pub(crate) fn parse_uart_log(text: &str) -> Result<Vec<UartRow>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (host_ns, at) = split_uart_host_prefix(line);
        if !at.starts_with('@') || at.len() < 10 {
            continue;
        }
        let t_ms: u32 = at[1..9]
            .parse()
            .with_context(|| format!("bad @ms in {line}"))?;
        let rest = at[9..].trim_start_matches(',').trim();
        let kind = rest
            .split(',')
            .next()
            .unwrap_or("UART")
            .trim()
            .to_string();
        if kind == "HEARTBEAT" {
            continue;
        }
        out.push(UartRow {
            t_ms,
            kind,
            raw: at.to_string(),
            host_ns,
            tcnt1: parse_kv_hex16(at, "tcnt1"),
        });
    }
    Ok(out)
}

fn split_uart_host_prefix(line: &str) -> (Option<u64>, &str) {
    let Some((a, b)) = line.split_once(char::is_whitespace) else {
        return (None, line);
    };
    if !b.starts_with('@') {
        return (None, line);
    }
    match a.parse::<u64>() {
        Ok(ns) => (Some(ns), b),
        Err(_) => (None, line),
    }
}

fn parse_kv_hex16(line: &str, key: &str) -> Option<u16> {
    let pat = format!("{key}=");
    let idx = line.find(&pat)?;
    let rest = &line[idx + pat.len()..];
    let tok: String = rest
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    u16::from_str_radix(&tok, 16).ok()
}

fn find_release_host_ns(prog: &[ProgRow]) -> Option<u64> {
    prog.iter()
        .rev()
        .find(|r| r.kind == "RESET" && r.msg.contains("RELEASE"))
        .map(|r| r.host_ns)
}

fn find_target_t0(uart: &[UartRow]) -> Option<&UartRow> {
    uart.iter()
        .find(|r| r.kind == "READY" || r.kind == "APP_START")
}

fn push_target(
    events: &mut Vec<TimelineEvent>,
    uart: Vec<UartRow>,
    origin_ns: Option<u64>,
    t0_ms: Option<u32>,
) {
    if let (Some(origin), Some(t0)) = (origin_ns, t0_ms) {
        for u in uart {
            let dt_ms = u.t_ms as i64 - t0 as i64;
            let host_ns = if dt_ms >= 0 {
                Some(origin.saturating_add((dt_ms as u64).saturating_mul(1_000_000)))
            } else {
                Some(origin.saturating_sub(((-dt_ms) as u64).saturating_mul(1_000_000)))
            };
            events.push(TimelineEvent {
                source: Source::Target,
                host_ns,
                t_fw: None,
                t_ms: Some(u.t_ms),
                kind: u.kind,
                msg: u.raw,
            });
        }
    } else {
        for u in uart {
            events.push(TimelineEvent {
                source: Source::Target,
                host_ns: u.host_ns,
                t_fw: None,
                t_ms: Some(u.t_ms),
                kind: u.kind,
                msg: u.raw,
            });
        }
    }
}

/// Merge programmer EP2 JSONL + oracle UART log into one host_ns timeline.
pub(crate) fn merge_timeline(
    prog: Vec<ProgRow>,
    uart: Vec<UartRow>,
) -> Result<(Vec<TimelineEvent>, ClockSync)> {
    let release_ns = find_release_host_ns(&prog);
    let t0 = find_target_t0(&uart);
    let t0_ms = t0.map(|r| r.t_ms);
    let ready_host = t0.and_then(|r| r.host_ns);
    let tcnt1_ready = t0.and_then(|r| r.tcnt1);

    let mut events: Vec<TimelineEvent> = prog
        .into_iter()
        .map(|r| TimelineEvent {
            source: Source::Programmer,
            host_ns: Some(r.host_ns),
            t_fw: r.t_fw,
            t_ms: None,
            kind: r.kind,
            msg: r.msg,
        })
        .collect();

    let sync = if let (Some(rel), Some(rdy)) = (release_ns, ready_host) {
        let dt = rdy as i64 - rel as i64;
        let doubt = dt.unsigned_abs() / 2;
        push_target(&mut events, uart, Some(rdy), t0_ms);
        ClockSync {
            time_basis: TimeBasis::HostObserver,
            dt_ready_host_ns: Some(dt),
            doubt_ns: Some(doubt),
            tcnt1_ready,
        }
    } else {
        push_target(&mut events, uart, release_ns, t0_ms);
        ClockSync {
            time_basis: TimeBasis::EventOrder,
            dt_ready_host_ns: None,
            doubt_ns: None,
            tcnt1_ready,
        }
    };

    events.sort_by(|a, b| match (a.host_ns, b.host_ns) {
        (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.kind.cmp(&b.kind)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.t_ms.cmp(&b.t_ms),
    });
    Ok((events, sync))
}

pub fn correlate_files(diag_jsonl: &Path, uart_log: &Path) -> Result<(Vec<TimelineEvent>, ClockSync)> {
    let diag = fs::read_to_string(diag_jsonl)
        .with_context(|| format!("read {}", diag_jsonl.display()))?;
    let uart =
        fs::read_to_string(uart_log).with_context(|| format!("read {}", uart_log.display()))?;
    let prog = parse_diag_jsonl(&diag)?;
    let uart = parse_uart_log(&uart)?;
    if prog.is_empty() {
        bail!("no programmer rows in {}", diag_jsonl.display());
    }
    merge_timeline(prog, uart)
}

pub fn merge_programmer_and_uart(
    prog: Vec<(u64, String, String)>,
    uart_text: &str,
) -> Result<Vec<TimelineEvent>> {
    let prog: Vec<ProgRow> = prog
        .into_iter()
        .map(|(host_ns, kind, msg)| ProgRow {
            t_fw: parse_fw_t(&msg),
            host_ns,
            kind,
            msg,
        })
        .collect();
    let uart = parse_uart_log(uart_text)?;
    Ok(merge_timeline(prog, uart)?.0)
}

pub fn emit_human(events: &[TimelineEvent], sync: &ClockSync) {
    match sync.time_basis {
        TimeBasis::EventOrder => {
            println!("# time_basis=event_order  (no host stamps on UART; READY glued to RELEASE)");
            println!("# doubt_ns=unbounded  — not an absolute delay");
        }
        TimeBasis::HostObserver => {
            println!(
                "# time_basis=host_observer  dt_ready_host_ns={}  doubt_ns={}  (Cristian |dt|/2)",
                sync.dt_ready_host_ns.unwrap_or(0),
                sync.doubt_ns.unwrap_or(0)
            );
            if let Some(t) = sync.tcnt1_ready {
                println!("# tcnt1_ready={t:04x}");
            }
        }
    }
    println!("# dual-truth  source=PROGRAMMER|TARGET  sync=RELEASE↔READY");
    for e in events {
        let src = match e.source {
            Source::Programmer => "PROGRAMMER",
            Source::Target => "TARGET    ",
        };
        let host = e
            .host_ns
            .map(|n| format!("{n}"))
            .unwrap_or_else(|| "-".into());
        let tfw = e
            .t_fw
            .map(|t| format!("t_fw={t}"))
            .unwrap_or_else(|| "t_fw=-".into());
        let tms = e
            .t_ms
            .map(|t| format!("t_ms={t}"))
            .unwrap_or_else(|| "t_ms=-".into());
        println!("{host}  {src}  {tfw}  {tms}  {}  {}", e.kind, e.msg);
    }
}

pub fn emit_jsonl(events: &[TimelineEvent], sync: &ClockSync) {
    println!(
        "{}",
        json!({
            "kind": "meta",
            "time_basis": sync.time_basis,
            "dt_ready_host_ns": sync.dt_ready_host_ns,
            "doubt_ns": sync.doubt_ns,
            "tcnt1_ready": sync.tcnt1_ready,
            "sync": "RESET_RELEASE_to_READY",
        })
    );
    for e in events {
        let row = json!({
            "source": e.source,
            "host_ns": e.host_ns,
            "t_fw": e.t_fw,
            "t_ms": e.t_ms,
            "kind": e.kind,
            "msg": e.msg,
        });
        println!("{row}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_maps_ready_near_release() {
        let diag = r#"
{"host_ns":1000000000,"kind":"frame","msg":"t=  100 RESET flags=0x01 ASSERT"}
{"host_ns":2000000000,"kind":"frame","msg":"t=  200 RESET flags=0x02 RELEASE"}
{"host_ns":2100000000,"kind":"frame","msg":"t=  210 SESSION_END"}
"#;
        let uart = r#"
@00000000 READY,who=canary
@00000029 APP_START,build=1
@00000391 FLASH_CRC,crc=ECDB
"#;
        let prog = parse_diag_jsonl(diag).unwrap();
        let uart = parse_uart_log(uart).unwrap();
        let (ev, sync) = merge_timeline(prog, uart).unwrap();
        assert_eq!(sync.time_basis, TimeBasis::EventOrder);
        assert_eq!(sync.doubt_ns, None);
        let ready = ev.iter().find(|e| e.kind == "READY").unwrap();
        assert_eq!(ready.source, Source::Target);
        assert_eq!(ready.host_ns, Some(2_000_000_000));
        let crc = ev.iter().find(|e| e.kind == "FLASH_CRC").unwrap();
        assert_eq!(crc.host_ns, Some(2_000_000_000 + 391 * 1_000_000));
    }

    #[test]
    fn host_stamps_give_cristian_doubt() {
        let diag = r#"
{"host_ns":2000000000,"kind":"frame","msg":"t=  200 RESET flags=0x02 RELEASE"}
"#;
        let uart = r#"
2029000000 @00000000 READY,who=canary,tcnt1=0042
2029000000 @00000029 APP_START
"#;
        let prog = parse_diag_jsonl(diag).unwrap();
        let uart = parse_uart_log(uart).unwrap();
        let (ev, sync) = merge_timeline(prog, uart).unwrap();
        assert_eq!(sync.time_basis, TimeBasis::HostObserver);
        assert_eq!(sync.dt_ready_host_ns, Some(29_000_000));
        assert_eq!(sync.doubt_ns, Some(14_500_000));
        assert_eq!(sync.tcnt1_ready, Some(0x42));
        let ready = ev.iter().find(|e| e.kind == "READY").unwrap();
        assert_eq!(ready.host_ns, Some(2_029_000_000));
        let app = ev.iter().find(|e| e.kind == "APP_START").unwrap();
        assert_eq!(app.host_ns, Some(2_029_000_000 + 29 * 1_000_000));
    }
}
