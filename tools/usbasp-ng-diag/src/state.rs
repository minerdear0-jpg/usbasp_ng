//! L2 AppState reducer: frames → log lines + fault stats.

use crate::decoder::{format_frame, reassemble_enableprog, reassemble_fault_snapshot};
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
}

#[derive(Default)]
pub struct AppState {
    pub events: Vec<LogEvent>,
    pub stats: FaultStats,
    ep_buf: Vec<DiagFrame>,
    snap_buf: Vec<DiagFrame>,
    ep_ns: Vec<u64>,
    snap_ns: Vec<u64>,
}

impl AppState {
    pub fn push_frame(&mut self, host_ns: u64, f: DiagFrame) {
        if f.ty == 0 {
            return;
        }
        self.stats.note_frame(&f);

        let level = Level::from_name(level_for(&f));
        let is_fault = matches!(f.ty, ERROR | TRACE_OVERFLOW)
            || ((f.ty == ENABLEPROG || f.ty == FAULT_SNAPSHOT) && f.flags & EP_FAIL != 0);

        self.events.push(LogEvent {
            host_ns,
            text: format_frame(&f),
            level,
            is_fault,
        });

        match f.ty {
            ENABLEPROG => {
                self.snap_buf.clear();
                self.snap_ns.clear();
                self.ep_buf.push(f);
                self.ep_ns.push(host_ns);
                if self.ep_buf.len() == 4 {
                    let fail = enableprog_failed(&self.ep_buf);
                    self.stats.note_enableprog(fail);
                    if let Some(line) = reassemble_enableprog(&self.ep_buf) {
                        self.events.push(LogEvent {
                            host_ns,
                            text: format!(">> {line}"),
                            level: if fail { Level::Error } else { Level::Info },
                            is_fault: fail,
                        });
                    }
                    self.ep_buf.clear();
                    self.ep_ns.clear();
                }
            }
            FAULT_SNAPSHOT => {
                self.ep_buf.clear();
                self.ep_ns.clear();
                self.snap_buf.push(f);
                self.snap_ns.push(host_ns);
                if self.snap_buf.len() == 4 {
                    let fail = snapshot_failed(&self.snap_buf);
                    if fail {
                        self.stats.note_snapshot_fail();
                    }
                    if let Some(line) = reassemble_fault_snapshot(&self.snap_buf) {
                        self.events.push(LogEvent {
                            host_ns,
                            text: format!(">> {line}"),
                            level: if fail { Level::Error } else { Level::Info },
                            is_fault: fail,
                        });
                    }
                    self.snap_buf.clear();
                    self.snap_ns.clear();
                }
            }
            _ => {
                self.ep_buf.clear();
                self.ep_ns.clear();
                self.snap_buf.clear();
                self.snap_ns.clear();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;

    #[test]
    fn demo_fail_marks_faults() {
        let cap = demo::build_scenario("enableprog_fail_sw").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        assert!(st.stats.enableprog_fail >= 1);
        assert!(st.events.iter().any(|e| e.is_fault));
    }
}
