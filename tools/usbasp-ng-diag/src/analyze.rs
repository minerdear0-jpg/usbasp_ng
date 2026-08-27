//! Facade: analyzers observe; correlator aggregates + verdict. Firmware stays dumb.

pub use crate::analysis::{Analysis, Finding, ANALYZER_SCHEMA};

use crate::analysis::correlate;
use crate::capture::CaptureFile;
use crate::evidence::EvidenceRecord;
use crate::version;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const CONTAINER_FORMAT: &str = "usbasp2e";
pub const CONTAINER_VERSION: u8 = 2;

pub fn run_pipeline(ev: &EvidenceRecord) -> Analysis {
    let analyzed: Vec<Finding> = crate::analyzers::all()
        .iter()
        .flat_map(|a| a.analyze(ev))
        .collect();
    correlate(ev, &analyzed)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

#[derive(Clone, Debug, Serialize)]
pub struct Usbasp2eFile {
    pub format: &'static str,
    pub format_version: u8,
    pub manifest: Manifest,
    pub provenance: ContainerProvenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<RawCapture>,
    /// Derived observations. Must not replace `raw`.
    pub observations: EvidenceRecord,
    pub analysis: AnalysisBlock,
}

#[derive(Clone, Debug, Serialize)]
pub struct Manifest {
    pub schema: &'static str,
    pub engine: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContainerProvenance {
    pub firmware_build_id: Option<String>,
    pub firmware_build_hash: Option<String>,
    pub board_id: Option<String>,
    pub diag_schema: Option<u8>,
    pub diag_capabilities: Option<String>,
    pub session_id: String,
    pub capture_id: String,
    pub diagplane: String,
    pub protocol: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RawCapture {
    pub kind: &'static str,
    pub sha256: String,
    pub hex: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalysisBlock {
    pub engine: &'static str,
    pub analyzer_version: String,
    pub schema: &'static str,
    pub derived_from: String,
    pub findings: Vec<Finding>,
    pub verdict: crate::analysis::Verdict,
}

impl Usbasp2eFile {
    pub fn from_evidence(observations: EvidenceRecord) -> Self {
        Self::assemble(observations, None)
    }

    pub fn from_capture(observations: EvidenceRecord, cap: &CaptureFile) -> Self {
        Self::assemble(observations, Some(cap))
    }

    fn assemble(observations: EvidenceRecord, cap: Option<&CaptureFile>) -> Self {
        let analysis = run_pipeline(&observations);
        let raw = cap.map(|c| {
            let bytes = c.to_bytes(true);
            RawCapture {
                kind: "USBDIAGv",
                sha256: sha256_hex(&bytes),
                hex: hex_encode(&bytes),
            }
        });
        let derived_from = match &raw {
            Some(r) => format!("sha256:{}", r.sha256),
            None => format!("capture_digest:{}", observations.integrity.capture_digest),
        };
        let provenance = ContainerProvenance {
            firmware_build_id: observations.identity.firmware_build_id.clone(),
            firmware_build_hash: observations.identity.firmware_build_hash.clone(),
            board_id: observations.identity.board_id.clone(),
            diag_schema: observations.identity.hello_schema,
            diag_capabilities: observations.identity.firmware_caps.clone(),
            session_id: observations.identity.session_id.clone(),
            capture_id: observations.identity.capture_id.clone(),
            diagplane: observations.provenance.diagplane.clone(),
            protocol: observations.provenance.protocol.clone(),
        };
        Self {
            format: CONTAINER_FORMAT,
            format_version: CONTAINER_VERSION,
            manifest: Manifest {
                schema: ANALYZER_SCHEMA,
                engine: "usbasp-ng-diag",
            },
            provenance,
            raw,
            analysis: AnalysisBlock {
                engine: "usbasp-ng-diag",
                analyzer_version: version::DIAGPLANE_VERSION.to_string(),
                schema: ANALYZER_SCHEMA,
                derived_from,
                findings: analysis.findings,
                verdict: analysis.verdict,
            },
            observations,
        }
    }

    pub fn emit_human(&self) {
        println!(
            "USBASP2E  format={} v{}  schema={}",
            self.format, self.format_version, self.manifest.schema
        );
        println!(
            "analyzer {}  derived_from {}",
            self.analysis.analyzer_version, self.analysis.derived_from
        );
        println!(
            "session={}  capture={}",
            self.provenance.session_id, self.provenance.capture_id
        );
        println!(
            "firmware_build_id={:?}  firmware_build_hash={:?}  board_id={:?}",
            self.provenance.firmware_build_id,
            self.provenance.firmware_build_hash,
            self.provenance.board_id
        );
        if let Some(r) = &self.raw {
            println!("raw {}  sha256={}", r.kind, r.sha256);
        } else {
            println!("raw unavailable (live/jsonl ingest)");
        }
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

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{CausalRelevance, Confidence, EvidenceSource, FindingStatus};
    use crate::demo;
    use crate::evidence::{self, PhysicalEvidence};
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
        assert!(a.findings.iter().all(|f| f.id != "ISP.PROGRAMMING_PATH"));
        let ep = a
            .findings
            .iter()
            .find(|f| f.id == "ISP.ENABLEPROG")
            .unwrap();
        assert_eq!(ep.status, FindingStatus::Fail);
        assert_eq!(ep.source, EvidenceSource::UsbaspInternal);
    }

    #[test]
    fn capability_is_not_fail_confirmed() {
        let mut rec = ev("enableprog_fail_sw");
        rec.integrity.physical_capture_capability = true;
        rec.integrity.physical_capture = false;
        rec.sources.physical = None;
        let a = run_pipeline(&rec);
        assert_eq!(a.verdict.result, "FAIL_UNCONFIRMED");
        assert!(a.findings.iter().all(|f| f.id != "EVIDENCE.CONFLICT"));
    }

    #[test]
    fn physical_rst_stayed_high_is_conflict_and_confirmed() {
        let mut rec = ev("enableprog_fail_sw");
        rec.sources.physical = Some(PhysicalEvidence {
            capture_id: "FX2-test".into(),
            rst_low: Some(false),
        });
        rec.integrity.physical_capture = true;
        let a = run_pipeline(&rec);
        assert!(a.findings.iter().any(|f| f.id == "EVIDENCE.CONFLICT"));
        assert_eq!(a.verdict.result, "FAIL_CONFIRMED");
    }

    #[test]
    fn memop_flash_confirms_programming_path() {
        let a = run_pipeline(&ev("memop_flash"));
        assert_eq!(a.verdict.result, "PASS");
        let path = a
            .findings
            .iter()
            .find(|f| f.id == "ISP.PROGRAMMING_PATH")
            .unwrap();
        assert_eq!(path.status, FindingStatus::Pass);
        assert_eq!(path.confidence, Confidence::High);
        assert_eq!(path.analyzer, "correlator");
        let ep = a
            .findings
            .iter()
            .find(|f| f.id == "ISP.ENABLEPROG")
            .unwrap();
        assert_eq!(ep.status, FindingStatus::Pass);
    }

    #[test]
    fn enableprog_pass_without_memop_has_no_path_finding() {
        let a = run_pipeline(&ev("session_hw_pass"));
        assert_eq!(a.verdict.result, "PASS");
        assert!(a.findings.iter().all(|f| f.id != "ISP.PROGRAMMING_PATH"));
    }

    #[test]
    fn rst_anomaly_with_prog_is_pass_with_anomaly() {
        let a = run_pipeline(&ev("pass_with_rst_anomaly"));
        assert_eq!(a.verdict.result, "PASS_WITH_ANOMALY", "{:?}", a.verdict.likely);
        assert!(a.findings.iter().any(|f| f.id == "ISP.PROGRAMMING_PATH"));
        assert!(a.findings.iter().all(|f| f.id != "ISP.MEMOP_INCOMPLETE"));
        let ep = a
            .findings
            .iter()
            .find(|f| f.id == "ISP.ENABLEPROG")
            .unwrap();
        assert_eq!(ep.status, FindingStatus::Pass);
        assert!(a.verdict.likely.contains("anomalous"));
    }

    #[test]
    fn flash_abort_with_line_is_fail_not_pass_anomaly() {
        let a = run_pipeline(&ev("flash_abort_line_anomaly"));
        assert_eq!(a.verdict.result, "FAIL_UNCONFIRMED", "{:?}", a.verdict.likely);
        assert!(a.findings.iter().any(|f| f.id == "ISP.MEMOP_INCOMPLETE"));
        assert!(a.findings.iter().all(|f| f.id != "ISP.PROGRAMMING_PATH"));
        assert!(a.verdict.likely.contains("did not complete"));
    }

    #[test]
    fn line_only_is_inconclusive() {
        let a = run_pipeline(&ev("line_fault_rst"));
        assert_eq!(a.verdict.result, "INCONCLUSIVE");
    }

    #[test]
    fn fail_with_line_is_unconfirmed() {
        let a = run_pipeline(&ev("enableprog_fail_line_anomaly"));
        assert_eq!(a.verdict.result, "FAIL_UNCONFIRMED");
        let line = a
            .findings
            .iter()
            .find(|f| f.id == "LINE.RST_ECHO")
            .unwrap();
        assert_eq!(line.causal_relevance, CausalRelevance::Unknown);
        assert!(a.findings.iter().all(|f| f.id != "EVIDENCE.CONFLICT"));
    }

    #[test]
    fn container_keeps_raw_and_null_build_hash() {
        let cap = demo::build_scenario("pass_with_rst_anomaly").unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let ev = evidence::from_state("demo:pass_with_rst_anomaly", &st, true);
        let pack = Usbasp2eFile::from_capture(ev, &cap);
        assert_eq!(pack.format_version, 2);
        assert!(pack.raw.is_some());
        assert!(pack.analysis.derived_from.starts_with("sha256:"));
        assert!(pack.provenance.firmware_build_id.is_none());
        assert!(pack.provenance.firmware_build_hash.is_none());
        let raw = pack.raw.as_ref().unwrap();
        let decoded = (0..raw.hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&raw.hex[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(sha256_hex(&decoded), raw.sha256);
        assert_eq!(decoded, cap.to_bytes(true));
    }
}

#[cfg(test)]
mod goldens {
    use super::run_pipeline;
    use crate::analysis::{CausalRelevance, Confidence, FindingStatus};
    use crate::demo;
    use crate::evidence;
    use crate::state::AppState;
    use serde::Deserialize;
    use std::fs;
    use std::path::PathBuf;

    #[derive(Deserialize)]
    struct Golden {
        demo: String,
        expect: Expect,
    }

    #[derive(Deserialize)]
    struct Expect {
        verdict: String,
        #[serde(default)]
        findings: std::collections::BTreeMap<String, FindingExpect>,
        #[serde(default)]
        must_not_have: Vec<String>,
    }

    #[derive(Deserialize)]
    struct FindingExpect {
        status: FindingStatus,
        #[serde(default)]
        confidence: Option<Confidence>,
        #[serde(default)]
        causal_relevance: Option<CausalRelevance>,
    }

    fn goldens_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../host/goldens/evidence")
    }

    #[test]
    fn canonical_evidence_goldens() {
        let dir = goldens_dir();
        let mut files: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("goldens dir {}: {e}", dir.display()))
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        files.sort();
        assert_eq!(files.len(), 6, "expected six canonical goldens in {}", dir.display());
        for path in files {
            let spec: Golden = serde_json::from_str(&fs::read_to_string(&path).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let cap = demo::build_scenario(&spec.demo).unwrap();
            let mut st = AppState::default();
            st.ingest_capture(&cap);
            let ev = evidence::from_state(&format!("demo:{}", spec.demo), &st, true);
            let a = run_pipeline(&ev);
            assert_eq!(
                a.verdict.result, spec.expect.verdict,
                "{} verdict",
                path.display()
            );
            for (id, exp) in &spec.expect.findings {
                let f = a
                    .findings
                    .iter()
                    .find(|f| f.id == id.as_str())
                    .unwrap_or_else(|| panic!("{} missing finding {id}", path.display()));
                assert_eq!(f.status, exp.status, "{id} status");
                if let Some(c) = exp.confidence {
                    assert_eq!(f.confidence, c, "{id} confidence");
                }
                if let Some(c) = exp.causal_relevance {
                    assert_eq!(f.causal_relevance, c, "{id} causal");
                }
            }
            for id in &spec.expect.must_not_have {
                assert!(
                    a.findings.iter().all(|f| f.id != id.as_str()),
                    "{} must not have {id}",
                    path.display()
                );
            }
        }
    }
}

#[cfg(test)]
mod corpus {
    use super::*;
    use crate::demo;
    use crate::evidence;
    use crate::state::AppState;

    fn pack(name: &str) -> Usbasp2eFile {
        let cap = demo::build_scenario(name).unwrap();
        let mut st = AppState::default();
        st.ingest_capture(&cap);
        let ev = evidence::from_state(&format!("demo:{name}"), &st, true);
        Usbasp2eFile::from_capture(ev, &cap)
    }

    #[test]
    fn replay_corpus_is_nine_isp_sessions() {
        assert_eq!(demo::replay_corpus().len(), 11);
    }

    #[test]
    fn replay_is_deterministic() {
        for name in demo::replay_corpus() {
            let a = serde_json::to_vec(&run_pipeline(&{
                let cap = demo::build_scenario(name).unwrap();
                let mut st = AppState::default();
                st.ingest_capture(&cap);
                evidence::from_state(&format!("demo:{name}"), &st, true)
            }))
            .unwrap();
            let b = serde_json::to_vec(&run_pipeline(&{
                let cap = demo::build_scenario(name).unwrap();
                let mut st = AppState::default();
                st.ingest_capture(&cap);
                evidence::from_state(&format!("demo:{name}"), &st, true)
            }))
            .unwrap();
            assert_eq!(a, b, "{name} analysis JSON must not depend on run time");
        }
    }

    #[test]
    fn raw_is_immutable_across_analysis() {
        for name in demo::replay_corpus() {
            let p1 = pack(name);
            let p2 = pack(name);
            let r1 = p1.raw.as_ref().expect("demo has raw");
            let r2 = p2.raw.as_ref().expect("demo has raw");
            assert_eq!(r1.sha256, r2.sha256, "{name} raw sha256");
            assert_eq!(r1.hex, r2.hex, "{name} raw bytes");
            let a1 = serde_json::to_vec(&p1.analysis).unwrap();
            let a2 = serde_json::to_vec(&p2.analysis).unwrap();
            assert_eq!(a1, a2, "{name} derived analysis");
            assert_eq!(p1.observations.integrity.capture_digest, p2.observations.integrity.capture_digest);
        }
    }
}
