//! Synthetic demo scenarios (no hardware).

use crate::caps::{caps_reports, YEL0_BCAP, YEL0_FCAP};
use crate::capture::{CaptureFile, CaptureHeader, CaptureRecord};
use crate::protocol::*;

fn report(ty: u8, flags: u8, tick: u16, a: u8, b: u8) -> [u8; 8] {
    [
        ty,
        flags,
        (tick & 0xff) as u8,
        (tick >> 8) as u8,
        a,
        b,
        0,
        0,
    ]
}

fn push(recs: &mut Vec<CaptureRecord>, ns: &mut u64, rep: [u8; 8]) {
    recs.push(CaptureRecord {
        host_ns: *ns,
        report: rep,
    });
    *ns += 1_000_000; // 1 ms steps
}

fn push_hello_caps(recs: &mut Vec<CaptureRecord>, ns: &mut u64, tick: u16) {
    push(recs, ns, report(HELLO, HELLO_CAPS_YEL0, tick, 1, 1));
    for rep in caps_reports(YEL0_FCAP, YEL0_BCAP, tick + 1) {
        push(recs, ns, rep);
    }
}

pub fn list_scenarios() -> &'static [&'static str] {
    &[
        "enableprog_fail_sw",
        "memop_flash",
        "overflow",
        "session_hw_pass",
        "capabilities_yel0",
    ]
}

pub fn build_scenario(name: &str) -> anyhow::Result<CaptureFile> {
    let mut ns = 1_700_000_000_000_000_000u64;
    let mut records = Vec::new();
    match name {
        "enableprog_fail_sw" => {
            push_hello_caps(&mut records, &mut ns, 100);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 110, 7, 7));
            push(
                &mut records,
                &mut ns,
                report(SCK_CONFIG, 0, 111, 7, TRANSPORT_SW),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 112, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ERROR, ERR_EP_AVR, 120, 0xff, 3),
            );
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_START, 130, 0xac, 0x53),
            );
            push(&mut records, &mut ns, report(ENABLEPROG, EP_CONT, 131, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_CONT, 132, 0xff, 0xff),
            );
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_END | EP_FAIL, 133, 0xff, 0xff),
            );
            push(
                &mut records,
                &mut ns,
                report(FAULT_SNAPSHOT, EP_START, 140, (7 << 4) | 7, TRANSPORT_SW),
            );
            push(
                &mut records,
                &mut ns,
                report(FAULT_SNAPSHOT, EP_CONT, 141, 0x01, 0x10),
            );
            push(
                &mut records,
                &mut ns,
                report(FAULT_SNAPSHOT, EP_CONT, 142, 0xac, 0x53),
            );
            push(
                &mut records,
                &mut ns,
                report(FAULT_SNAPSHOT, EP_END | EP_FAIL, 143, 0xff, 6),
            );
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 150, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 151, 0, 0));
        }
        "memop_flash" => {
            push_hello_caps(&mut records, &mut ns, 10);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 20, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(SCK_CONFIG, 0, 21, 8, TRANSPORT_HW),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 22, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_START, 30, 0xac, 0x53),
            );
            push(&mut records, &mut ns, report(ENABLEPROG, EP_CONT, 31, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_CONT, 32, 0xff, 0xff),
            );
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_END | EP_OK, 33, 0x53, 0),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_START, 40, MEM_FLASH, 128),
            );
            for p in 1u8..=4 {
                push(
                    &mut records,
                    &mut ns,
                    report(MEMOP, EP_END | EP_OK, 40 + p as u16, MEM_FLASH, p),
                );
            }
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 50, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 51, 0, 0));
        }
        "overflow" => {
            push_hello_caps(&mut records, &mut ns, 1);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 10, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(TRACE_OVERFLOW, 0, 11, 20, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 12, 0, 0));
        }
        "session_hw_pass" => {
            push_hello_caps(&mut records, &mut ns, 50);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 60, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(SCK_CONFIG, 0, 61, 8, TRANSPORT_HW),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 62, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_START, 70, 0xac, 0x53),
            );
            push(&mut records, &mut ns, report(ENABLEPROG, EP_CONT, 71, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_CONT, 72, 0xff, 0xff),
            );
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_END | EP_OK, 73, 0x53, 0),
            );
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 80, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 81, 0, 0));
        }
        "capabilities_yel0" => {
            push_hello_caps(&mut records, &mut ns, 1);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 10, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(SCK_CONFIG, 0, 11, 8, TRANSPORT_HW),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 12, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 13, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 14, 0, 0));
        }
        other => anyhow::bail!(
            "unknown scenario {other:?}; try: {}",
            list_scenarios().join(", ")
        ),
    }
    Ok(CaptureFile {
        header: Some(CaptureHeader::default()),
        records,
    })
}
