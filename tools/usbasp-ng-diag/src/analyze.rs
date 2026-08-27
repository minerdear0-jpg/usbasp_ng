//! Facade: Evidence → analyzers → correlation → Verdict. Firmware stays dumb.

pub use crate::analysis::{Analysis, Finding};

use crate::analysis::correlate;
use crate::evidence::EvidenceRecord;
use serde::Serialize;

pub fn run_pipeline(ev: &EvidenceRecord) -> Analysis {
    let findings: Vec<Finding> = crate::analyzers::all()
        .iter()
        .flat_map(|a| a.analyze(ev))
        .collect();
    let verdict = correlate(ev, &findings);
    Analysis { findings, verdict }
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
        println!(
            "USBASP2 ANALYSIS  format={} v{}",
            self.format, self.format_version
        );
        println!(
            "session={}  capture={}",
            self.evidence.identity.session_id, self.evidence.identity.capture_id
        );
        println!();
        for f in &self.analysis.findings {
            println!(
                "FINDING {}  domain={:?}  status={:?}  conf={:?}  causal={:?}  scope={}",
                f.id, f.domain, f.status, f.confidence, f.causal_relevance, f.scope
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
    use crate::analysis::{CausalRelevance, Confidence, EvidenceSource, FindingStatus};
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
    fn fail_sw_is_fail_unconfirmed() {
        let a = run_pipeline(&ev("enableprog_fail_sw"));
        assert_eq!(a.verdict.result, "FAIL_UNCONFIRMED");
        assert!(a.verdict.likely.contains("programming mode"));
        let ep = a
            .findings
            .iter()
            .find(|f| f.id == "ISP.ENABLEPROG")
            .unwrap();
        assert_eq!(ep.status, FindingStatus::Fail);
        assert_eq!(ep.source, EvidenceSource::UsbaspInternal);
    }

    #[test]
    fn memop_flash_is_pass() {
        let a = run_pipeline(&ev("memop_flash"));
        assert_eq!(a.verdict.result, "PASS");
    }

    #[test]
    fn rst_anomaly_with_prog_is_pass_with_anomaly() {
        let a = run_pipeline(&ev("pass_with_rst_anomaly"));
        assert_eq!(a.verdict.result, "PASS_WITH_ANOMALY", "{:?}", a.verdict.likely);
        let line = a
            .findings
            .iter()
            .find(|f| f.id == "LINE.RST_ECHO")
            .unwrap();
        assert_eq!(line.status, FindingStatus::Anomaly);
        assert_eq!(line.confidence, Confidence::Low);
        assert_eq!(line.causal_relevance, CausalRelevance::Unknown);
        let ep = a
            .findings
            .iter()
            .find(|f| f.id == "ISP.ENABLEPROG")
            .unwrap();
        assert_eq!(ep.status, FindingStatus::Pass);
        assert!(a.verdict.likely.contains("inconsistent"));
    }

    #[test]
    fn line_only_is_inconclusive() {
        let a = run_pipeline(&ev("line_fault_rst"));
        assert_eq!(a.verdict.result, "INCONCLUSIVE");
    }
}
