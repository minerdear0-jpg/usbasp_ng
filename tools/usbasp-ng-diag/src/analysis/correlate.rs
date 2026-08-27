use crate::analysis::finding::{CausalRelevance, Finding, FindingStatus};
use crate::analysis::verdict::Verdict;
use crate::evidence::EvidenceRecord;

pub fn correlate(ev: &EvidenceRecord, findings: &[Finding]) -> Verdict {
    let ep_pass = findings.iter().any(|f| {
        f.id == "ISP.ENABLEPROG" && f.status == FindingStatus::Pass
    });
    let ep_fail = findings.iter().any(|f| {
        f.id == "ISP.ENABLEPROG" && f.status == FindingStatus::Fail
    });
    let flash_ok = findings.iter().any(|f| {
        f.id == "ISP.FLASH" && f.status == FindingStatus::Pass
    });
    let flash_fail = findings.iter().any(|f| f.id == "ISP.FLASH_POLL");
    let line_anom = findings.iter().any(|f| {
        f.id == "LINE.RST_ECHO" && f.status == FindingStatus::Anomaly
    });
    let phys = ev.integrity.physical_capture;

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
                likely: "ISP operation succeeded. RST GPIO echo mismatch is inconsistent with ENABLEPROG/MEMOP and is not a physical proof.".into(),
                supported_by: supported,
                not_proven: vec![
                    "physical RST on the connector (PINx is the MCU pad)".into(),
                    "open vs short vs inverter vs probe timing".into(),
                    "FX2 / PHYSICAL_CAPTURE".into(),
                    "RST anomaly as cause of a programming failure (none observed)".into(),
                ],
            };
        }
        return Verdict {
            result: "PASS",
            likely: "programming-enable sequence matched firmware observation".into(),
            supported_by: supported,
            not_proven: vec![
                "pin edges".into(),
                "PHYSICAL_CAPTURE".into(),
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
        let result = if phys {
            "FAIL_CONFIRMED"
        } else {
            "FAIL_UNCONFIRMED"
        };
        return Verdict {
            result,
            likely: if ev.execution.ep_attempts >= 2 && ev.execution.sck_ids.len() >= 2 {
                "target did not answer ENABLEPROG at any recorded SCK speed".into()
            } else {
                "target did not enter programming mode".into()
            },
            supported_by: supported,
            not_proven: vec![
                rst.into(),
                "physical SCK/RST edges".into(),
            ],
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

    let _ = (flash_ok, CausalRelevance::Unknown);
    Verdict {
        result: "INCONCLUSIVE",
        likely: "not enough ISP timeline for a session verdict".into(),
        supported_by: supported,
        not_proven: vec!["cause".into()],
    }
}
