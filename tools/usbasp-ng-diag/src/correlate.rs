//! Dual-truth timeline: PROGRAMMER (EP2) ↔ TARGET (oracle UART).
//!
//! Sync (no FX2 yet): host_ns(RESET RELEASE) ≈ target READY/APP_START @ t_ms=0+.
//! Map target `@TTTTTTTT` → host_ns ≈ release_host_ns + (t_ms - t0_ms) * 1e6.

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
    for tok in ["ENABLEPROG", "MEMOP", "ISP_PINS", "RESET", "SESSION_END", "SESSION_BEGIN", "TRACE_END", "TRACE_BEGIN", "SCK_CONFIG", "HELLO", "CAPS", "TRACE_OVERFLOW", "FAULT_SNAPSHOT", "ERROR"] {
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
        if !line.starts_with('@') || line.len() < 10 {
            continue;
        }
        let t_ms: u32 = line[1..9]
            .parse()
            .with_context(|| format!("bad @ms in {line}"))?;
        let rest = line[9..].trim_start_matches(',').trim();
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
            raw: line.to_string(),
        });
    }
    Ok(out)
}

fn find_release_host_ns(prog: &[ProgRow]) -> Option<u64> {
    prog.iter()
        .rev()
        .find(|r| r.kind == "RESET" && r.msg.contains("RELEASE"))
        .map(|r| r.host_ns)
}

fn find_target_t0_ms(uart: &[UartRow]) -> Option<u32> {
    uart.iter()
        .find(|r| r.kind == "READY" || r.kind == "APP_START")
        .map(|r| r.t_ms)
}

/// Merge programmer EP2 JSONL + oracle UART log into one host_ns timeline.
pub(crate) fn merge_timeline(prog: Vec<ProgRow>, uart: Vec<UartRow>) -> Result<Vec<TimelineEvent>> {
    let release_ns = find_release_host_ns(&prog);
    let t0_ms = find_target_t0_ms(&uart);

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

    if let (Some(rel), Some(t0)) = (release_ns, t0_ms) {
        for u in uart {
            let dt_ms = u.t_ms as i64 - t0 as i64;
            let host_ns = if dt_ms >= 0 {
                Some(rel.saturating_add((dt_ms as u64).saturating_mul(1_000_000)))
            } else {
                Some(rel.saturating_sub(((-dt_ms) as u64).saturating_mul(1_000_000)))
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
        // No sync — still emit target rows without host_ns.
        for u in uart {
            events.push(TimelineEvent {
                source: Source::Target,
                host_ns: None,
                t_fw: None,
                t_ms: Some(u.t_ms),
                kind: u.kind,
                msg: u.raw,
            });
        }
    }

    events.sort_by(|a, b| match (a.host_ns, b.host_ns) {
        (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.kind.cmp(&b.kind)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.t_ms.cmp(&b.t_ms),
    });
    Ok(events)
}

pub fn correlate_files(diag_jsonl: &Path, uart_log: &Path) -> Result<Vec<TimelineEvent>> {
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

/// Merge already-decoded programmer events with a UART log (TUI; CLI dump unchanged).
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
    merge_timeline(prog, uart)
}

pub fn emit_human(events: &[TimelineEvent]) {
    println!("# dual-truth timeline  source=PROGRAMMER|TARGET");
    println!("# sync: RESET RELEASE host_ns ↔ READY/APP_START target ms");
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

pub fn emit_jsonl(events: &[TimelineEvent]) {
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
        let ev = merge_timeline(prog, uart).unwrap();
        let ready = ev.iter().find(|e| e.kind == "READY").unwrap();
        assert_eq!(ready.source, Source::Target);
        assert_eq!(ready.host_ns, Some(2_000_000_000));
        let crc = ev.iter().find(|e| e.kind == "FLASH_CRC").unwrap();
        assert_eq!(crc.host_ns, Some(2_000_000_000 + 391 * 1_000_000));
    }
}
