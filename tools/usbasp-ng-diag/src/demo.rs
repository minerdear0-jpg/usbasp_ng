//! Synthetic demo scenarios (no hardware).

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

pub fn list_scenarios() -> &'static [&'static str] {
    &["enableprog_fail_sw", "memop_flash", "overflow", "session_hw_pass"]
}

pub fn build_scenario(name: &str) -> anyhow::Result<CaptureFile> {
    let mut ns = 1_700_000_000_000_000_000u64;
    let mut records = Vec::new();
    match name {
        "enableprog_fail_sw" => {
            push(&mut records, &mut ns, report(HELLO, 0x07, 100, 1, 1));
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 101, 7, 7));
            push(
                &mut records,
                &mut ns,
                report(SCK_CONFIG, 0, 102, 7, TRANSPORT_SW),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 103, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ERROR, ERR_EP_AVR, 110, 0xff, 3),
            );
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_START, 120, 0xac, 0x53),
            );
            push(&mut records, &mut ns, report(ENABLEPROG, EP_CONT, 121, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_CONT, 122, 0xff, 0xff),
            );
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_END | EP_FAIL, 123, 0xff, 0xff),
            );
            push(
                &mut records,
                &mut ns,
                report(FAULT_SNAPSHOT, EP_START, 130, (7 << 4) | 7, TRANSPORT_SW),
            );
            push(
                &mut records,
                &mut ns,
                report(FAULT_SNAPSHOT, EP_CONT, 131, 0x01, 0x10),
            );
            push(
                &mut records,
                &mut ns,
                report(FAULT_SNAPSHOT, EP_CONT, 132, 0xac, 0x53),
            );
            push(
                &mut records,
                &mut ns,
                report(FAULT_SNAPSHOT, EP_END | EP_FAIL, 133, 0xff, 6),
            );
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 140, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 141, 0, 0));
        }
        "memop_flash" => {
            push(&mut records, &mut ns, report(HELLO, 0x07, 10, 1, 1));
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 11, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(SCK_CONFIG, 0, 12, 8, TRANSPORT_HW),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 13, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_START, 20, 0xac, 0x53),
            );
            push(&mut records, &mut ns, report(ENABLEPROG, EP_CONT, 21, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_CONT, 22, 0xff, 0xff),
            );
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_END | EP_OK, 23, 0x53, 0),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_START, 30, MEM_FLASH, 128),
            );
            for p in 1u8..=4 {
                push(
                    &mut records,
                    &mut ns,
                    report(MEMOP, EP_END | EP_OK, 30 + p as u16, MEM_FLASH, p),
                );
            }
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 40, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 41, 0, 0));
        }
        "overflow" => {
            push(&mut records, &mut ns, report(HELLO, 0x07, 1, 1, 1));
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 2, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(TRACE_OVERFLOW, 0, 3, 20, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 4, 0, 0));
        }
        "session_hw_pass" => {
            push(&mut records, &mut ns, report(HELLO, 0x07, 50, 1, 1));
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 51, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(SCK_CONFIG, 0, 52, 8, TRANSPORT_HW),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 53, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_START, 60, 0xac, 0x53),
            );
            push(&mut records, &mut ns, report(ENABLEPROG, EP_CONT, 61, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_CONT, 62, 0xff, 0xff),
            );
            push(
                &mut records,
                &mut ns,
                report(ENABLEPROG, EP_END | EP_OK, 63, 0x53, 0),
            );
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 70, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 71, 0, 0));
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
