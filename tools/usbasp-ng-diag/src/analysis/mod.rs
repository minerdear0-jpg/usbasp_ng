mod causality;
mod correlate;
mod finding;
mod verdict;

pub use causality::CausalRelevance;
pub use correlate::correlate;
pub use finding::{Domain, EvidenceSource, Finding, FindingStatus};
pub use verdict::Verdict;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, Serialize)]
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub verdict: Verdict,
}
