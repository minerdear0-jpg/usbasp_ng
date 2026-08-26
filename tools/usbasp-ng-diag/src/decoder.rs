use crate::protocol::*;

pub fn type_name(ty: u8) -> &'static str {
    match ty {
        HELLO => "HELLO",
        SESSION_BEGIN => "SESSION_BEGIN",
        SESSION_END => "SESSION_END",
        RESET => "RESET",
        SCK_CONFIG => "SCK_CONFIG",
        ENABLEPROG => "ENABLEPROG",
        FAULT_SNAPSHOT => "FAULT_SNAPSHOT",
        TRACE_OVERFLOW => "TRACE_OVERFLOW",
        ERROR => "ERROR",
        MEMOP => "MEMOP",
        CAPS => "CAPS",
        TRACE_BEGIN => "TRACE_BEGIN",
        TRACE_END => "TRACE_END",
        _ => "UNKNOWN",
    }
}

pub fn format_frame(f: &DiagFrame) -> String {
    let mut extra = String::new();
    match f.ty {
        HELLO => {
            extra = format!(" schema={} profile={} caps=0x{:02x}", f.a, f.b, f.flags);
        }
        RESET => {
            if f.flags & RESET_ASSERT != 0 {
                extra.push_str(" ASSERT");
            }
            if f.flags & RESET_RELEASE != 0 {
                extra.push_str(" RELEASE");
            }
        }
        SCK_CONFIG => {
            let tr = if f.b == TRANSPORT_SW { "SW" } else { "HW" };
            extra = format!(" sck_id={} transport={}", f.a, tr);
        }
        ENABLEPROG => {
            extra = format!(" {} data={:02x}{:02x}", seq_flags(f.flags), f.a, f.b);
        }
        FAULT_SNAPSHOT => {
            extra = format!(" {} data={:02x}{:02x}", seq_flags(f.flags), f.a, f.b);
            if f.flags & EP_START != 0 {
                let tr = if f.b == TRANSPORT_SW { "SW" } else { "HW" };
                extra.push_str(&format!(
                    " sck_req={} eff={} transport={}",
                    f.a >> 4,
                    f.a & 0x0f,
                    tr
                ));
            } else if f.flags & EP_END != 0 {
                let res = if f.flags & EP_FAIL != 0 {
                    "FAIL"
                } else if f.flags & EP_OK != 0 {
                    "OK"
                } else {
                    "?"
                };
                extra.push_str(&format!(
                    " rx0=0x{:02x} sw_delay={} {res}",
                    f.a, f.b
                ));
            }
        }
        TRACE_OVERFLOW => {
            extra = format!(" dropped={}", f.a);
        }
        ERROR => {
            let path = if f.flags & ERR_EP_AVR != 0 {
                "AVR"
            } else if f.flags & ERR_EP_AT89 != 0 {
                "AT89"
            } else {
                "?"
            };
            extra = format!(" try={path} check=0x{:02x} sw_delay={}", f.a, f.b);
        }
        MEMOP => {
            let mem = match f.a {
                MEM_FLASH => "FLASH",
                MEM_EEPROM => "EEPROM",
                MEM_READFLASH => "READFLASH",
                _ => "?",
            };
            if f.flags & EP_START != 0 {
                extra = format!(" START {mem} pagesize={}", f.b);
            } else if f.flags & EP_END != 0 {
                extra = format!(" END {mem} pages={}", f.b);
            } else {
                extra = format!(" {} {mem} b={}", seq_flags(f.flags), f.b);
            }
        }
        CAPS => {
            extra = format!(" {} data={:02x}{:02x}", seq_flags(f.flags), f.a, f.b);
        }
        TRACE_BEGIN => {
            let state = f.flags & 0x0f;
            let ts_mode = f.flags >> 4;
            extra = format!(
                " slots={} frame_size={} state={} ts_mode={}",
                f.a, f.b, state, ts_mode
            );
        }
        TRACE_END => {
            if f.flags & EP_END != 0 {
                let wi = u16::from_le_bytes([f.a, f.b]);
                let ov = if f.flags & 0x80 != 0 { "YES" } else { "no" };
                extra = format!(" END write_index={wi} overflow={ov}");
            } else if f.flags & EP_START != 0 {
                let valid = u16::from_le_bytes([f.a, f.b]);
                extra = format!(" START valid={valid}");
            } else {
                extra = format!(" {} data={:02x}{:02x}", seq_flags(f.flags), f.a, f.b);
            }
        }
        _ => {}
    }
    format!(
        "t={:5} {:16} flags=0x{:02x} a={:02x} b={:02x}{extra}",
        f.timestamp,
        type_name(f.ty),
        f.flags,
        f.a,
        f.b
    )
}

fn seq_flags(flags: u8) -> String {
    let mut p = Vec::new();
    if flags & EP_START != 0 {
        p.push("START");
    }
    if flags & EP_CONT != 0 {
        p.push("CONT");
    }
    if flags & EP_END != 0 {
        p.push("END");
    }
    if flags & EP_OK != 0 {
        p.push("OK");
    }
    if flags & EP_FAIL != 0 {
        p.push("FAIL");
    }
    if p.is_empty() {
        format!("0x{flags:02x}")
    } else {
        p.join("|")
    }
}

