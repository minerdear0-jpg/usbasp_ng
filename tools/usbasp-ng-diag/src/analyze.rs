//! Host analysis: Evidence → Findings → Verdict. Not firmware.
//!
//! Decoder stays dumb (frames → Evidence). Analyzers never see HID packets.
//! TUI is a frontend, not this engine.

use crate::evidence::EvidenceRecord;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(dead_code)] // reserved for UART / FX2 / host avrdude tags
pub enum EvidenceSource {
    UsbaspInternal,
    TargetUart,
    PhysicalCapture,
    HostProtocol,
    UserAssertion,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Finding {
    pub id: &'static str,
    pub analyzer: &'static str,
    pub severity: Severity,
    pub confidence: Confidence,
    pub claim: String,
    pub expected: String,
    pub observed: String,
    pub source: EvidenceSource,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Verdict {
    pub result: &'static str,
    pub likely: String,
    pub supported_by: Vec<String>,
    pub not_proven: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub verdict: Verdict,
}

pub trait Analyzer {
    fn id(&self) -> &'static str;
    fn analyze(&self, ev: &EvidenceRecord) -> Vec<Finding>;
}

/// CONNECT → SCK → RESET → ENABLEPROG → MEMOP → DISCONNECT. No Hz, no FX2.
pub struct IspSessionAnalyzer;

impl Analyzer for IspSessionAnalyzer {
    fn id(&self) -> &'static str {
        "isp_session"
    }

    fn analyze(&self, ev: &EvidenceRecord) -> Vec<Finding> {
        let mut out = Vec::new();
        let src = EvidenceSource::UsbaspInternal;

        out.push(Finding {
            id: "ISP.SESSION",
            analyzer: self.id(),
            severity: if ev.execution.session_end {
                Severity::Info
            } else {
                Severity::Warn
            },
            confidence: Confidence::High,
            claim: if ev.execution.session_end {
                "ISP session opened and closed".into()
            } else {
                "ISP session did not emit SESSION_END".into()
            },
            expected: "SESSION_BEGIN then SESSION_END".into(),
            observed: format!(
                "begin={} end={} frames={}",
                ev.execution.session, ev.execution.session_end, ev.execution.frames
            ),
            source: src,
            evidence: vec![format!("session_id={}", ev.identity.session_id)],
        });

        out.push(Finding {
            id: "ISP.RESET",
            analyzer: self.id(),
            severity: Severity::Info,
            confidence: Confidence::High,
            claim: "RESET drive intent recorded (not pin sense)".into(),
            expected: "ASSERT then RELEASE".into(),
            observed: format!(
                "assert={} release={}",
                ev.execution.reset_assert, ev.execution.reset_release
            ),
            source: src,
            evidence: vec!["DIAG_RESET flags = drive intent".into()],
        });

        if let Some(c) = ev.claims.iter().find(|c| c.name == "LINE_FAULT") {
            let fail = c.verdict == "FAIL";
            out.push(Finding {
                id: "ISP.LINE",
                analyzer: self.id(),
                severity: if fail { Severity::Error } else { Severity::Info },
                confidence: Confidence::High,
                claim: if fail {
                    "programmer pad did not follow PORT after drive".into()
                } else {
                    "RST/MOSI/SCK PINx followed PORT after drive".into()
                },
                expected: c.expected.clone(),
                observed: c.observed.clone(),
                source: src,
                evidence: vec!["DIAG_LINE_FAULT".into()],
            });
        }

        if let Some(c) = ev.claims.iter().find(|c| c.name == "ENABLEPROG") {
            let fail = c.verdict == "FAIL";
            out.push(Finding {
                id: "ISP.ENABLEPROG",
                analyzer: self.id(),
                severity: if fail { Severity::Error } else { Severity::Info },
                confidence: if fail { Confidence::Medium } else { Confidence::High },
                claim: if fail {
                    "ENABLEPROG response is invalid".into()
                } else {
                    "ENABLEPROG echo matched expected 0x53".into()
                },
                expected: c.expected.clone(),
                observed: c.observed.clone(),
                source: src,
                evidence: vec![
                    format!("attempts={}", ev.execution.ep_attempts),
                    format!("sck_ids={:?}", ev.execution.sck_ids),
                    format!("transport={:?}", ev.configuration.sck_transport),
                ],
            });
        }

        if let Some(c) = ev.claims.iter().find(|c| c.name == "FLASH_POLL") {
            out.push(Finding {
                id: "ISP.FLASH_POLL",
                analyzer: self.id(),
                severity: Severity::Error,
                confidence: Confidence::Medium,
                claim: "flash page write did not complete data polling".into(),
                expected: c.expected.clone(),
                observed: c.observed.clone(),
                source: src,
                evidence: vec!["MEMOP CONT|FAIL".into()],
            });
        }

        if let Some(c) = ev.claims.iter().find(|c| c.name == "ISP_PINS") {
            let fail = c.verdict == "FAIL";
            out.push(Finding {
                id: "ISP.PINS",
                analyzer: self.id(),
                severity: if fail { Severity::Error } else { Severity::Info },
                confidence: Confidence::Medium,
                claim: if fail {
                    "ISP pins still driving after disconnect".into()
                } else {
                    "ISP pins Hi-Z after disconnect".into()
                },
                expected: c.expected.clone(),
                observed: c.observed.clone(),
                source: src,
                evidence: vec!["DIAG_ISP_PINS after_disc".into()],
            });
        }

        out
    }
}

pub fn run_pipeline(ev: &EvidenceRecord) -> Analysis {
    let analyzers: [&dyn Analyzer; 1] = [&IspSessionAnalyzer];
    let findings: Vec<Finding> = analyzers
        .iter()
        .flat_map(|a| a.analyze(ev))
        .collect();
    let verdict = correlate(ev, &findings);
    Analysis { findings, verdict }
}

fn correlate(ev: &EvidenceRecord, findings: &[Finding]) -> Verdict {
    let line_fail = findings.iter().any(|f| f.id == "ISP.LINE" && f.severity == Severity::Error);
    let ep_fail = findings.iter().any(|f| f.id == "ISP.ENABLEPROG" && f.severity == Severity::Error);
    let poll_fail = findings.iter().any(|f| f.id == "ISP.FLASH_POLL");
    let ep_pass = findings.iter().any(|f| f.id == "ISP.ENABLEPROG" && f.severity == Severity::Info);
    let supported: Vec<String> = findings
        .iter()
        .map(|f| format!("{}: {}", f.id, f.claim))
        .collect();

    if line_fail {
        return Verdict {
            result: "FAIL",
            likely: "programmer pad did not follow the level just written".into(),
            supported_by: supported,
            not_proven: vec![
                "open vs short vs another driver".into(),
                "electrical integrity of the ribbon".into(),
                "FX2 / PHYSICAL_CAPTURE".into(),
            ],
        };
    }
    if poll_fail {
        return Verdict {
            result: "FAIL",
            likely: "a flash page did not leave 0xFF after write".into(),
            supported_by: supported,
            not_proven: vec![
                "bad cell vs lock vs ISP drop on that page".into(),
                "avrdude verify-mismatch (not on EP2)".into(),
            ],
        };
    }
    if ep_fail {
        let likely = if ev.execution.ep_attempts >= 2 && ev.execution.sck_ids.len() >= 2
        {
            "target did not answer ENABLEPROG at any recorded SCK speed".into()
        } else {
            "target did not enter programming mode".into()
        };
        return Verdict {
            result: "FAIL",
            likely,
            supported_by: supported,
            not_proven: vec![
                "RESET/SCK/MISO electrical".into(),
                "physical SCK edges".into(),
            ],
        };
    }
    if ep_pass {
        return Verdict {
            result: "PASS",
            likely: "programming-enable sequence matched firmware observation".into(),
            supported_by: supported,
            not_proven: vec![
                "pin edges".into(),
                "PHYSICAL_CAPTURE".into(),
                "target signature (not an EP2 frame)".into(),
            ],
        };
    }
    Verdict {
        result: "INCOMPLETE",
        likely: "not enough ISP timeline for a session verdict".into(),
        supported_by: supported,
        not_proven: vec!["cause".into()],
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Usbasp2eFile {
    pub format: &'static str,
    pub format_version: u8,
    pub evidence: EvidenceRecord,
    pub analysis: Analysis,
}

impl Usbasp2eFile {
    pub fn from_evidence(evidence: EvidenceRecord) -> Self {
        let analysis = run_pipeline(&evidence);
        Self {
            format: "usbasp2e",
            format_version: 1,
            evidence,
            analysis,
        }
    }

    pub fn emit_human(&self) {
        println!("USBASP2 ANALYSIS  format={} v{}", self.format, self.format_version);
        println!(
            "session={}  capture={}",
            self.evidence.identity.session_id, self.evidence.identity.capture_id
        );
        println!();
        for f in &self.analysis.findings {
            println!(
                "FINDING {}  [{}] confidence={:?}",
                f.id,
                format!("{:?}", f.severity).to_uppercase(),
                f.confidence
            );
            println!("  claim     {}", f.claim);
            println!("  expected  {}", f.expected);
            println!("  observed  {}", f.observed);
            println!("  source    {:?}", f.source);
            for e in &f.evidence {
                println!("  evidence  {e}");
            }
            println!();
        }
        let v = &self.analysis.verdict;
        println!("VERDICT: {}", v.result);
        println!("LIKELY:  {}", v.likely);
        println!("SUPPORTED BY:");
        for s in &v.supported_by {
            println!("  - {s}");
        }
        println!("NOT PROVEN:");
        for s in &v.not_proven {
            println!("  - {s}");
        }
    }

    pub fn emit_json(&self) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(std::io::stdout(), self)?;
        println!();
        Ok(())
    }

    pub fn write_path(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let f = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;
    use crate::evidence;
    use crate::state::AppState;

    fn ev(name: &str) -> crate::evidence::EvidenceRecord {
        let cap = demo::build_scenario(name).unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        evidence::from_state(&format!("demo:{name}"), &st, true)
    }

    #[test]
    fn fail_sw_verdict_is_fail_not_firmware_philosophy() {
        let a = run_pipeline(&ev("enableprog_fail_sw"));
        assert_eq!(a.verdict.result, "FAIL");
        assert!(a.verdict.likely.contains("programming mode"));
        assert!(a.verdict.not_proven.iter().any(|s| s.contains("electrical")));
        let ep = a
            .findings
            .iter()
            .find(|f| f.id == "ISP.ENABLEPROG")
            .unwrap();
        assert_eq!(ep.source, EvidenceSource::UsbaspInternal);
        assert_eq!(ep.severity, Severity::Error);
    }

    #[test]
    fn memop_flash_verdict_is_pass() {
        let a = run_pipeline(&ev("memop_flash"));
        assert_eq!(a.verdict.result, "PASS");
        assert!(a
            .verdict
            .not_proven
            .iter()
            .any(|s| s.contains("PHYSICAL_CAPTURE")));
    }

    #[test]
    fn line_fault_verdict_is_pad() {
        let a = run_pipeline(&ev("line_fault_rst"));
        assert_eq!(a.verdict.result, "FAIL");
        assert!(a.verdict.likely.contains("pad"));
        assert!(a.findings.iter().any(|f| f.id == "ISP.LINE"));
    }
}
