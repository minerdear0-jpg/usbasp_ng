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
    // TRACE_BEGIN: slots=64, frame_size=6, state=ARMED, ts_mode=0
    push(
        recs,
        ns,
        report(TRACE_BEGIN, 0x01, tick + 5, 64, 6),
    );
}

fn push_trace_end(
    recs: &mut Vec<CaptureRecord>,
    ns: &mut u64,
    tick: u16,
    valid: u16,
    write_index: u16,
    overflow: bool,
    triggered: bool,
    trigger_kind: u8,
    post_count: u8,
    trigger_index: u16,
    trigger_t: u16,
) {
    let vb = valid.to_le_bytes();
    let wb = write_index.to_le_bytes();
    let ib = trigger_index.to_le_bytes();
    push(
        recs,
        ns,
        report(TRACE_END, EP_START, tick, vb[0], vb[1]),
    );
    let mut fl1 = EP_CONT;
    if overflow {
        fl1 |= 0x80;
    }
    push(recs, ns, report(TRACE_END, fl1, tick + 1, wb[0], wb[1]));
    let mut fl2 = EP_CONT;
    if triggered {
        fl2 |= 0x80;
    }
    push(
        recs,
        ns,
        report(TRACE_END, fl2, tick + 2, trigger_kind, post_count),
    );
    push(
        recs,
        ns,
        report(TRACE_END, EP_END, trigger_t, ib[0], ib[1]),
    );
}

pub fn list_scenarios() -> &'static [&'static str] {
    &[
        "enableprog_fail_sw",
        "enableprog_fail_line_anomaly",
        "enableprog_ladder_silent",
        "line_fault_rst",
        "pass_with_rst_anomaly",
        "memop_flash",
        "memop_poll_fail",
        "flash_abort_line_anomaly",
        "flash_poll_fail_end_ok",
        "overflow",
        "session_hw_pass",
        "capabilities_yel0",
    ]
}

/// Canonical ISP sessions for replay and determinism. Not CAPS-only HELLO.
pub fn replay_corpus() -> &'static [&'static str] {
    &[
        "memop_flash",
        "pass_with_rst_anomaly",
        "enableprog_fail_sw",
        "enableprog_fail_line_anomaly",
        "line_fault_rst",
        "overflow",
        "enableprog_ladder_silent",
        "memop_poll_fail",
        "flash_abort_line_anomaly",
        "flash_poll_fail_end_ok",
        "session_hw_pass",
    ]
}