/// Reassemble four ENABLEPROG frames into one human line.
pub fn reassemble_enableprog(frames: &[DiagFrame]) -> Option<String> {
    if frames.len() != 4 {
        return None;
    }
    if frames[0].flags & EP_START == 0 || frames[3].flags & EP_END == 0 {
        return None;
    }
    let tx = [frames[0].a, frames[0].b, frames[1].a, frames[1].b];
    let rx = [frames[2].a, frames[2].b, frames[3].a, frames[3].b];
    let result = if frames[3].flags & EP_OK != 0 {
        "PASS"
    } else if frames[3].flags & EP_FAIL != 0 {
        "FAIL"
    } else {
        "?"
    };
    Some(format!(
        "ENABLEPROG  TX {:02X} {:02X} {:02X} {:02X}  RX {:02X} {:02X} {:02X} {:02X}  {result}",
        tx[0], tx[1], tx[2], tx[3], rx[0], rx[1], rx[2], rx[3]
    ))
}

/// Reassemble four compact FAULT_SNAPSHOT frames into one human line.
pub fn reassemble_fault_snapshot(frames: &[DiagFrame]) -> Option<String> {
    if frames.len() != 4 {
        return None;
    }
    if frames[0].flags & EP_START == 0 || frames[3].flags & EP_END == 0 {
        return None;
    }
    let packed = frames[0].a;
    let tr = if frames[0].b == TRANSPORT_SW {
        "SW"
    } else {
        "HW"
    };
    let end = frames[3].flags;
    let result = if end & EP_FAIL != 0 {
        "FAIL"
    } else if end & EP_OK != 0 {
        "OK"
    } else {
        "?"
    };
    Some(format!(
        "FAULT_SNAPSHOT  sck_req={} eff={} transport={} reset=0x{:02x} state=0x{:02x} tx={:02x}{:02x}.. rx0=0x{:02x} sw_delay={} {result}",
        packed >> 4,
        packed & 0x0f,
        tr,
        frames[1].a,
        frames[1].b,
        frames[2].a,
        frames[2].b,
        frames[3].a,
        frames[3].b
    ))
}

/// Reassemble TRACE_END pair into one human line.
pub fn reassemble_trace_end(frames: &[DiagFrame]) -> Option<String> {
    if frames.len() != 2 {
        return None;
    }
    if frames[0].flags & EP_START == 0 || frames[1].flags & EP_END == 0 {
        return None;
    }
    let valid = u16::from_le_bytes([frames[0].a, frames[0].b]);
    let wi = u16::from_le_bytes([frames[1].a, frames[1].b]);
    let ov = frames[1].flags & 0x80 != 0;
    Some(format!(
        "TRACE_END  valid={valid}  write_index={wi}  overflow={}",
        if ov { "YES" } else { "no" }
    ))
}

/// Reassemble four DIAG_CAPS frames into one human line.
pub fn reassemble_caps(frames: &[DiagFrame]) -> Option<String> {
    crate::caps::CapsAdvert::from_frames(frames).map(|adv| {
        format!(
            "CAPS  firmware=0x{:08x}  board=0x{:08x}  ({})",
            adv.firmware.0,
            adv.board.0,
            adv.summary_line()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enableprog_fail_line() {
        let frames = [
            DiagFrame {
                ty: ENABLEPROG,
                flags: EP_START,
                timestamp: 0,
                a: 0xac,
                b: 0x53,
            },
            DiagFrame {
                ty: ENABLEPROG,
                flags: EP_CONT,
                timestamp: 0,
                a: 0,
                b: 0,
            },
            DiagFrame {
                ty: ENABLEPROG,
                flags: EP_CONT,
                timestamp: 0,
                a: 0xc1,
                b: 0xff,
            },
            DiagFrame {
                ty: ENABLEPROG,
                flags: EP_END | EP_FAIL,
                timestamp: 0,
                a: 0xff,
                b: 0xff,
            },
        ];
        let s = reassemble_enableprog(&frames).unwrap();
        assert!(s.contains("FAIL"));
        assert!(s.contains("C1"));
    }

    #[test]
    fn fault_snapshot_compact() {
        let frames = [
            DiagFrame {
                ty: FAULT_SNAPSHOT,
                flags: EP_START,
                timestamp: 0,
                a: (7 << 4) | 7,
                b: TRANSPORT_SW,
            },
            DiagFrame {
                ty: FAULT_SNAPSHOT,
                flags: EP_CONT,
                timestamp: 0,
                a: 0x01,
                b: 0x10,
            },
            DiagFrame {
                ty: FAULT_SNAPSHOT,
                flags: EP_CONT,
                timestamp: 0,
                a: 0xac,
                b: 0x53,
            },
            DiagFrame {
                ty: FAULT_SNAPSHOT,
                flags: EP_END | EP_FAIL,
                timestamp: 0,
                a: 0xff,
                b: 6,
            },
        ];
        let s = reassemble_fault_snapshot(&frames).unwrap();
        assert!(s.contains("sck_req=7"));
        assert!(s.contains("transport=SW"));
        assert!(s.contains("tx=ac53"));
        assert!(s.contains("sw_delay=6"));
        assert!(s.contains("FAIL"));
    }

    #[test]
    fn caps_reassemble() {
        let reps = crate::caps::caps_reports(crate::caps::YEL0_FCAP, crate::caps::YEL0_BCAP, 0);
        let frames: Vec<_> = reps
            .iter()
            .map(|r| DiagFrame::from_report(r).unwrap())
            .collect();
        let s = reassemble_caps(&frames).unwrap();
        assert!(s.contains("firmware=0x0000000f"));
        assert!(s.contains("board=0x00000002"));
        assert!(s.contains("TRACE"));
    }
}
