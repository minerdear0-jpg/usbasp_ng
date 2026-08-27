//! Rule layer: findings + verdict → prose. No new claims.

use crate::analysis::{Analysis, FindingStatus};
use crate::evidence::EvidenceRecord;
use crate::explanation::{Explanation, Tone, EXPLANATION_SCHEMA};

fn has(a: &Analysis, id: &str) -> bool {
    a.findings.iter().any(|f| f.id == id)
}

fn status(a: &Analysis, id: &str) -> Option<FindingStatus> {
    a.findings.iter().find(|f| f.id == id).map(|f| f.status)
}

pub fn explain(ev: &EvidenceRecord, analysis: &Analysis) -> Explanation {
    let v = analysis.verdict.result;
    let path = status(analysis, "ISP.PROGRAMMING_PATH") == Some(FindingStatus::Pass);
    let ep_pass = status(analysis, "ISP.ENABLEPROG") == Some(FindingStatus::Pass);
    let ep_fail = status(analysis, "ISP.ENABLEPROG") == Some(FindingStatus::Fail);
    let line = status(analysis, "LINE.RST_ECHO") == Some(FindingStatus::Anomaly);
    let poll = has(analysis, "ISP.FLASH_POLL");
    let incomplete = has(analysis, "ISP.MEMOP_INCOMPLETE");
    let stall = has(analysis, "ISP.MEMOP_STALL");
    let flash_ok = status(analysis, "ISP.FLASH") == Some(FindingStatus::Pass);
    let pages = ev.execution.memop_pages;

    match v {
        "PASS_WITH_ANOMALY" => pass_with_anomaly(path, ep_pass, flash_ok, line, pages),
        "PASS" => pass_clean(path, ep_pass, flash_ok, pages),
        "FAIL_UNCONFIRMED" if incomplete => fail_incomplete(ep_pass, line),
        "FAIL_UNCONFIRMED" if poll => fail_poll(ep_pass, line, ev),
        "FAIL_UNCONFIRMED" if stall => fail_stall(ep_pass, line, pages),
        "FAIL_UNCONFIRMED" if ep_fail => fail_enableprog(line, ev),
        "FAIL_UNCONFIRMED" => fail_generic(analysis),
        "FAIL_CONFIRMED" => fail_confirmed(analysis),
        "INCONCLUSIVE" => inconclusive(line, ep_pass),
        _ => Explanation {
            schema: EXPLANATION_SCHEMA,
            tone: Tone::Info,
            headline: format!("SESSION {v}"),
            subhead: analysis.verdict.likely.clone(),
            why: vec![],
            why_warning: vec![],
            why_not_failure: vec![],
            why_failed: vec![],
            what_we_know: analysis.verdict.supported_by.clone(),
            not_proven: analysis.verdict.not_proven.clone(),
            certainty: vec![],
        },
    }
}

fn pass_with_anomaly(
    path: bool,
    ep_pass: bool,
    flash_ok: bool,
    line: bool,
    pages: Option<u8>,
) -> Explanation {
    let mut why = Vec::new();
    if ep_pass {
        why.push("Target responded to programming-enable (echo 0x53).".into());
    }
    if path || flash_ok {
        why.push("ISP programming path is confirmed.".into());
    }
    if let Some(n) = pages {
        if flash_ok || path {
            why.push(format!("Flash memory operation completed ({n} pages on EP2)."));
        }
    }
    if why.is_empty() {
        why.push("Requested ISP session completed successfully.".into());
    }

    let mut warn = Vec::new();
    if line {
        warn.push(
            "The programmer's RST GPIO readback did not match the expected pin state.".into(),
        );
        warn.push(
            "That observation is from the MCU pad (PINx), not an independent cable/probe measurement."
                .into(),
        );
    }

    let mut not_fail = Vec::new();
    if ep_pass {
        not_fail.push("Target nevertheless entered programming mode.".into());
    }
    if path || flash_ok {
        not_fail.push("Flash operations still completed successfully.".into());
    }
    not_fail.push(
        "No evidence establishes the RST anomaly as causal for this result.".into(),
    );

    Explanation {
        schema: EXPLANATION_SCHEMA,
        tone: Tone::Warn,
        headline: "PROGRAMMING SUCCEEDED".into(),
        subhead: if line {
            "1 ANOMALY DETECTED (RST GPIO echo)".into()
        } else {
            "ANOMALY DETECTED".into()
        },
        why,
        why_warning: warn,
        why_not_failure: not_fail,
        why_failed: vec![],
        what_we_know: vec![
            "USBasp communicated with the host.".into(),
            "Protocol observations are from USBASP_INTERNAL only.".into(),
        ],
        not_proven: vec![
            "Physical ISP cable / target RESET fault.".into(),
            "Independent PHYSICAL_CAPTURE of RST/SCK edges.".into(),
        ],
        certainty: vec![
            "HIGH — programming success".into(),
            "LOW — physical RESET fault".into(),
        ],
    }
}

