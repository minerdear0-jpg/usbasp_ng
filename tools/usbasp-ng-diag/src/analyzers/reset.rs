use crate::analysis::{
    CausalRelevance, Confidence, Domain, EvidenceSource, Finding, FindingStatus,
};
use crate::analyzers::Analyzer;
use crate::evidence::EvidenceRecord;

pub struct ResetAnalyzer;

impl Analyzer for ResetAnalyzer {
    fn id(&self) -> &'static str {
        "reset"
    }

    fn analyze(&self, ev: &EvidenceRecord) -> Vec<Finding> {
        let mut out = Vec::new();
        out.push(Finding {
            id: "RESET.INTENT",
            analyzer: self.id(),
            domain: Domain::IspProtocol,
            status: FindingStatus::Pass,
            scope: "RESET",
            confidence: Confidence::High,
            causal_relevance: CausalRelevance::Supporting,
            claim: "RESET drive intent recorded (not pin sense)".into(),
            expected: "ASSERT then RELEASE".into(),
            observed: format!(
                "assert={} release={}",
                ev.execution.reset_assert, ev.execution.reset_release
            ),
            source: EvidenceSource::UsbaspInternal,
            evidence: vec!["DIAG_RESET flags = programmer drive intent".into()],
        });

        if let Some(c) = ev.claims.iter().find(|c| c.name == "LINE_FAULT") {
            let fail = c.verdict == "FAIL";
            let phys = ev.integrity.physical_capture;
            out.push(Finding {
                id: "LINE.RST_ECHO",
                analyzer: self.id(),
                domain: Domain::PhysicalLine,
                status: if fail {
                    FindingStatus::Anomaly
                } else {
                    FindingStatus::Pass
                },
                scope: "RST",
                confidence: if fail && !phys {
                    Confidence::Low
                } else if fail {
                    Confidence::Medium
                } else {
                    Confidence::High
                },
                causal_relevance: if fail {
                    CausalRelevance::Unknown
                } else {
                    CausalRelevance::Supporting
                },
                claim: if fail {
                    "RST GPIO echo mismatch (MCU PINx vs PORT, not ISP connector)".into()
                } else {
                    "RST/MOSI/SCK PINx followed PORT after drive".into()
                },
                expected: c.expected.clone(),
                observed: c.observed.clone(),
                source: EvidenceSource::UsbaspInternal,
                evidence: vec![
                    "DIAG_LINE_FAULT".into(),
                    format!("physical_capture={}", if phys { "YES" } else { "NO" }),
                    "PINx ≠ cable; GPIO→trace→inverter→connector→target".into(),
                ],
            });
        }
        out
    }
}
