//! L0 wire constants — USBASP-NG DIAG v1 (must match firmware/include/diag/).

#![allow(dead_code)]

pub const SCHEMA_V1: u8 = 1;

pub const HELLO: u8 = 1;
pub const SESSION_BEGIN: u8 = 2;
pub const SESSION_END: u8 = 3;
pub const RESET: u8 = 4;
pub const SCK_CONFIG: u8 = 5;
pub const ENABLEPROG: u8 = 6;
pub const FAULT_SNAPSHOT: u8 = 9;
pub const TRACE_OVERFLOW: u8 = 10;
pub const ERROR: u8 = 11;
pub const MEMOP: u8 = 12;
pub const CAPS: u8 = 13;

pub const RESET_ASSERT: u8 = 0x01;
pub const RESET_RELEASE: u8 = 0x02;

pub const EP_START: u8 = 0x01;
pub const EP_CONT: u8 = 0x02;
pub const EP_END: u8 = 0x04;
pub const EP_OK: u8 = 0x10;
pub const EP_FAIL: u8 = 0x20;

pub const ERR_EP_AVR: u8 = 0x01;
pub const ERR_EP_AT89: u8 = 0x02;

pub const TRANSPORT_HW: u8 = 0;
pub const TRANSPORT_SW: u8 = 1;

pub const MEM_FLASH: u8 = 0;
pub const MEM_EEPROM: u8 = 1;
pub const MEM_READFLASH: u8 = 2;

/* DIAG_HELLO.flags — legacy compact uint8 */
pub const HELLO_CAP_SESSION: u8 = 0x01;
pub const HELLO_CAP_TRANSACTION: u8 = 0x02;
pub const HELLO_CAP_SNAPSHOT: u8 = 0x04;
pub const HELLO_CAP_TRACE: u8 = 0x08;
pub const HELLO_CAP_SCK_STATS: u8 = 0x10;
pub const HELLO_CAP_TIMESTAMP: u8 = 0x20;

/// HELLO flags for today's USBasp2 (SESSION|TRANSACTION|SNAPSHOT|TIMESTAMP).
pub const HELLO_CAPS_YEL0: u8 =
    HELLO_CAP_SESSION | HELLO_CAP_TRANSACTION | HELLO_CAP_SNAPSHOT | HELLO_CAP_TIMESTAMP;

pub const VID: u16 = 0x16c0;
pub const PID: u16 = 0x05dc;
pub const EP2_IN: u8 = 0x82;
pub const IF_MONITOR: u8 = 2;

/// Six-byte diagnostics payload (bytes 0..5 of the 8-byte HID report).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagFrame {
    pub ty: u8,
    pub flags: u8,
    pub timestamp: u16,
    pub a: u8,
    pub b: u8,
}

impl DiagFrame {
    pub fn from_report(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }
        Some(Self {
            ty: data[0],
            flags: data[1],
            timestamp: u16::from_le_bytes([data[2], data[3]]),
            a: data[4],
            b: data[5],
        })
    }
}
