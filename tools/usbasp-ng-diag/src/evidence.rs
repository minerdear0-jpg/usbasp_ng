//! Host Evidence Record v1 — frozen diagnostic session, not a new EP2 type.
//!
//! Firmware reports observations. This module records expected vs observed,
//! verdict, and confidence. It never claims physical_capture from EP2.

use crate::scene::{diagnosis, DiagTone};
use crate::state::AppState;
use crate::version;
use serde::Serialize;

pub const EVIDENCE_SCHEMA: u8 = 1;

#[derive(Clone, Debug, Serialize)]
pub struct EvidenceRecord {
    pub schema: u8,
    pub identity: Identity,
    pub configuration: Configuration,
    pub target: Target,
    pub execution: Execution,
    pub claims: Vec<Claim>,
    pub result: ResultBlock,
    pub integrity: Integrity,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, Serialize)]
pub struct Identity {
    pub session_id: String,
    pub capture_id: String,
    pub device_source: String,
    pub hello_schema: Option<u8>,
    pub hello_profile: Option<u8>,
    pub firmware_caps: Option<String>,
    pub board_caps: Option<String>,
    /// Not on EP2 today. Null until firmware advertises a build hash.
    pub firmware_build_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Configuration {
    pub sck_id: Option<u8>,
    pub sck_transport: Option<&'static str>,
    pub sck_hz_observed: Option<u32>,
    pub diagnostics: DiagnosticsCfg,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticsCfg {
    pub trace: bool,
    pub trigger: bool,
    pub pretrigger: bool,
    pub timestamp: bool,
    pub persistence: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Target {
    /// ISP signature is not an EP2 frame. Null unless a tagged host source supplies it.
    pub signature: Option<String>,
    pub signature_source: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Execution {
    pub session: bool,
    pub session_end: bool,
    pub reset_assert: bool,
    pub reset_release: bool,
    pub ep_attempts: usize,
    pub sck_ids: Vec<u8>,
    pub trace_events: Option<u16>,
    pub trace_overflow: bool,
    pub frames: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Claim {
    pub name: &'static str,
    pub expected: String,
    pub observed: String,
    pub verdict: &'static str,
    pub evidence: &'static str,
    pub confidence: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResultBlock {
    pub tone: &'static str,
    pub diagnosis: String,
    pub observation: String,
    pub interpretation: String,
    pub cannot_prove: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Integrity {
    pub protocol_observed: bool,
    pub physical_capture: bool,
    pub persistent_evidence: bool,
    pub capture_digest: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Provenance {
    pub diagplane: String,
    pub protocol: String,
    pub complete: bool,
}

pub fn from_state(source: &str, state: &AppState, complete: bool) -> EvidenceRecord {
    let (tone, diagnosis) = diagnosis(state);
    let digest = capture_digest(state);
    let session_id = state
        .session_t0
        .map(|t| format!("{t:016x}"))
        .unwrap_or_else(|| "none".into());
    let mut sck_ids: Vec<u8> = state.ep_attempts.iter().filter_map(|a| a.sck_id).collect();
    sck_ids.sort_unstable();
    sck_ids.dedup();
    if sck_ids.is_empty() {
        if let Some(id) = state.sck_id {
            sck_ids.push(id);
        }
    }

    let caps = state.caps;
    let f = caps.map(|c| c.firmware);
    let b = caps.map(|c| c.board);
    let diagnostics = DiagnosticsCfg {
        trace: f.is_some_and(|c| c.contains(crate::caps::DiagCaps::TRACE)),
        trigger: f.is_some_and(|c| c.contains(crate::caps::DiagCaps::TRIGGER)),
        pretrigger: f.is_some_and(|c| c.contains(crate::caps::DiagCaps::PRETRIGGER)),
        timestamp: f.is_some_and(|c| c.contains(crate::caps::DiagCaps::TIMESTAMP)),
        persistence: false,
    };

    EvidenceRecord {
        schema: EVIDENCE_SCHEMA,
        identity: Identity {
            session_id,
            capture_id: digest.clone(),
            device_source: source.to_string(),
            hello_schema: state.hello_schema,
            hello_profile: state.hello_profile,
            firmware_caps: caps.map(|c| format!("0x{:08x}", c.firmware.0)),
            board_caps: caps.map(|c| format!("0x{:08x}", c.board.0)),
            firmware_build_id: None,
        },
        configuration: Configuration {
            sck_id: state.sck_id,
            sck_transport: match state.sck_sw {
                Some(true) => Some("SW"),
                Some(false) => Some("HW"),
                None => None,
            },
            sck_hz_observed: None,
            diagnostics,
        },
        target: Target {
            signature: None,
            signature_source: "none",
        },
        execution: Execution {
            session: state.saw_session,
            session_end: state.saw_session_end,
            reset_assert: state.saw_reset,
            reset_release: state.saw_release,
            ep_attempts: state.ep_attempts.len(),
            sck_ids,
            trace_events: state.trace_valid,
            trace_overflow: state.trace_overflow,
            frames: state.events.len(),
        },
        claims: claims(state),
        result: result_block(state, tone, &diagnosis),
        integrity: Integrity {
            protocol_observed: !state.events.is_empty(),
            physical_capture: b.is_some_and(|c| c.contains(crate::caps::BoardCaps::PHYSICAL_CAPTURE)),
            persistent_evidence: false,
            capture_digest: digest,
        },
        provenance: Provenance {
            diagplane: version::DIAGPLANE_VERSION.to_string(),
            protocol: version::PROTOCOL_VERSION_STR.to_string(),
            complete,
        },
    }
}

fn claims(state: &AppState) -> Vec<Claim> {
    let mut out = Vec::new();
    if let Some(ok) = state.line_ok {
        let name = match state.line_bit.unwrap_or(0) {
            2 => "RST",
            3 => "MOSI",
            5 => "SCK",
            _ => "ISP",
        };
        let drive = if state.line_drive_high == Some(true) {
            "HIGH"
        } else if state.line_drive_high == Some(false) && !ok {
            "LOW"
        } else {
            "PORT"
        };
        out.push(Claim {
            name: "LINE_FAULT",
            expected: format!("{name} PINx follows PORT after drive"),
            observed: format!(
                "{name} drive={drive} pin={:#04x}",
                state.line_pin.unwrap_or(0)
            ),
            verdict: if ok { "PASS" } else { "FAIL" },
            evidence: "protocol",
            confidence: "high",
        });
    }
    if let Some(fail) = state.ep_fail {
        let tx = state
            .ep_tx
            .map(|b| format!("{:02X} {:02X} {:02X} {:02X}", b[0], b[1], b[2], b[3]))
            .unwrap_or_else(|| "—".into());
        let rx = state
            .ep_rx
            .map(|b| format!("{:02X} {:02X} {:02X} {:02X}", b[0], b[1], b[2], b[3]))
            .unwrap_or_else(|| "—".into());
        let (verdict, confidence) = if fail {
            if state.ladder_all_silent() {
                ("FAIL", "medium")
            } else {
                ("FAIL", "medium")
            }
        } else {
            ("PASS", "high")
        };
        out.push(Claim {
            name: "ENABLEPROG",
            expected: "AVR echo 0x53 (programming enable)".into(),
            observed: format!("TX {tx}  RX {rx}"),
            verdict,
            evidence: "protocol",
            confidence,
        });
    }
    if let Some(&(addr, false)) = state.memop_pages.iter().find(|(_, ok)| !*ok) {
        out.push(Claim {
            name: "FLASH_POLL",
            expected: "page leaves 0xFF after write (AVR data polling)".into(),
            observed: format!("MEMOP CONT|FAIL @{addr:#06x}"),
            verdict: "FAIL",
            evidence: "protocol",
            confidence: "medium",
        });
    }
    if let Some(ok) = state.pins_ok {
        out.push(Claim {
            name: "ISP_PINS",
            expected: "RST/MOSI/SCK Hi-Z after disconnect".into(),
            observed: format!(
                "ddr={:#04x} pin={:#04x}",
                state.pins_ddr.unwrap_or(0),
                state.pins_pin.unwrap_or(0)
            ),
            verdict: if ok { "PASS" } else { "FAIL" },
            evidence: "protocol",
            confidence: "medium",
        });
    }
    out
}

fn result_block(state: &AppState, tone: DiagTone, diagnosis: &str) -> ResultBlock {
    let observation = if state.line_ok == Some(false) {
        "LINE_FAULT PINx did not follow PORT".into()
    } else if state.ep_fail == Some(true) {
        "ENABLEPROG FAIL".into()
    } else if state.memop_pages.iter().any(|(_, ok)| !*ok) {
        "MEMOP page poll FAIL".into()
    } else if state.ep_fail == Some(false) {
        "ENABLEPROG PASS".into()
    } else {
        "no ENABLEPROG result".into()
    };
    let (interpretation, cannot_prove) = if state.line_ok == Some(false) {
        (
            "programmer pad did not match the level just written".into(),
            "open vs short vs another driver — not FX2 / not which cm of ribbon".into(),
        )
    } else if state.ladder_all_silent() {
        (
            "target did not answer ENABLEPROG at any recorded SCK speed".into(),
            "RESET wiring, SCK, MISO, power, or electrical fault — protocol cannot pick one".into(),
        )
    } else if state.ep_fail == Some(true) && state.miso_silent {
        (
            "target did not enter programming mode (silent MISO)".into(),
            "RESET/SCK/MISO/electrical — not distinguished on EP2".into(),
        )
    } else if state.memop_pages.iter().any(|(_, ok)| !*ok) {
        (
            "flash page write did not complete polling".into(),
            "bad cell vs lock vs ISP drop during that page; verify-mismatch is avrdude-only".into(),
        )
    } else if state.ep_fail == Some(false) {
        (
            "programming enable sequence matched expected echo".into(),
            "pin edges — not observed".into(),
        )
    } else {
        ("insufficient ISP timeline".into(), "cause".into())
    };
    ResultBlock {
        tone: match tone {
            DiagTone::Ok => "ok",
            DiagTone::Bad => "bad",
            DiagTone::Warn => "warn",
            DiagTone::Info => "info",
        },
        diagnosis: diagnosis.to_string(),
        observation,
        interpretation,
        cannot_prove,
    }
}

fn capture_digest(state: &AppState) -> String {
    let mut buf = Vec::new();
    for e in &state.events {
        buf.extend_from_slice(&e.host_ns.to_le_bytes());
        buf.push(e.ty);
        buf.extend_from_slice(e.text.as_bytes());
    }
    format!("{:08x}", crc32(&buf))
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

impl EvidenceRecord {
    pub fn emit_human(&self) {
        println!("USBASP2 DIAGNOSTIC EVIDENCE");
        println!("schema       = {}", self.schema);
        println!("session      = {}", self.identity.session_id);
        println!("capture      = {}", self.identity.capture_id);
        println!("source       = {}", self.identity.device_source);
        println!(
            "hello        = schema {:?}  profile {:?}",
            self.identity.hello_schema, self.identity.hello_profile
        );
        println!(
            "firmware_build_id = {}",
            self.identity
                .firmware_build_id
                .as_deref()
                .unwrap_or("(not on EP2)")
        );
        println!();
        println!("SCK:");
        println!(
            "  id         = {:?}",
            self.configuration.sck_id
        );
        println!(
            "  transport  = {}",
            self.configuration.sck_transport.unwrap_or("—")
        );
        println!("  observed   = (Hz not on wire)");
        println!();
        println!("TARGET:");
        println!(
            "  signature  = {}",
            self.target.signature.as_deref().unwrap_or("—")
        );
        println!("  source     = {}", self.target.signature_source);
        println!();
        for c in &self.claims {
            println!("{}:", c.name);
            println!("  expected   = {}", c.expected);
            println!("  observed   = {}", c.observed);
            println!("  verdict    = {}", c.verdict);
            println!("  evidence   = {}", c.evidence);
            println!("  confidence = {}", c.confidence);
            println!();
        }
        println!("RESULT:");
        println!("  [{}] {}", self.result.tone.to_uppercase(), self.result.diagnosis);
        println!("  observation     = {}", self.result.observation);
        println!("  interpretation  = {}", self.result.interpretation);
        println!("  cannot_prove    = {}", self.result.cannot_prove);
        println!();
        println!("CONFIDENCE:");
        println!(
            "  protocol_observed = {}",
            yn(self.integrity.protocol_observed)
        );
        println!(
            "  physical_capture  = {}",
            yn(self.integrity.physical_capture)
        );
        println!(
            "  persistent_evidence = {}",
            yn(self.integrity.persistent_evidence)
        );
        println!("  capture_digest   = {}", self.integrity.capture_digest);
        println!();
        println!(
            "diagplane {}  protocol {}  complete={}",
            self.provenance.diagplane,
            self.provenance.protocol,
            yn(self.provenance.complete)
        );
    }

    pub fn emit_json(&self) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(std::io::stdout(), self)?;
        println!();
        Ok(())
    }
}

fn yn(v: bool) -> &'static str {
    if v {
        "YES"
    } else {
        "NO"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;
    use crate::state::AppState;

    #[test]
    fn fail_sw_does_not_claim_physical() {
        let cap = demo::build_scenario("enableprog_fail_sw").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let ev = from_state("demo:enableprog_fail_sw", &st, true);
        assert_eq!(ev.schema, 1);
        assert!(!ev.integrity.physical_capture);
        assert!(ev.integrity.protocol_observed);
        assert!(!ev.integrity.persistent_evidence);
        assert_eq!(ev.target.signature_source, "none");
        assert!(ev.identity.firmware_build_id.is_none());
        let ep = ev.claims.iter().find(|c| c.name == "ENABLEPROG").unwrap();
        assert_eq!(ep.verdict, "FAIL");
        assert_eq!(ep.evidence, "protocol");
        assert!(ev.result.cannot_prove.contains("RESET"));
    }

    #[test]
    fn line_fault_is_not_physical_capture() {
        let cap = demo::build_scenario("line_fault_rst").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let ev = from_state("demo:line_fault_rst", &st, true);
        let lf = ev.claims.iter().find(|c| c.name == "LINE_FAULT").unwrap();
        assert_eq!(lf.verdict, "FAIL");
        assert!(!ev.integrity.physical_capture);
        assert!(ev.result.observation.contains("LINE_FAULT"));
    }

    #[test]
    fn ladder_silent_separates_cause() {
        let cap = demo::build_scenario("enableprog_ladder_silent").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let ev = from_state("demo:enableprog_ladder_silent", &st, true);
        assert!(ev.result.observation.contains("FAIL"));
        assert!(ev.result.interpretation.contains("any recorded SCK"));
        assert_eq!(ev.execution.ep_attempts, 3);
    }

    #[test]
    fn memop_ok_is_protocol_pass() {
        let cap = demo::build_scenario("memop_flash").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let ev = from_state("demo:memop_flash", &st, true);
        let ep = ev.claims.iter().find(|c| c.name == "ENABLEPROG").unwrap();
        assert_eq!(ep.verdict, "PASS");
        assert_eq!(ev.result.tone, "ok");
        assert!(!ev.integrity.physical_capture);
    }
}