fn pass_clean(path: bool, ep_pass: bool, flash_ok: bool, pages: Option<u8>) -> Explanation {
    let mut why = Vec::new();
    if ep_pass {
        why.push("Target responded to programming-enable (echo 0x53).".into());
    }
    if path || flash_ok {
        why.push("ISP programming path is confirmed.".into());
    }
    if let Some(n) = pages {
        if flash_ok || path {
            why.push(format!("Flash memory operation completed ({n} pages on EP2)."));
        }
    }
    Explanation {
        schema: EXPLANATION_SCHEMA,
        tone: Tone::Ok,
        headline: "PROGRAMMING SUCCEEDED".into(),
        subhead: String::new(),
        why,
        why_warning: vec![],
        why_not_failure: vec![],
        why_failed: vec![],
        what_we_know: vec!["Protocol path matched expected ENABLEPROG / MEMOP outcomes.".into()],
        not_proven: vec![
            "Pin edges (not on EP2).".into(),
            "PHYSICAL_CAPTURE.".into(),
        ],
        certainty: vec!["HIGH — programming success".into()],
    }
}

fn fail_incomplete(ep_pass: bool, line: bool) -> Explanation {
    let mut failed = vec![
        "A memory operation started on EP2 but never ended (no MEMOP END).".into(),
        "CONT page OK counts are not a finished write.".into(),
    ];
    if ep_pass {
        failed.push(
            "ENABLEPROG succeeded earlier — that does not mean the write finished.".into(),
        );
    }
    let mut not = vec![
        "Host USB I/O drop vs target vs cable — not distinguished on EP2 alone.".into(),
    ];
    if line {
        not.push("RST GPIO anomaly as the cause (observed, not established).".into());
    }
    Explanation {
        schema: EXPLANATION_SCHEMA,
        tone: Tone::Fail,
        headline: "PROGRAMMING FAILED".into(),
        subhead: "ROOT CAUSE NOT CONFIRMED — MEMOP incomplete".into(),
        why: vec![],
        why_warning: vec![],
        why_not_failure: vec![],
        why_failed: failed,
        what_we_know: vec!["ISP transaction was attempted; MEMOP END was not observed.".into()],
        not_proven: not,
        certainty: vec![
            "HIGH — operation did not complete on EP2".into(),
            "LOW — which physical layer dropped".into(),
        ],
    }
}

fn fail_poll(ep_pass: bool, line: bool, ev: &EvidenceRecord) -> Explanation {
    let addr = ev
        .claims
        .iter()
        .find(|c| c.name == "FLASH_POLL")
        .map(|c| c.observed.clone())
        .unwrap_or_else(|| "a flash page".into());
    let mut failed = vec![
        format!("Flash page data-polling failed ({addr})."),
        "MEMOP END|OK does not cancel CONT|FAIL (sticky poll failure).".into(),
    ];
    if ep_pass {
        failed.push("Programming mode was entered; the write still failed polling.".into());
    }
    let mut not = vec![
        "avrdude verify-mismatch detail is not on EP2.".into(),
        "Open vs short vs torn ribbon vs bad cell — not proven.".into(),
    ];
    if line {
        not.push("RST GPIO anomaly as sole cause (consistent at best, not proven).".into());
    }
    Explanation {
        schema: EXPLANATION_SCHEMA,
        tone: Tone::Fail,
        headline: "PROGRAMMING FAILED".into(),
        subhead: "ROOT CAUSE NOT CONFIRMED — flash poll FAIL".into(),
        why: vec![],
        why_warning: vec![],
        why_not_failure: vec![],
        why_failed: failed,
        what_we_know: vec![
            "USBasp communicated with the host.".into(),
            "At least one FLASH CONT|FAIL was recorded.".into(),
        ],
        not_proven: not,
        certainty: vec![
            "HIGH — write path did not complete cleanly".into(),
            "LOW — physical root cause".into(),
        ],
    }
}

