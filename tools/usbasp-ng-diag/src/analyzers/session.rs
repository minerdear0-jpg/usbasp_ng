use crate::analysis::{
    CausalRelevance, Confidence, Domain, EvidenceSource, Finding, FindingStatus,
};
use crate::analyzers::Analyzer;
use crate::evidence::EvidenceRecord;

pub struct SessionAnalyzer;

impl Analyzer for SessionAnalyzer {
    fn id(&self) -> &'static str {
        "session"
    }

    fn analyze(&self, ev: &EvidenceRecord) -> Vec<Finding> {
        let ok = ev.execution.session && ev.execution.session_end;
        vec![Finding {
            id: "SESSION.LIFECYCLE",
            analyzer: self.id(),
            domain: Domain::Session,
            status: if ok {
                FindingStatus::Pass
            } else if ev.execution.session {
                FindingStatus::Anomaly
            } else {
                FindingStatus::Fail
            },
            scope: "SESSION",
            confidence: Confidence::High,
            causal_relevance: CausalRelevance::Supporting,
            claim: if ok {
                "ISP session opened and closed".into()
            } else {
                "ISP session incomplete".into()
            },
            expected: "SESSION_BEGIN then SESSION_END".into(),
            observed: format!(
                "begin={} end={} frames={}",
                ev.execution.session, ev.execution.session_end, ev.execution.frames
            ),
            source: EvidenceSource::UsbaspInternal,
            evidence: vec![format!("session_id={}", ev.identity.session_id)],
        }]
    }
}
