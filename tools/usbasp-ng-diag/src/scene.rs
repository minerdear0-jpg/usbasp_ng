//! L3 scene: diagnosis, phase strip, dual-column rows.
//! Wire protocol unchanged. Correlate CLI dump unchanged.

use crate::correlate::{Source, TimelineEvent};
use crate::protocol::*;
use crate::state::{AppState, Level, LogEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagTone {
    Ok,
    Bad,
    Warn,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseMark {
    Idle,
    Ok,
    Fail,
    Active,
}

#[derive(Clone, Debug)]
pub struct ViewRow {
    #[allow(dead_code)]
    pub host_ns: Option<u64>,
    pub rel_ms: Option<i64>,
    pub prog: String,
    pub target: String,
    pub is_anchor: bool,
    pub is_fault: bool,
    pub level: Level,
}

pub fn is_wire_fragment(e: &LogEvent) -> bool {
    // Default watch is the scene, not EP2 chrome. `w` shows decoder frames.
    matches!(e.ty, HELLO | CAPS | TRACE_BEGIN | TRACE_END)
        || (!e.semantic && matches!(e.ty, ENABLEPROG | FAULT_SNAPSHOT))
}

/// No EP2 for this long during an open ISP session → stall (target RESET / USB hang).
pub const ISP_STALL_NS: u64 = 5_000_000_000;

pub fn diagnosis(state: &AppState) -> (DiagTone, String) {
    diagnosis_at(state, None)
}

fn line_echo_name(state: &AppState) -> &'static str {
    match state.line_bit.unwrap_or(0) {
        2 => "RST",
        3 => "MOSI",
        5 => "SCK",
        _ => "PIN",
    }
}

fn line_anomaly_note(state: &AppState) -> String {
    format!(
        "{} GPIO echo ANOMALY pin={:#04x} (MCU PINx, not connector)",
        line_echo_name(state),
        state.line_pin.unwrap_or(0)
    )
}

/// `now_ns` = host wall clock (live watch). Demos omit it (no stall).
pub fn diagnosis_at(state: &AppState, now_ns: Option<u64>) -> (DiagTone, String) {
    // Protocol outcomes outrank LINE_FAULT. PINx echo is not a connector fact.
    if state.ep_fail == Some(true) {
        let n = state.ep_attempts.len();
        let rx = state.ep_rx.unwrap_or([0xff; 4]);
        if state.ladder_all_silent() {
            return (
                DiagTone::Bad,
                format!(
                    "NO TARGET — ENABLEPROG FAIL at {n} SCK speeds, RX=FF — check ISP cable/power, not baud"
                ),
            );
        }
        if rx.iter().all(|&b| b == 0xff) {
            return (
                DiagTone::Bad,
                "TARGET SILENT — ENABLEPROG FAIL  RX=FF  ECHO 53 MISS".into(),
            );
        }
        return (
            DiagTone::Bad,
            format!(
                "BUS NOISE — ENABLEPROG FAIL  RX {:02X} {:02X}  ECHO 53 MISS",
                rx[0], rx[1]
            ),
        );
    }
    if state.pins_ok == Some(false) {
        return (
            DiagTone::Bad,
            "PINS STILL DRIVING — DDR RST/MOSI/SCK".into(),
        );
    }
    if state.flash_poll_failed {
        let addr = state.flash_poll_fail_addr.unwrap_or(0);
        return (
            DiagTone::Bad,
            format!(
                "FLASH POLL FAIL @{addr:#06x} — page never left 0xFF (or ribbon torn). MEMOP END OK does not clear this. Verify-mismatch is avrdude-only"
            ),
        );
    }
    if let Some(&(addr, false)) = state.memop_pages.iter().find(|(_, ok)| !*ok) {
        return (
            DiagTone::Bad,
            format!(
                "FLASH POLL FAIL @{addr:#06x} — page never left 0xFF (cell/lock/ISP). Verify-mismatch is avrdude-only, not EP2"
            ),
        );
    }
    if state.memop_end_ok == Some(false) {
        return (DiagTone::Bad, "MEMOP END FAIL".into());
    }
    if state.trace_overflow {
        return (
            DiagTone::Warn,
            format!("TRACE LOSS — dropped={}", state.stats.dropped),
        );
    }

    let open = state.saw_session && !state.saw_session_end;
    if open {
        if let (Some(now), Some(last)) = (now_ns, state.events.last().map(|e| e.host_ns)) {
            if now.saturating_sub(last) >= ISP_STALL_NS {
                return (
                    DiagTone::Bad,
                    "ISP STALL — no EP2 (target RESET or avrdude hung)".into(),
                );
            }
        }
    }

    if state.last_flash_ok == Some(true) && state.last_verify_ok == Some(true) {
        let w = state.last_flash_pages.unwrap_or(0);
        let r = state.last_verify_pages.unwrap_or(0);
        let pins = match state.pins_ok {
            Some(true) => "  DISC=Hi-Z",
            Some(false) => "  DISC=DRIVE",
            None => "",
        };
        if open {
            return (
                DiagTone::Warn,
                format!("VERIFY OK — waiting DISCONNECT{pins}"),
            );
        }
        if state.line_ok == Some(false) {
            return (
                DiagTone::Warn,
                format!(
                    "PASS WITH ANOMALY — FLASH WRITE {w} pages  VERIFY {r} OK{pins}  {}",
                    line_anomaly_note(state)
                ),
            );
        }
        return (
            DiagTone::Ok,
            format!("FLASH WRITE {w} pages  VERIFY {r} OK{pins}"),
        );
    }
    if state.memop_end_ok == Some(true) {
        let n = state.memop_end_pages.unwrap_or(0);
        let mem = mem_name(state.memop_kind);
        if open {
            return (
                DiagTone::Warn,
                format!("{mem} {n} pages — waiting SESSION_END"),
            );
        }
        if state.line_ok == Some(false) {
            return (
                DiagTone::Warn,
                format!(
                    "PASS WITH ANOMALY — {mem} {n} pages OK  {}",
                    line_anomaly_note(state)
                ),
            );
        }
        return (DiagTone::Ok, format!("{mem} {n} pages OK"));
    }
    if state.ep_fail == Some(false) && state.memop_kind.is_some() && state.memop_end_ok.is_none()
    {
        let addr = state
            .memop_pages
            .last()
            .map(|(a, _)| *a)
            .unwrap_or(0);
        let n = state.memop_pages.iter().filter(|(_, ok)| *ok).count();
        if open {
            return (
                DiagTone::Warn,
                format!(
                    "{} @{addr:#06x} — {n} CONT, no MEMOP END yet",
                    mem_name(state.memop_kind)
                ),
            );
        }
        return (
            DiagTone::Bad,
            format!(
                "MEMOP INCOMPLETE — {} {n} CONT pages, no MEMOP END (session closed). CONT OK ≠ write finished",
                mem_name(state.memop_kind)
            ),
        );
    }
    if state.ep_fail == Some(false) {
        if open {
            return (
                DiagTone::Warn,
                "IN SESSION — ENABLEPROG PASS, avrdude still in ISP".into(),
            );
        }
        if state.line_ok == Some(false) {
            return (
                DiagTone::Warn,
                format!(
                    "PASS WITH ANOMALY — ENABLEPROG PASS  {}",
                    line_anomaly_note(state)
                ),
            );
        }
        return (DiagTone::Ok, "ENABLEPROG PASS".into());
    }
    if state.line_ok == Some(false) {
        return (
            DiagTone::Warn,
            format!(
                "{} — no ENABLEPROG yet; not session FAIL  PHYSICAL_CAPTURE=NO",
                line_anomaly_note(state)
            ),
        );
    }
    if state.saw_session {
        return (DiagTone::Info, "—".into());
    }
    (DiagTone::Info, "—".into())
}

pub fn phases(state: &AppState) -> [(&'static str, PhaseMark); 6] {
    let connect = if state.saw_connect {
        PhaseMark::Ok
    } else {
        PhaseMark::Idle
    };
    let reset = if state.saw_reset {
        PhaseMark::Ok
    } else {
        PhaseMark::Idle
    };
    let sck = if state.sck_id.is_some() {
        PhaseMark::Ok
    } else {
        PhaseMark::Idle
    };
    let prog = match state.ep_fail {
        Some(true) => PhaseMark::Fail,
        Some(false) if state.saw_session_end => PhaseMark::Ok,
        Some(false) => PhaseMark::Active,
        None => PhaseMark::Idle,
    };
    let flash = match (state.memop_kind, state.memop_end_ok) {
        (None, _) => PhaseMark::Idle,
        (Some(_), Some(false)) => PhaseMark::Fail,
        (Some(_), Some(true)) => PhaseMark::Ok,
        (Some(_), None) => PhaseMark::Active,
    };
    let disc = match state.pins_ok {
        Some(false) => PhaseMark::Fail,
        Some(true) => PhaseMark::Ok,
        None => PhaseMark::Idle,
    };
    [
        ("CONNECT", connect),
        ("RESET", reset),
        ("SCK", sck),
        ("PROG", prog),
        ("FLASH", flash),
        ("DISC", disc),
    ]
}

pub fn programmer_rows(state: &AppState, wire: bool, faults_only: bool) -> Vec<ViewRow> {
    let t0 = state
        .events
        .first()
        .map(|e| e.host_ns)
        .or(state.session_t0);
    state
        .events
        .iter()
        .filter(|e| (wire || !is_wire_fragment(e)) && (!faults_only || e.is_fault))
        .map(|e| ViewRow {
            host_ns: Some(e.host_ns),
            rel_ms: t0.map(|t| rel_ms(e.host_ns, t)),
            prog: compact_prog(e),
            target: String::new(),
            is_anchor: false,
            is_fault: e.is_fault,
            level: e.level,
        })
        .collect()
}

pub fn dual_rows(
    events: &[TimelineEvent],
    t0: Option<u64>,
    faults_only: bool,
) -> Vec<ViewRow> {
    let t0 = t0.or_else(|| events.iter().find_map(|e| e.host_ns));
    let mut rows = Vec::new();
    let mut i = 0;
    while i < events.len() {
        let ns = events[i].host_ns;
        let mut j = i + 1;
        while j < events.len() && events[j].host_ns == ns {
            j += 1;
        }
        let group = &events[i..j];
        let progs: Vec<&TimelineEvent> = group
            .iter()
            .filter(|e| e.source == Source::Programmer)
            .collect();
        let tgts: Vec<&TimelineEvent> = group
            .iter()
            .filter(|e| e.source == Source::Target)
            .collect();
        let anchor = progs.iter().any(|e| e.kind == "RESET" && e.msg.contains("RELEASE"))
            && tgts.iter().any(|e| e.kind == "READY");
        let n = progs.len().max(tgts.len()).max(1);
        for k in 0..n {
            let p = progs.get(k);
            let t = tgts.get(k);
            let is_fault = p.is_some_and(|e| prog_fault(e)) || t.is_some_and(|e| target_fault(e));
            if faults_only && !is_fault && !anchor {
                continue;
            }
            let host = ns.or_else(|| p.and_then(|e| e.host_ns).or_else(|| t.and_then(|e| e.host_ns)));
            rows.push(ViewRow {
                host_ns: host,
                rel_ms: match (host, t0) {
                    (Some(h), Some(t)) => Some(rel_ms(h, t)),
                    _ => None,
                },
                prog: p.map(|e| compact_timeline_prog(e)).unwrap_or_default(),
                target: t.map(|e| compact_target(e)).unwrap_or_default(),
                is_anchor: anchor && k == 0,
                is_fault,
                level: if is_fault { Level::Error } else { Level::Info },
            });
        }
        i = j;
    }
    rows
}

pub fn rel_label(ms: Option<i64>) -> String {
    match ms {
        Some(v) if v < 0 => format!("-{:>5}ms", -v),
        Some(v) => format!("+{v:>5}ms"),
        None => "      —".into(),
    }
}

fn rel_ms(host_ns: u64, t0: u64) -> i64 {
    if host_ns >= t0 {
        ((host_ns - t0) / 1_000_000) as i64
    } else {
        -(((t0 - host_ns) / 1_000_000) as i64)
    }
}

fn mem_name(kind: Option<u8>) -> &'static str {
    match kind {
        Some(MEM_FLASH) => "FLASH WRITE",
        Some(MEM_EEPROM) => "EEPROM WRITE",
        Some(MEM_READFLASH) => "FLASH READ",
        _ => "MEMOP",
    }
}

fn compact_prog(e: &LogEvent) -> String {
    if e.semantic {
        return e.text.trim_start_matches('>').trim().to_string();
    }
    compact_frame(&e.text)
}

fn compact_timeline_prog(e: &TimelineEvent) -> String {
    if e.msg.contains(">> ") || e.kind != "frame" && !e.msg.contains("flags=") {
        let s = e.msg.trim_start_matches('>').trim();
        return s.to_string();
    }
    compact_frame(&e.msg)
}

fn compact_frame(text: &str) -> String {
    let s = strip_tick(text);
    let mut out = String::new();
    for tok in s.split_whitespace() {
        if tok.starts_with("flags=") || (tok.starts_with("a=") && tok.len() <= 4) || tok.starts_with("b=")
        {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(tok);
    }
    out
}

fn strip_tick(text: &str) -> &str {
    let t = text.trim_start();
    if let Some(rest) = t.strip_prefix("t=") {
        rest.trim_start_matches(|c: char| c.is_ascii_digit() || c.is_whitespace())
    } else {
        t
    }
}

fn compact_target(e: &TimelineEvent) -> String {
    let raw = e.msg.as_str();
    let rest = if raw.starts_with('@') && raw.len() >= 10 {
        raw[10..].trim_start_matches(',').trim()
    } else {
        raw
    };
    if rest.len() > 48 {
        format!("{}…", &rest[..47])
    } else {
        rest.to_string()
    }
}

fn prog_fault(e: &TimelineEvent) -> bool {
    let m = e.msg.to_ascii_uppercase();
    m.contains("FAIL") || m.contains("OVERFLOW") || e.kind == "ERROR"
}

fn target_fault(e: &TimelineEvent) -> bool {
    if e.kind == "FAULT" {
        return !e.msg.contains("kind=off");
    }
    e.msg.contains("result=FAIL")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correlate::merge_timeline;
    use crate::correlate::{parse_diag_jsonl, parse_uart_log};
    use crate::demo;

    #[test]
    fn idle_chassis_is_unlit() {
        let st = AppState::default();
        assert!(
            phases(&st).iter().all(|(_, m)| *m == PhaseMark::Idle),
            "{:?}",
            phases(&st)
        );
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Info);
        assert_eq!(line, "—");
    }

    #[test]
    fn fail_sw_is_target_silent() {
        let cap = demo::build_scenario("enableprog_fail_sw").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Bad);
        assert!(line.contains("TARGET SILENT"), "{line}");
        let ph = phases(&st);
        assert_eq!(ph[3], ("PROG", PhaseMark::Fail));
        assert_eq!(ph[4], ("FLASH", PhaseMark::Idle));
    }

    #[test]
    fn line_fault_rst_is_open() {
        let cap = demo::build_scenario("line_fault_rst").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Warn);
        assert!(line.contains("GPIO echo ANOMALY"), "{line}");
        assert!(line.contains("RST"), "{line}");
        assert!(!line.contains("LINE OPEN"), "{line}");
    }

    #[test]
    fn rst_anomaly_with_flash_is_pass_with_anomaly() {
        let cap = demo::build_scenario("pass_with_rst_anomaly").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Warn);
        assert!(line.contains("PASS WITH ANOMALY"), "{line}");
        assert!(line.contains("FLASH WRITE"), "{line}");
        assert!(line.contains("RST"), "{line}");
    }

    #[test]
    fn ladder_silent_is_no_target() {
        let cap = demo::build_scenario("enableprog_ladder_silent").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Bad);
        assert!(line.contains("NO TARGET"), "{line}");
        assert!(line.contains("cable"), "{line}");
        assert!(!line.contains("TARGET SILENT"), "{line}");
    }

    #[test]
    fn memop_poll_fail_is_page() {
        let cap = demo::build_scenario("memop_poll_fail").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Bad);
        assert!(line.contains("FLASH POLL FAIL @0x0400"), "{line}");
        assert!(line.contains("0xFF"), "{line}");
    }

    #[test]
    fn memop_flash_all_green() {
        let cap = demo::build_scenario("memop_flash").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Ok);
        assert!(line.contains("FLASH WRITE"), "{line}");
        assert!(line.contains("VERIFY"), "{line}");
        let ph = phases(&st);
        assert_eq!(ph[3], ("PROG", PhaseMark::Ok));
        assert_eq!(ph[4], ("FLASH", PhaseMark::Ok));
        assert_eq!(ph[5], ("DISC", PhaseMark::Ok));
    }

    #[test]
    fn open_session_is_not_pass() {
        let cap = demo::build_scenario("session_hw_pass").unwrap();
        let mut st = AppState::default();
        for rec in &cap.records {
            let Some(f) = rec.frame() else { continue };
            if f.ty == SESSION_END {
                continue;
            }
            st.push_frame(rec.host_ns, f);
        }
        assert!(st.saw_session && !st.saw_session_end);
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Warn, "{line}");
        assert!(line.contains("IN SESSION"), "{line}");
        assert_eq!(phases(&st)[3], ("PROG", PhaseMark::Active));
        let last = st.events.last().unwrap().host_ns;
        let (tone, line) = diagnosis_at(&st, Some(last + crate::scene::ISP_STALL_NS));
        assert_eq!(tone, DiagTone::Bad, "{line}");
        assert!(line.contains("ISP STALL"), "{line}");
    }

    #[test]
    fn flash_read_banner_is_not_write() {
        let cap = demo::build_scenario("session_hw_pass").unwrap();
        let mut st = AppState::default();
        for rec in &cap.records {
            let Some(f) = rec.frame() else { continue };
            if f.ty == SESSION_END {
                continue;
            }
            st.push_frame(rec.host_ns, f);
        }
        st.push_frame(
            99,
            DiagFrame {
                ty: MEMOP,
                flags: EP_START,
                timestamp: 0,
                a: MEM_READFLASH,
                b: 64,
            },
        );
        st.push_frame(
            100,
            DiagFrame {
                ty: MEMOP,
                flags: EP_CONT,
                timestamp: 0,
                a: 0x00,
                b: 0x00,
            },
        );
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Warn, "{line}");
        assert!(line.contains("FLASH READ"), "{line}");
        assert!(!line.contains("WRITE"), "{line}");
    }

    #[test]
    fn overflow_warns_loss() {
        let cap = demo::build_scenario("overflow").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let (tone, line) = diagnosis(&st);
        assert_eq!(tone, DiagTone::Warn);
        assert!(line.contains("TRACE LOSS"), "{line}");
    }

    #[test]
    fn dual_anchor_same_host_ns() {
        let diag = r#"
{"host_ns":1000000000,"kind":"frame","msg":"t=  100 RESET flags=0x01 ASSERT"}
{"host_ns":2000000000,"kind":"frame","msg":"t=  200 RESET flags=0x02 RELEASE"}
{"host_ns":2100000000,"kind":"frame","msg":"t=  210 SESSION_END"}
"#;
        let uart = r#"
@00000000 READY,who=canary
@00000029 APP_START,build=1
"#;
        let prog = parse_diag_jsonl(diag).unwrap();
        let uart = parse_uart_log(uart).unwrap();
        let (ev, _) = merge_timeline(prog, uart).unwrap();
        let rows = dual_rows(&ev, Some(1_000_000_000), false);
        let anchor = rows.iter().find(|r| r.is_anchor).expect("anchor row");
        assert!(anchor.prog.contains("RELEASE"), "{}", anchor.prog);
        assert!(anchor.target.contains("READY"), "{}", anchor.target);
        assert_eq!(anchor.host_ns, Some(2_000_000_000));
    }
}
