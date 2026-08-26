//! JSONL (lnav) and fault-oriented presentation.

use crate::decoder::{format_frame, reassemble_enableprog, reassemble_fault_snapshot, type_name};
use crate::protocol::*;
use serde_json::json;

pub fn host_ns_iso(host_ns: u64) -> String {
    let secs = (host_ns / 1_000_000_000) as i64;
    let usec = (host_ns % 1_000_000_000) / 1000;
    let (y, mo, d, h, mi, s) = civil_utc(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{usec:06}Z")
}

/// Civil date/time from Unix seconds (UTC). Howard Hinnant algorithm.
fn civil_utc(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let sod = ((secs % 86400) + 86400) % 86400;
    let h = (sod / 3600) as u32;
    let mi = ((sod % 3600) / 60) as u32;
    let s = (sod % 60) as u32;
    let z = secs.div_euclid(86400) + 719468;
    let era = if z >= 0 { z } else { z - 146096 }.div_euclid(146097);
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + (era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, h, mi, s)
}

pub fn level_for(f: &DiagFrame) -> &'static str {
    match f.ty {
        TRACE_OVERFLOW => "warning",
        ERROR => "error",
        ENABLEPROG | FAULT_SNAPSHOT if f.flags & EP_FAIL != 0 => "error",
        HELLO | SESSION_BEGIN | SESSION_END | RESET | SCK_CONFIG | CAPS | TRACE_BEGIN
        | TRACE_END => "info",
        _ => "debug",
    }
}

pub fn emit_jsonl_frame(host_ns: u64, f: &DiagFrame) {
    let row = json!({
        "ts": host_ns_iso(host_ns),
        "host_ns": host_ns,
        "level": level_for(f),
        "msg": format_frame(f),
        "kind": "frame",
        "type": type_name(f.ty),
        "type_id": f.ty,
        "flags": f.flags,
        "tick": f.timestamp,
        "a": f.a,
        "b": f.b,
    });
    println!("{row}");
}

pub fn emit_jsonl_semantic(host_ns: u64, kind: &str, level: &str, msg: &str) {
    let row = json!({
        "ts": host_ns_iso(host_ns),
        "host_ns": host_ns,
        "level": level,
        "msg": msg,
        "kind": kind,
    });
    println!("{row}");
}

#[derive(Default)]
pub struct FaultStats {
    pub enableprog_pass: u32,
    pub enableprog_fail: u32,
    pub snapshot_fail: u32,
    pub errors: u32,
    pub overflows: u32,
    pub dropped: u32,
}

impl FaultStats {
    pub fn note_frame(&mut self, f: &DiagFrame) {
        match f.ty {
            ERROR => self.errors += 1,
            TRACE_OVERFLOW => {
                self.overflows += 1;
                self.dropped = self.dropped.saturating_add(u32::from(f.a));
            }
            _ => {}
        }
    }

    pub fn note_enableprog(&mut self, fail: bool) {
        if fail {
            self.enableprog_fail += 1;
        } else {
            self.enableprog_pass += 1;
        }
    }

    pub fn note_snapshot_fail(&mut self) {
        self.snapshot_fail += 1;
    }

    pub fn print_summary(&self) {
        println!("=== FAULT SUMMARY ===");
        println!(
            "ENABLEPROG  PASS={}  FAIL={}",
            self.enableprog_pass, self.enableprog_fail
        );
        println!("FAULT_SNAPSHOT FAIL={}", self.snapshot_fail);
        println!("ERROR notes={}", self.errors);
        println!(
            "TRACE_OVERFLOW count={}  dropped_sum={}",
            self.overflows, self.dropped
        );
        if self.enableprog_fail == 0
            && self.snapshot_fail == 0
            && self.errors == 0
            && self.overflows == 0
        {
            println!("(no faults)");
        }
    }
}

pub fn enableprog_failed(frames: &[DiagFrame]) -> bool {
    reassemble_enableprog(frames).is_some_and(|l| l.contains("FAIL"))
}

pub fn snapshot_failed(frames: &[DiagFrame]) -> bool {
    reassemble_fault_snapshot(frames).is_some_and(|l| l.contains("FAIL"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_looks_sane() {
        // 2023-11-14T22:13:20Z approx for 1.7e18 ns
        let s = host_ns_iso(1_700_000_000_000_000_000);
        assert!(s.starts_with("2023-"), "{s}");
        assert!(s.contains('T') && s.ends_with('Z'));
    }
}
