mod causality;
mod confidence;
mod correlate;
mod finding;
mod verdict;

pub use causality::CausalRelevance;
pub use confidence::Confidence;
pub use correlate::correlate;
pub use finding::{Domain, EvidenceSource, Finding, FindingStatus};
pub use verdict::Verdict;

use serde::Serialize;

/// Frozen host analysis semantics. Bump only when finding/verdict meaning changes.
pub const ANALYZER_SCHEMA: &str = "evidence-v1";

#[derive(Clone, Debug, Serialize)]
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub verdict: Verdict,
}
