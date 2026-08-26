//! L2 AppState reducer: frames → log lines + fault stats.

use crate::caps::CapsAdvert;
use crate::decoder::{
    format_frame, reassemble_caps, reassemble_enableprog, reassemble_fault_snapshot,
    reassemble_trace_end,
};
use crate::jsonl::{enableprog_failed, level_for, snapshot_failed, FaultStats};
use crate::protocol::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

impl Level {
    pub fn from_name(s: &str) -> Self {
        match s {
            "error" => Self::Error,
            "warning" => Self::Warn,
            "info" => Self::Info,
            _ => Self::Debug,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogEvent {
    pub host_ns: u64,
    pub text: String,
    pub level: Level,
    pub is_fault: bool,
    pub ty: u8,
    pub semantic: bool,
}

#[derive(Default)]
pub struct AppState {
    pub events: Vec<LogEvent>,
    pub stats: FaultStats,
    pub caps: Option<CapsAdvert>,
    pub hello_schema: Option<u8>,
    pub hello_profile: Option<u8>,
    pub hello_flags: Option<u8>,
    pub trace_slots: Option<u8>,
    pub trace_overflow: bool,
    pub trace_triggered: bool,
    pub trace_end_line: Option<String>,
    pub trace_kind: Option<u8>,
    pub trace_post: Option<u8>,
    pub saw_connect: bool,
    pub saw_session: bool,
    pub saw_session_end: bool,
    pub saw_reset: bool,
    pub saw_release: bool,
    pub reset_asserted: bool,
    pub session_t0: Option<u64>,
    pub sck_id: Option<u8>,
    pub sck_sw: Option<bool>,
    pub ep_tx: Option<[u8; 4]>,
    pub ep_rx: Option<[u8; 4]>,
    pub ep_fail: Option<bool>,
    pub err_check: Option<u8>,
    pub err_delay: Option<u8>,
    pub snap_rx0: Option<u8>,
    pub snap_delay: Option<u8>,
    pub snap_sw: Option<bool>,
    pub memop_kind: Option<u8>,
    pub memop_pagesize: Option<u8>,
    pub memop_pages: Vec<(u16, bool)>,
    pub memop_end_pages: Option<u8>,
    pub memop_end_ok: Option<bool>,
    pub last_flash_pages: Option<u8>,
    pub last_flash_ok: Option<bool>,
    pub last_verify_pages: Option<u8>,
    pub last_verify_ok: Option<bool>,
    pub pins_ok: Option<bool>,
    pub pins_ddr: Option<u8>,
    pub pins_pin: Option<u8>,
    ep_buf: Vec<DiagFrame>,
    snap_buf: Vec<DiagFrame>,
    caps_buf: Vec<DiagFrame>,
    trace_end_buf: Vec<DiagFrame>,
    ep_ns: Vec<u64>,
    snap_ns: Vec<u64>,
}

impl AppState {
    pub fn push_frame(&mut self, host_ns: u64, f: DiagFrame) {
        if f.ty == 0 {
            return;
        }
        self.stats.note_frame(&f);

        if f.ty == HELLO {
            self.hello_schema = Some(f.a);
            self.hello_profile = Some(f.b);
            self.hello_flags = Some(f.flags);
        }

        self.note_scene(host_ns, &f);

        let level = Level::from_name(level_for(&f));
        let is_fault = matches!(f.ty, ERROR | TRACE_OVERFLOW)
            || ((f.ty == ENABLEPROG
                || f.ty == FAULT_SNAPSHOT
                || f.ty == MEMOP
                || f.ty == ISP_PINS)
                && f.flags & EP_FAIL != 0);

        if f.ty == TRACE_OVERFLOW {
            self.trace_overflow = true;
        }
        if f.ty == TRACE_BEGIN {
            self.trace_slots = Some(f.a);
        }

        self.events.push(LogEvent {
            host_ns,
            text: format_frame(&f),
            level,
            is_fault,
            ty: f.ty,
            semantic: false,
        });

        match f.ty {
            ENABLEPROG => {
                self.snap_buf.clear();
                self.snap_ns.clear();
                self.caps_buf.clear();
                self.trace_end_buf.clear();
                self.ep_buf.push(f);
                self.ep_ns.push(host_ns);
                if self.ep_buf.len() == 4 {
                    let fail = enableprog_failed(&self.ep_buf);
                    self.stats.note_enableprog(fail);
                    self.ep_tx = Some([
                        self.ep_buf[0].a,
                        self.ep_buf[0].b,
                        self.ep_buf[1].a,
                        self.ep_buf[1].b,
                    ]);
                    self.ep_rx = Some([
                        self.ep_buf[2].a,
                        self.ep_buf[2].b,
                        self.ep_buf[3].a,
                        self.ep_buf[3].b,
                    ]);
                    self.ep_fail = Some(fail);
                    if let Some(line) = reassemble_enableprog(&self.ep_buf) {
                        self.events.push(LogEvent {
                            host_ns,
                            text: format!(">> {line}"),
                            level: if fail { Level::Error } else { Level::Info },
                            is_fault: fail,
                            ty: ENABLEPROG,
                            semantic: true,
                        });
                    }
                    self.ep_buf.clear();
                    self.ep_ns.clear();
                }
            }
            FAULT_SNAPSHOT => {
                self.ep_buf.clear();
                self.ep_ns.clear();
                self.caps_buf.clear();
                self.trace_end_buf.clear();
                self.snap_buf.push(f);
                self.snap_ns.push(host_ns);
                if self.snap_buf.len() == 4 {
                    let fail = snapshot_failed(&self.snap_buf);
                    if fail {
                        self.stats.note_snapshot_fail();
                    }
                    self.snap_rx0 = Some(self.snap_buf[3].a);
                    self.snap_delay = Some(self.snap_buf[3].b);
                    self.snap_sw = Some(self.snap_buf[0].b == TRANSPORT_SW);
                    if let Some(line) = reassemble_fault_snapshot(&self.snap_buf) {
                        self.events.push(LogEvent {
                            host_ns,
                            text: format!(">> {line}"),
                            level: if fail { Level::Error } else { Level::Info },
                            is_fault: fail,
                            ty: FAULT_SNAPSHOT,
                            semantic: true,
                        });
                    }
                    self.snap_buf.clear();
                    self.snap_ns.clear();
                }
            }
            CAPS => {
                self.ep_buf.clear();
                self.ep_ns.clear();
                self.snap_buf.clear();
                self.snap_ns.clear();
                self.trace_end_buf.clear();
                self.caps_buf.push(f);
                if self.caps_buf.len() == 4 {
                    if let Some(adv) = CapsAdvert::from_frames(&self.caps_buf) {
                        self.caps = Some(adv);
                        if let Some(line) = reassemble_caps(&self.caps_buf) {
                            self.events.push(LogEvent {
                                host_ns,
                                text: format!(">> {line}"),
                                level: Level::Info,
                                is_fault: false,
                                ty: CAPS,
                                semantic: true,
                            });
                        }
                    }
                    self.caps_buf.clear();
                }
            }
            TRACE_END => {
                self.ep_buf.clear();
                self.ep_ns.clear();
                self.snap_buf.clear();
                self.snap_ns.clear();
                self.caps_buf.clear();
                self.trace_end_buf.push(f);
                if self.trace_end_buf.len() == 4 {
                    if let Some(line) = reassemble_trace_end(&self.trace_end_buf) {
                        if self.trace_end_buf[1].flags & 0x80 != 0 {
                            self.trace_overflow = true;
                        }
                        if self.trace_end_buf[2].flags & 0x80 != 0 {
                            self.trace_triggered = true;
                        }
                        self.trace_kind = Some(self.trace_end_buf[2].a);
                        self.trace_post = Some(self.trace_end_buf[2].b);
                        self.trace_end_line = Some(line.clone());
                        self.events.push(LogEvent {
                            host_ns,
                            text: format!(">> {line}"),
                            level: if self.trace_overflow || self.trace_triggered {
                                Level::Warn
                            } else {
                                Level::Info
                            },
                            is_fault: false,
                            ty: TRACE_END,
                            semantic: true,
                        });
                    }
                    self.trace_end_buf.clear();
                }
            }
            _ => {
                self.ep_buf.clear();
                self.ep_ns.clear();
                self.snap_buf.clear();
                self.snap_ns.clear();
                self.caps_buf.clear();
                self.trace_end_buf.clear();
            }
        }

        // Cap memory for live sessions
        const MAX: usize = 5000;
        if self.events.len() > MAX {
            let drop_n = self.events.len() - MAX;
            self.events.drain(0..drop_n);
        }
    }

    pub fn ingest_capture(&mut self, cap: &crate::capture::CaptureFile) {
        for rec in &cap.records {
            if let Some(f) = rec.frame() {
                self.push_frame(rec.host_ns, f);
            }
        }
    }

    pub fn ingest_jsonl(&mut self, text: &str) -> anyhow::Result<()> {
        for (lineno, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let obj: serde_json::Value = serde_json::from_str(line)
                .map_err(|e| anyhow::anyhow!("jsonl line {}: {e}", lineno + 1))?;
            if obj.get("kind").and_then(|v| v.as_str()) != Some("frame") {
                continue;
            }
            let host_ns = obj
                .get("host_ns")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("jsonl line {}: missing host_ns", lineno + 1))?;
            let num = |k: &str| obj.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
            let f = DiagFrame {
                ty: num("type_id") as u8,
                flags: num("flags") as u8,
                timestamp: num("tick") as u16,
                a: num("a") as u8,
                b: num("b") as u8,
            };
            if f.ty != 0 {
                self.push_frame(host_ns, f);
            }
        }
        Ok(())
    }

    fn note_scene(&mut self, host_ns: u64, f: &DiagFrame) {
        match f.ty {
            HELLO | CAPS => self.saw_connect = true,
            SESSION_BEGIN => {
                self.session_t0 = Some(host_ns);
                self.saw_session = true;
                self.reset_session_ops();
            }
            SESSION_END => self.saw_session_end = true,
            RESET => {
                self.saw_reset = true;
                if f.flags & RESET_ASSERT != 0 {
                    self.reset_asserted = true;
                }
                if f.flags & RESET_RELEASE != 0 {
                    self.reset_asserted = false;
                    self.saw_release = true;
                }
            }
            SCK_CONFIG => {
                self.sck_id = Some(f.a);
                self.sck_sw = Some(f.b == TRANSPORT_SW);
            }
            ERROR => {
                self.err_check = Some(f.a);
                self.err_delay = Some(f.b);
            }
            MEMOP => {
                if f.flags & EP_START != 0 {
                    self.memop_kind = Some(f.a);
                    self.memop_pagesize = Some(f.b);
                    self.memop_pages.clear();
                    self.memop_end_pages = None;
                    self.memop_end_ok = None;
                } else if f.flags & EP_CONT != 0 {
                    let addr = (u16::from(f.a) << 8) | u16::from(f.b);
                    self.memop_pages.push((addr, f.flags & EP_FAIL == 0));
                } else if f.flags & EP_END != 0 {
                    self.memop_kind = Some(f.a);
                    self.memop_end_pages = Some(f.b);
                    self.memop_end_ok = Some(f.flags & EP_FAIL == 0);
                    let ok = f.flags & EP_FAIL == 0;
                    match f.a {
                        MEM_FLASH | MEM_EEPROM => {
                            self.last_flash_pages = Some(f.b);
                            self.last_flash_ok = Some(ok);
                        }
                        MEM_READFLASH => {
                            self.last_verify_pages = Some(f.b);
                            self.last_verify_ok = Some(ok);
                        }
                        _ => {}
                    }
                }
            }
            ISP_PINS => {
                self.pins_ok = Some(f.flags & EP_FAIL == 0);
                self.pins_ddr = Some(f.a);
                self.pins_pin = Some(f.b);
            }
            _ => {}
        }
    }

    fn reset_session_ops(&mut self) {
        self.ep_tx = None;
        self.ep_rx = None;
        self.ep_fail = None;
        self.err_check = None;
        self.err_delay = None;
        self.snap_rx0 = None;
        self.snap_delay = None;
        self.snap_sw = None;
        self.memop_kind = None;
        self.memop_pagesize = None;
        self.memop_pages.clear();
        self.memop_end_pages = None;
        self.memop_end_ok = None;
        self.last_flash_pages = None;
        self.last_flash_ok = None;
        self.last_verify_pages = None;
        self.last_verify_ok = None;
        self.pins_ok = None;
        self.pins_ddr = None;
        self.pins_pin = None;
        self.saw_reset = false;
        self.saw_release = false;
        self.reset_asserted = false;
        self.sck_id = None;
        self.sck_sw = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caps::{DiagCaps, YEL0_FCAP};
    use crate::demo;

    #[test]
    fn demo_fail_marks_faults() {
        let cap = demo::build_scenario("enableprog_fail_sw").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        assert!(st.stats.enableprog_fail >= 1);
        assert!(st.events.iter().any(|e| e.is_fault));
        let caps = st.caps.expect("CAPS in demo");
        assert_eq!(caps.firmware.0, YEL0_FCAP);
        assert!(caps.firmware.contains(DiagCaps::TIMESTAMP));
        assert!(caps.firmware.contains(DiagCaps::TRACE));
        assert!(caps.firmware.contains(DiagCaps::TRIGGER));
        assert!(!caps.firmware.contains(DiagCaps::SCK_STATS));
    }

    #[test]
    fn capabilities_yel0_scenario() {
        let cap = demo::build_scenario("capabilities_yel0").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let caps = st.caps.expect("CAPS");
        let text = caps.format_report("USBASP2 DIAG v1");
        assert!(text.contains("✓ SESSION"));
        assert!(text.contains("✓ TIMESTAMP"));
        assert!(text.contains("✓ TRACE"));
        assert!(text.contains("✓ TRIGGER"));
        assert!(text.contains("sck jumper        ✓"));
        assert!(text.contains("physical capture  ✗"));
    }

    #[test]
    fn session_pass_no_trigger() {
        let cap = demo::build_scenario("session_hw_pass").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        assert_eq!(st.trace_slots, Some(64));
        assert!(!st.trace_overflow);
        assert!(!st.trace_triggered);
        let end = st.trace_end_line.expect("TRACE_END");
        assert!(end.contains("overflow=no"));
        assert!(end.contains("triggered=no"));
    }

    #[test]
    fn enableprog_fail_triggers() {
        let cap = demo::build_scenario("enableprog_fail_sw").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        assert!(st.trace_triggered);
        let end = st.trace_end_line.expect("TRACE_END");
        assert!(end.contains("triggered=YES"));
        assert!(end.contains("ENABLEPROG_FAIL"));
    }

    #[test]
    fn overflow_scenario_keeps_loss() {
        let cap = demo::build_scenario("overflow").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        assert!(st.trace_overflow);
        assert!(st.events.iter().any(|e| e.text.contains("TRACE_OVERFLOW")));
    }
}
