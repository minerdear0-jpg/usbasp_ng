//! Coherent instrument dump (probe Analyze / snapshot-now).
//! Assembled on the host from EP2 AppState — no extra GET opcodes, wire unchanged.

use crate::scene::{diagnosis, DiagTone};
use crate::state::AppState;
use crate::version;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub diagplane: String,
    pub protocol: String,
    pub source: String,
    pub complete: bool,
    pub diagnosis: String,
    pub tone: &'static str,
    pub connect: bool,
    pub session: bool,
    pub session_end: bool,
    pub reset_asserted: bool,
    pub saw_release: bool,
    pub sck_id: Option<u8>,
    pub sck_sw: Option<bool>,
    pub enableprog_fail: Option<bool>,
    pub ep_tx: Option<String>,
    pub ep_rx: Option<String>,
    pub trace_slots: Option<u8>,
    pub trace_overflow: bool,
    pub trace_triggered: bool,
    pub trace_valid: Option<u16>,
    pub trace_write_index: Option<u16>,
    pub trace_kind: Option<u8>,
    pub trace_post: Option<u8>,
    pub memop_kind: Option<u8>,
    pub memop_end_pages: Option<u8>,
    pub memop_end_ok: Option<bool>,
    pub last_flash_pages: Option<u8>,
    pub last_flash_ok: Option<bool>,
    pub last_verify_pages: Option<u8>,
    pub last_verify_ok: Option<bool>,
    pub pins_ok: Option<bool>,
    pub pins_ddr: Option<u8>,
    pub pins_pin: Option<u8>,
    pub frames: usize,
    pub dropped: u32,
    pub caps: Option<String>,
}

fn hex4(b: [u8; 4]) -> String {
    format!("{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3])
}

fn tone_name(t: DiagTone) -> &'static str {
    match t {
        DiagTone::Ok => "ok",
        DiagTone::Bad => "bad",
        DiagTone::Warn => "warn",
        DiagTone::Info => "info",
    }
}

pub fn from_state(source: &str, state: &AppState, complete: bool) -> Snapshot {
    let (tone, line) = diagnosis(state);
    Snapshot {
        diagplane: version::DIAGPLANE_VERSION.to_string(),
        protocol: version::PROTOCOL_VERSION_STR.to_string(),
        source: source.to_string(),
        complete,
        diagnosis: line,
        tone: tone_name(tone),
        connect: state.saw_connect,
        session: state.saw_session,
        session_end: state.saw_session_end,
        reset_asserted: state.reset_asserted,
        saw_release: state.saw_release,
        sck_id: state.sck_id,
        sck_sw: state.sck_sw,
        enableprog_fail: state.ep_fail,
        ep_tx: state.ep_tx.map(hex4),
        ep_rx: state.ep_rx.map(hex4),
        trace_slots: state.trace_slots,
        trace_overflow: state.trace_overflow,
        trace_triggered: state.trace_triggered,
        trace_valid: state.trace_valid,
        trace_write_index: state.trace_write_index,
        trace_kind: state.trace_kind,
        trace_post: state.trace_post,
        memop_kind: state.memop_kind,
        memop_end_pages: state.memop_end_pages,
        memop_end_ok: state.memop_end_ok,
        last_flash_pages: state.last_flash_pages,
        last_flash_ok: state.last_flash_ok,
        last_verify_pages: state.last_verify_pages,
        last_verify_ok: state.last_verify_ok,
        pins_ok: state.pins_ok,
        pins_ddr: state.pins_ddr,
        pins_pin: state.pins_pin,
        frames: state.events.len(),
        dropped: state.stats.dropped,
        caps: state.caps.as_ref().map(|c| c.summary_line()),
    }
}