fn fail_stall(ep_pass: bool, line: bool, pages: Option<u8>) -> Explanation {
    let mut failed = vec![
        "FLASH write stalled: multi-second host gap between MEMOP frames mid-write.".into(),
        "MEMOP END|OK after that gap is not a completed flash.".into(),
    ];
    if let Some(n) = pages {
        failed.push(format!("Firmware reported END with {n} pages — still not success after a stall."));
    }
    if ep_pass {
        failed.push(
            "ENABLEPROG succeeded earlier — that does not mean the write finished.".into(),
        );
    }
    let mut not = vec![
        "Host USB I/O drop vs cable vs target — not distinguished on EP2 alone.".into(),
        "avrdude verify-mismatch is corroboration, not EP2 proof.".into(),
    ];
    if line {
        not.push("RST GPIO anomaly as the cause (observed, not established).".into());
    }
    Explanation {
        schema: EXPLANATION_SCHEMA,
        tone: Tone::Fail,
        headline: "PROGRAMMING FAILED".into(),
        subhead: "ROOT CAUSE NOT CONFIRMED — MEMOP stall".into(),
        why: vec![],
        why_warning: vec![],
        why_not_failure: vec![],
        why_failed: failed,
        what_we_know: vec![
            "ISP write started; host timeline shows a stall before MEMOP END.".into(),
        ],
        not_proven: not,
        certainty: vec![
            "HIGH — write did not complete cleanly on EP2 timeline".into(),
            "LOW — which layer dropped (USB / cable / target)".into(),
        ],
    }
}

fn fail_enableprog(line: bool, ev: &EvidenceRecord) -> Explanation {
    let ladder = ev.execution.ep_attempts >= 2 && ev.execution.sck_ids.len() >= 2;
    let mut failed = vec!["Target did not enter programming mode (ENABLEPROG FAIL).".into()];
    if ladder {
        failed.push("ENABLEPROG failed at multiple recorded SCK speeds.".into());
    }
    let mut know = vec![
        "USBasp communicated with the host.".into(),
        "An ISP programming-enable transaction was attempted.".into(),
    ];
    if line {
        know.push("An RST GPIO echo anomaly was also observed (plausible contributor only).".into());
    }
    Explanation {
        schema: EXPLANATION_SCHEMA,
        tone: Tone::Fail,
        headline: "PROGRAMMING FAILED".into(),
        subhead: "ROOT CAUSE NOT CONFIRMED".into(),
        why: vec![],
        why_warning: vec![],
        why_not_failure: vec![],
        why_failed: failed,
        what_we_know: know,
        not_proven: vec![
            "Physical RESET / SCK / MISO / power / clock — not independently captured.".into(),
            "Which single cause among the ISP path — not selected.".into(),
        ],
        certainty: vec![
            "HIGH — programming mode was not entered".into(),
            "LOW — physical root cause".into(),
        ],
    }
}

fn fail_generic(analysis: &Analysis) -> Explanation {
    Explanation {
        schema: EXPLANATION_SCHEMA,
        tone: Tone::Fail,
        headline: "PROGRAMMING FAILED".into(),
        subhead: "ROOT CAUSE NOT CONFIRMED".into(),
        why: vec![],
        why_warning: vec![],
        why_not_failure: vec![],
        why_failed: vec![analysis.verdict.likely.clone()],
        what_we_know: analysis.verdict.supported_by.clone(),
        not_proven: analysis.verdict.not_proven.clone(),
        certainty: vec!["MEDIUM — session failed; cause unconfirmed".into()],
    }
}

fn fail_confirmed(analysis: &Analysis) -> Explanation {
    Explanation {
        schema: EXPLANATION_SCHEMA,
        tone: Tone::Fail,
        headline: "PROGRAMMING FAILED".into(),
        subhead: "CAUSE CONFIRMED BY INDEPENDENT EVIDENCE".into(),
        why: vec![],
        why_warning: vec![],
        why_not_failure: vec![],
        why_failed: vec![analysis.verdict.likely.clone()],
        what_we_know: analysis.verdict.supported_by.clone(),
        not_proven: analysis.verdict.not_proven.clone(),
        certainty: vec!["HIGH — independent sources agree on the failing claim".into()],
    }
}

fn inconclusive(line: bool, ep_pass: bool) -> Explanation {
    let mut know = Vec::new();
    if line {
        know.push("RST GPIO echo anomaly only — no ENABLEPROG outcome to judge the session.".into());
    }
    if ep_pass {
        know.push("ENABLEPROG was observed but the ISP session is not closed / incomplete.".into());
    }
    Explanation {
        schema: EXPLANATION_SCHEMA,
        tone: Tone::Info,
        headline: "SESSION INCONCLUSIVE".into(),
        subhead: "NOT ENOUGH EVIDENCE FOR A SESSION OUTCOME".into(),
        why: vec![],
        why_warning: if line {
            vec!["RESET GPIO observation is anomalous but not a verdict by itself.".into()]
        } else {
            vec![]
        },
        why_not_failure: vec![],
        why_failed: vec![],
        what_we_know: know,
        not_proven: vec!["Final programming outcome.".into(), "Physical line state.".into()],
        certainty: vec!["LOW — incomplete timeline".into()],
    }
}
