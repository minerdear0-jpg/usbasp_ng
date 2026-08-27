use crate::analysis::finding::{
    CausalRelevance, Domain, EvidenceSource, Finding, FindingStatus,
};
use crate::analysis::confidence::Confidence;
use crate::analysis::{Analysis, Verdict};
use crate::evidence::EvidenceRecord;

/// Analyzer findings stay immutable. Aggregates (path, conflict) are correlator-owned.
pub fn correlate(ev: &EvidenceRecord, analyzed: &[Finding]) -> Analysis {
    let mut findings = analyzed.to_vec();
    findings.extend(aggregates(ev, analyzed));
    let verdict = verdict_of(ev, &findings);
    Analysis { findings, verdict }
}

fn aggregates(ev: &EvidenceRecord, analyzed: &[Finding]) -> Vec<Finding> {
    let mut extra = Vec::new();
    let ep_pass = has(analyzed, "ISP.ENABLEPROG", FindingStatus::Pass);
    let flash_ok = has(analyzed, "ISP.FLASH", FindingStatus::Pass);
    let verify_ok = ev.execution.verify_ok == Some(true);

    if ep_pass && flash_ok {
        let mut evidence = vec![
            "ENABLEPROG PASS → programming_mode CONFIRMED".into(),
            "MEMOP PASS → programming_path CONFIRMED".into(),
        ];
        if verify_ok {
            evidence.push("VERIFY PASS corroborates path (confidence stays HIGH)".into());
        }
        extra.push(Finding {
            id: "ISP.PROGRAMMING_PATH",
            analyzer: "correlator",
            domain: Domain::Correlation,
            status: FindingStatus::Pass,
            scope: "SESSION",
            confidence: Confidence::High,
            causal_relevance: CausalRelevance::ExplainsSuccess,
            claim: "ISP programming path operational".into(),
            expected: "ENABLEPROG PASS then MEMOP PASS".into(),
            observed: format!(
                "ENABLEPROG PASS  MEMOP pages={}  verify={:?}",
                ev.execution.memop_pages.unwrap_or(0),
                ev.execution.verify_ok
            ),
            source: EvidenceSource::UsbaspInternal,
            evidence,
        });
    }

    if let Some(p) = &ev.sources.physical {
        if ev.execution.reset_assert && p.rst_low == Some(false) {
            extra.push(Finding {
                id: "EVIDENCE.CONFLICT",
                analyzer: "correlator",
                domain: Domain::Correlation,
                status: FindingStatus::Anomaly,
                scope: "RESET",
                confidence: Confidence::High,
                causal_relevance: CausalRelevance::Unknown,
                claim: "USBASP_INTERNAL RESET assert disagrees with PHYSICAL_CAPTURE RST level"
                    .into(),
                expected: "independent sources agree on RST assertion".into(),
                observed: format!(
                    "internal assert={}  physical rst_low={:?}  capture_id={}",
                    ev.execution.reset_assert, p.rst_low, p.capture_id
                ),
                source: EvidenceSource::PhysicalCapture,
                evidence: vec![
                    "conflict is not a verdict of which source is wrong".into(),
                    format!("physical capture_id={}", p.capture_id),
                ],
            });
        }
    }

    extra
}

fn has(findings: &[Finding], id: &str, status: FindingStatus) -> bool {
    findings.iter().any(|f| f.id == id && f.status == status)
}

fn verdict_of(ev: &EvidenceRecord, findings: &[Finding]) -> Verdict {
    let ep_pass = has(findings, "ISP.ENABLEPROG", FindingStatus::Pass);
    let ep_fail = has(findings, "ISP.ENABLEPROG", FindingStatus::Fail);
    let line_anom = has(findings, "LINE.RST_ECHO", FindingStatus::Anomaly);
    let path_ok = has(findings, "ISP.PROGRAMMING_PATH", FindingStatus::Pass);
    let conflict = findings.iter().any(|f| f.id == "EVIDENCE.CONFLICT");
    let flash_fail = findings.iter().any(|f| f.id == "ISP.FLASH_POLL");
    let phys_src = ev.sources.physical.is_some();

    let supported: Vec<String> = findings
        .iter()
        .map(|f| format!("{} [{:?}] {}", f.id, f.status, f.claim))
        .collect();

    if flash_fail {
        return Verdict {
            result: "FAIL_UNCONFIRMED",
            likely: "a flash page did not leave 0xFF after write".into(),
            supported_by: supported,
            not_proven: vec![
                "bad cell vs lock vs ISP drop".into(),
                "avrdude verify-mismatch (not on EP2)".into(),
            ],
        };
    }

    if ep_pass {
        if line_anom {
            return Verdict {
                result: "PASS_WITH_ANOMALY",
                likely: "ISP programming path confirmed. RST GPIO observation is anomalous but not established as causal.".into(),
                supported_by: supported,
                not_proven: vec![
                    "physical RST on the connector (PINx is the MCU pad)".into(),
                    "open vs short vs inverter vs probe timing".into(),
                    "PHYSICAL_CAPTURE evidence (capability is not evidence)".into(),
                    "RST anomaly as cause of a programming failure (none observed)".into(),
                ],
            };
        }
        return Verdict {
            result: "PASS",
            likely: if path_ok {
                "programming path confirmed (ENABLEPROG + MEMOP)".into()
            } else {
                "programming-enable sequence matched firmware observation".into()
            },
            supported_by: supported,
            not_proven: vec![
                "pin edges".into(),
                "PHYSICAL_CAPTURE evidence".into(),
                "target signature (not an EP2 frame)".into(),
            ],
        };
    }

    if ep_fail {
        let rst = if line_anom {
            "RST GPIO echo mismatch (plausible contributor, not cause)"
        } else {
            "RESET/SCK/MISO electrical"
        };
        // Capability never confirms. Independent capture that RST stayed HIGH can.
        let confirmed = conflict
            && phys_src
            && ev
                .sources
                .physical
                .as_ref()
                .is_some_and(|p| p.rst_low == Some(false));
        return Verdict {
            result: if confirmed {
                "FAIL_CONFIRMED"
            } else {
                "FAIL_UNCONFIRMED"
            },
            likely: if confirmed {
                "RST physical assertion failure (internal assert vs capture stayed HIGH)".into()
            } else if ev.execution.ep_attempts >= 2 && ev.execution.sck_ids.len() >= 2 {
                "target did not answer ENABLEPROG at any recorded SCK speed".into()
            } else {
                "target did not enter programming mode".into()
            },
            supported_by: supported,
            not_proven: if confirmed {
                vec!["which layer between MCU pad and connector".into()]
            } else {
                vec![rst.into(), "physical SCK/RST edges".into()]
            },
        };
    }

    if line_anom {
        return Verdict {
            result: "INCONCLUSIVE",
            likely: "RST GPIO echo anomaly only; no ENABLEPROG result to confirm or deny programming".into(),
            supported_by: supported,
            not_proven: vec!["session outcome".into(), "physical line".into()],
        };
    }

    Verdict {
        result: "INCONCLUSIVE",
        likely: "not enough ISP timeline for a session verdict".into(),
        supported_by: supported,
        not_proven: vec!["cause".into()],
    }
}
