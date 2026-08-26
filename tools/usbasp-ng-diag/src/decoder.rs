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
}
