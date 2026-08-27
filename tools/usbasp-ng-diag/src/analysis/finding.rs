use serde::{Deserialize, Serialize};

use crate::analysis::Confidence;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(dead_code)]
pub enum EvidenceSource {
    UsbaspInternal,
    TargetUart,
    PhysicalCapture,
    HostProtocol,
    UserAssertion,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Domain {
    Session,
    IspProtocol,
    PhysicalLine,
    SckConfig,
    Correlation,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingStatus {
    Pass,
    Fail,
    Anomaly,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(dead_code)]
pub enum CausalRelevance {
    Required,
    Supporting,
    Incidental,
    Contradicted,
    Unknown,
    ExplainsSuccess,
    Plausible,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Finding {
    pub id: &'static str,
    pub analyzer: &'static str,
    pub domain: Domain,
    pub status: FindingStatus,
    pub scope: &'static str,
    pub confidence: Confidence,
    pub causal_relevance: CausalRelevance,
    pub claim: String,
    pub expected: String,
    pub observed: String,
    pub source: EvidenceSource,
    pub evidence: Vec<String>,
}