impl Snapshot {
    pub fn emit_human(&self) {
        println!("{}  source={}", version::banner_short(), self.source);
        println!(
            "diagnosis  [{}] {}",
            self.tone.to_uppercase(),
            self.diagnosis
        );
        println!();
        println!("USB/ISP");
        println!(
            "  connect={}  session={}  session_end={}  complete={}",
            yn(self.connect),
            yn(self.session),
            yn(self.session_end),
            yn(self.complete)
        );
        let sck = match (self.sck_id, self.sck_sw) {
            (Some(id), Some(true)) => format!("SW id={id}"),
            (Some(id), _) => format!("HW id={id}"),
            _ => "—".into(),
        };
        println!(
            "  reset_asserted={}  release={}  sck={sck}",
            yn(self.reset_asserted),
            yn(self.saw_release)
        );
        if let Some(c) = &self.caps {
            println!("  {c}");
        }
        println!();
        println!("ENABLEPROG");
        match self.enableprog_fail {
            Some(true) => println!("  result=FAIL"),
            Some(false) => println!("  result=PASS"),
            None => println!("  result=—"),
        }
        println!(
            "  tx={}  rx={}",
            self.ep_tx.as_deref().unwrap_or("—"),
            self.ep_rx.as_deref().unwrap_or("—")
        );
        println!();
        println!("TRACE");
        println!(
            "  slots={}  overflow={}  triggered={}  valid={}  write_index={}",
            self.trace_slots.map(|n| n.to_string()).unwrap_or("—".into()),
            yn(self.trace_overflow),
            yn(self.trace_triggered),
            self.trace_valid.map(|n| n.to_string()).unwrap_or("—".into()),
            self.trace_write_index
                .map(|n| n.to_string())
                .unwrap_or("—".into())
        );
        println!();
        println!("MEMOP");
        println!(
            "  end_pages={}  end_ok={}  flash_pages={}  flash_ok={}  verify_pages={}  verify_ok={}",
            opt_u8(self.memop_end_pages),
            opt_bool(self.memop_end_ok),
            opt_u8(self.last_flash_pages),
            opt_bool(self.last_flash_ok),
            opt_u8(self.last_verify_pages),
            opt_bool(self.last_verify_ok)
        );
        println!();
        println!("ISP_PINS");
        let pins = match self.pins_ok {
            Some(true) => "Hi-Z",
            Some(false) => "DRIVE",
            None => "—",
        };
        println!(
            "  after_disconnect={pins}  ddr={}  pin={}",
            opt_hex(self.pins_ddr),
            opt_hex(self.pins_pin)
        );
        println!();
        println!("frames={}  dropped={}", self.frames, self.dropped);
    }

    pub fn emit_json(&self) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(std::io::stdout(), self)?;
        println!();
        Ok(())
    }
}

fn yn(v: bool) -> &'static str {
    if v {
        "yes"
    } else {
        "no"
    }
}

fn opt_u8(v: Option<u8>) -> String {
    v.map(|n| n.to_string()).unwrap_or_else(|| "—".into())
}

fn opt_bool(v: Option<bool>) -> String {
    match v {
        Some(true) => "yes".into(),
        Some(false) => "no".into(),
        None => "—".into(),
    }
}

fn opt_hex(v: Option<u8>) -> String {
    v.map(|n| format!("0x{n:02x}")).unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;
    use crate::state::AppState;

    #[test]
    fn fail_sw_snapshot_is_target_silent() {
        let cap = demo::build_scenario("enableprog_fail_sw").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let s = from_state("demo:enableprog_fail_sw", &st, true);
        assert_eq!(s.tone, "bad");
        assert!(s.diagnosis.contains("TARGET SILENT"));
        assert_eq!(s.enableprog_fail, Some(true));
        assert!(s.trace_triggered);
        assert!(s.complete);
        assert!(s.session_end);
    }

    #[test]
    fn memop_flash_snapshot_is_ok() {
        let cap = demo::build_scenario("memop_flash").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let s = from_state("demo:memop_flash", &st, true);
        assert_eq!(s.tone, "ok");
        assert_eq!(s.last_flash_ok, Some(true));
        assert_eq!(s.last_verify_ok, Some(true));
        assert_eq!(s.pins_ok, Some(true));
        assert!(!s.trace_overflow);
    }

    #[test]
    fn idle_snapshot_is_unlit() {
        let st = AppState::default();
        let s = from_state("idle", &st, false);
        assert_eq!(s.tone, "info");
        assert_eq!(s.diagnosis, "—");
        assert!(!s.complete);
        assert_eq!(s.frames, 0);
    }
}