pub fn build_scenario(name: &str) -> anyhow::Result<CaptureFile> {
    let mut ns = 1_700_000_000_000_000_000u64;
    let mut records = Vec::new();
    match name {
        "enableprog_fail_sw" | "enableprog_fail_line_anomaly" => {
            push_hello_caps(&mut records, &mut ns, 100);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 110, 7, 7));
            if name == "enableprog_fail_line_anomaly" {
                push(
                    &mut records,
                    &mut ns,
                    report(
                        LINE_FAULT,
                        LINE_DRIVE_HIGH | EP_FAIL,
                        110,
                        2,
                        0x14,
                    ),
                );
            }
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
            push_trace_end(
                &mut records,
                &mut ns,
                151,
                20,
                24,
                false,
                true,
                3,
                8,
                18,
                133,
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 160, 0, 0));
        }
        "enableprog_ladder_silent" => {
            push_hello_caps(&mut records, &mut ns, 100);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 110, 7, 7));
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 112, 0, 0));
            for (i, id) in [8u8, 4, 0].iter().copied().enumerate() {
                let t = 120 + (i as u16) * 10;
                let tr = if id == 0 { TRANSPORT_SW } else { TRANSPORT_HW };
                push(&mut records, &mut ns, report(SCK_CONFIG, 0, t, id, tr));
                push(
                    &mut records,
                    &mut ns,
                    report(ENABLEPROG, EP_START, t + 1, 0xac, 0x53),
                );
                push(
                    &mut records,
                    &mut ns,
                    report(ENABLEPROG, EP_CONT, t + 2, 0, 0),
                );
                push(
                    &mut records,
                    &mut ns,
                    report(ENABLEPROG, EP_CONT, t + 3, 0xff, 0xff),
                );
                push(
                    &mut records,
                    &mut ns,
                    report(ENABLEPROG, EP_END | EP_FAIL, t + 4, 0xff, 0xff),
                );
            }
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 160, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 161, 0, 0));
        }
        "line_fault_rst" => {
            push_hello_caps(&mut records, &mut ns, 100);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 110, 7, 7));
            push(
                &mut records,
                &mut ns,
                report(
                    LINE_FAULT,
                    LINE_DRIVE_HIGH | EP_FAIL,
                    111,
                    2,
                    0x00,
                ),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 112, 0, 0));
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 150, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 160, 0, 0));
        }
        "memop_flash" | "pass_with_rst_anomaly" => {
            push_hello_caps(&mut records, &mut ns, 10);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 20, 8, 8));
            if name == "pass_with_rst_anomaly" {
                push(
                    &mut records,
                    &mut ns,
                    report(
                        LINE_FAULT,
                        LINE_DRIVE_HIGH | EP_FAIL,
                        21,
                        2,
                        0x14,
                    ),
                );
            }
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
            for p in 0u8..4 {
                let addr = (p as u16) * 128;
                push(
                    &mut records,
                    &mut ns,
                    report(
                        MEMOP,
                        EP_CONT | EP_OK,
                        41 + p as u16,
                        (addr >> 8) as u8,
                        (addr & 0xff) as u8,
                    ),
                );
            }
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_END | EP_OK, 45, MEM_FLASH, 4),
            );
            /* Verify reads (coalesced READFLASH) — closes dangling write in firmware. */
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_START, 46, MEM_READFLASH, 128),
            );
            for p in 0u8..4 {
                let addr = (p as u16) * 128;
                push(
                    &mut records,
                    &mut ns,
                    report(
                        MEMOP,
                        EP_CONT | EP_OK,
                        47 + p as u16,
                        (addr >> 8) as u8,
                        (addr & 0xff) as u8,
                    ),
                );
            }
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_END | EP_OK, 51, MEM_READFLASH, 4),
            );
            push_trace_end(
                &mut records,
                &mut ns,
                52,
                18,
                22,
                false,
                false,
                0,
                0,
                0,
                0,
            );
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 56, 0, 0),
            );
            push(
                &mut records,
                &mut ns,
                report(ISP_PINS, PINS_AFTER_DISC | EP_OK, 57, 0x00, 0x00),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 60, 0, 0));
        }
        "memop_poll_fail" => {
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
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_CONT | EP_FAIL, 41, 0x04, 0x00),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_END | EP_FAIL, 42, MEM_FLASH, 1),
            );
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 50, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 51, 0, 0));
        }
        "flash_abort_line_anomaly" => {
            // ENABLEPROG PASS + LINE anomaly + CONT pages, no MEMOP END — must not be PASS.
            push_hello_caps(&mut records, &mut ns, 10);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 20, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(
                    LINE_FAULT,
                    LINE_DRIVE_HIGH | EP_FAIL,
                    21,
                    2,
                    0x14,
                ),
            );
            push(
                &mut records,
                &mut ns,
                report(SCK_CONFIG, 0, 22, 8, TRANSPORT_HW),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 23, 0, 0));
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
                report(MEMOP, EP_START, 40, MEM_FLASH, 64),
            );
            for p in 0u8..23 {
                let addr = (p as u16) * 64;
                push(
                    &mut records,
                    &mut ns,
                    report(
                        MEMOP,
                        EP_CONT | EP_OK,
                        41 + p as u16,
                        (addr >> 8) as u8,
                        (addr & 0xff) as u8,
                    ),
                );
            }
            // USB dies: no MEMOP END. Session may still close on host teardown.
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 80, 0, 0),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 81, 0, 0));
        }
        "flash_poll_fail_end_ok" => {
            // Ribbon tear: CONT|FAIL then firmware still emits END|OK + READFLASH.
            // Host must NOT paint PASS (sticky poll fail survives READFLASH START).
            push_hello_caps(&mut records, &mut ns, 10);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 20, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(
                    LINE_FAULT,
                    LINE_DRIVE_HIGH | EP_FAIL,
                    21,
                    2,
                    0x14,
                ),
            );
            push(
                &mut records,
                &mut ns,
                report(SCK_CONFIG, 0, 22, 8, TRANSPORT_HW),
            );
            push(&mut records, &mut ns, report(RESET, RESET_ASSERT, 23, 0, 0));
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
                report(MEMOP, EP_START, 40, MEM_FLASH, 64),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_CONT | EP_OK, 41, 0x10, 0x00),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_CONT | EP_FAIL, 42, 0x11, 0xc0),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_CONT | EP_FAIL, 43, 0x12, 0x00),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_END | EP_OK, 44, MEM_FLASH, 128),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_START, 50, MEM_READFLASH, 64),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_CONT | EP_OK, 51, 0x00, 0x00),
            );
            push(
                &mut records,
                &mut ns,
                report(MEMOP, EP_END | EP_OK, 52, MEM_READFLASH, 1),
            );
            push(
                &mut records,
                &mut ns,
                report(RESET, RESET_RELEASE, 60, 0, 0),
            );
            push(
                &mut records,
                &mut ns,
                report(ISP_PINS, PINS_AFTER_DISC | EP_OK, 61, 0x00, 0x00),
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 62, 0, 0));
        }
        "overflow" => {
            push_hello_caps(&mut records, &mut ns, 1);
            push(&mut records, &mut ns, report(SESSION_BEGIN, 0, 10, 8, 8));
            push(
                &mut records,
                &mut ns,
                report(TRACE_OVERFLOW, 0, 11, 20, 0),
            );
            push_trace_end(
                &mut records,
                &mut ns,
                12,
                64,
                90,
                true,
                false,
                0,
                0,
                0,
                0,
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 14, 0, 0));
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
            push_trace_end(
                &mut records,
                &mut ns,
                81,
                16,
                20,
                false,
                false,
                0,
                0,
                0,
                0,
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 83, 0, 0));
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
            push_trace_end(
                &mut records,
                &mut ns,
                14,
                12,
                14,
                false,
                false,
                0,
                0,
                0,
                0,
            );
            push(&mut records, &mut ns, report(SESSION_END, 0, 16, 0, 0));
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
