use crate::analysis::{
    CausalRelevance, Confidence, Domain, EvidenceSource, Finding, FindingStatus,
};
use crate::analyzers::Analyzer;
use crate::evidence::EvidenceRecord;

pub struct SckAnalyzer;

impl Analyzer for SckAnalyzer {
    fn id(&self) -> &'static str {
        "sck"
    }

    fn analyze(&self, ev: &EvidenceRecord) -> Vec<Finding> {
        let Some(id) = ev.configuration.sck_id else {
            return Vec::new();
        };
        vec![Finding {
            id: "SCK.CONFIG",
            analyzer: self.id(),
            domain: Domain::SckConfig,
            status: FindingStatus::Pass,
            scope: "SCK",
            confidence: Confidence::High,
            causal_relevance: CausalRelevance::Supporting,
            claim: "SCK configuration recorded (id + HW/SW, not Hz)".into(),
            expected: "DIAG_SCK_CONFIG".into(),
            observed: format!(
                "id={id} transport={:?} hz=unavailable",
                ev.configuration.sck_transport
            ),
            source: EvidenceSource::UsbaspInternal,
            evidence: vec!["PHYSICAL_CAPTURE=NO — not a measured period".into()],
        }]
    }
}
