mod isp;
mod reset;
mod sck;
mod session;

use crate::analysis::Finding;
use crate::evidence::EvidenceRecord;

pub trait Analyzer {
    fn id(&self) -> &'static str;
    fn analyze(&self, ev: &EvidenceRecord) -> Vec<Finding>;
}

pub fn all() -> [&'static dyn Analyzer; 4] {
    [
        &session::SessionAnalyzer,
        &sck::SckAnalyzer,
        &reset::ResetAnalyzer,
        &isp::IspAnalyzer,
    ]
}
