use crate::analysis::{
    CausalRelevance, Confidence, Domain, EvidenceSource, Finding, FindingStatus,
};
use crate::analyzers::Analyzer;
use crate::evidence::EvidenceRecord;

pub struct IspAnalyzer;

impl Analyzer for IspAnalyzer {
    fn id(&self) -> &'static str {
        "isp"
    }

    fn analyze(&self, ev: &EvidenceRecord) -> Vec<Finding> {
        let mut out = Vec::new();

        if let Some(c) = ev.claims.iter().find(|c| c.name == "ENABLEPROG") {
            let fail = c.verdict == "FAIL";
            out.push(Finding {
                id: "ISP.ENABLEPROG",
                analyzer: self.id(),
                domain: Domain::IspProtocol,
                status: if fail {
                    FindingStatus::Fail
                } else {
                    FindingStatus::Pass
                },
                scope: "SESSION",
                confidence: if fail {
                    Confidence::Medium
                } else {
                    Confidence::High
                },
                causal_relevance: if fail {
                    CausalRelevance::Required
                } else {
                    CausalRelevance::ExplainsSuccess
                },
                claim: if fail {
                    "ENABLEPROG response is invalid".into()
                } else {
                    "target entered programming mode (echo 0x53)".into()
                },
                expected: c.expected.clone(),
                observed: c.observed.clone(),
                source: EvidenceSource::UsbaspInternal,
                evidence: vec![
                    format!("attempts={}", ev.execution.ep_attempts),
                    format!("sck_ids={:?}", ev.execution.sck_ids),
                ],
            });
        }

        if ev.execution.memop_ok == Some(true) {
            out.push(Finding {
                id: "ISP.FLASH",
                analyzer: self.id(),
                domain: Domain::IspProtocol,
                status: FindingStatus::Pass,
                scope: "SESSION",
                confidence: Confidence::High,
                causal_relevance: CausalRelevance::ExplainsSuccess,
                claim: "flash page operations succeeded after ENABLEPROG".into(),
                expected: "MEMOP CONT|OK".into(),
                observed: format!(
                    "pages={}",
                    ev.execution.memop_pages.unwrap_or(0)
                ),
                source: EvidenceSource::UsbaspInternal,
                evidence: vec!["subsequent MEMOP PASS corroborates programming mode".into()],
            });
        }

        if let Some(c) = ev.claims.iter().find(|c| c.name == "FLASH_POLL") {
            out.push(Finding {
                id: "ISP.FLASH_POLL",
                analyzer: self.id(),
                domain: Domain::IspProtocol,
                status: FindingStatus::Fail,
                scope: "SESSION",
                confidence: Confidence::Medium,
                causal_relevance: CausalRelevance::Required,
                claim: "flash page write did not complete data polling".into(),
                expected: c.expected.clone(),
                observed: c.observed.clone(),
                source: EvidenceSource::UsbaspInternal,
                evidence: vec!["MEMOP CONT|FAIL".into()],
            });
        }

        if let Some(c) = ev.claims.iter().find(|c| c.name == "ISP_PINS") {
            let fail = c.verdict == "FAIL";
            out.push(Finding {
                id: "ISP.PINS",
                analyzer: self.id(),
                domain: Domain::IspProtocol,
                status: if fail {
                    FindingStatus::Fail
                } else {
                    FindingStatus::Pass
                },
                scope: "SESSION",
                confidence: Confidence::Medium,
                causal_relevance: CausalRelevance::Supporting,
                claim: if fail {
                    "ISP pins still driving after disconnect".into()
                } else {
                    "ISP pins Hi-Z after disconnect".into()
                },
                expected: c.expected.clone(),
                observed: c.observed.clone(),
                source: EvidenceSource::UsbaspInternal,
                evidence: vec!["DIAG_ISP_PINS after_disc".into()],
            });
        }

        out
    }
}
