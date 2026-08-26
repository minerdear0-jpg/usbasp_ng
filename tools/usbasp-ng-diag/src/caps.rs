//! Firmware and board capability bitsets (DIAG_CAPS).
//!
//! Hosts gate features on these masks — never on firmware version alone.

#![allow(dead_code)]

use crate::protocol::*;

/// Firmware diagnostics capabilities (`uint32` LE, DIAG_CAPS frames 0..1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiagCaps(pub u32);

impl DiagCaps {
    pub const SESSION: u32 = 1 << 0;
    pub const SNAPSHOT: u32 = 1 << 1;
    pub const TIMESTAMP: u32 = 1 << 2;
    pub const TRACE: u32 = 1 << 3;
    pub const TRIGGER: u32 = 1 << 4;
    pub const PRETRIGGER: u32 = 1 << 5;
    pub const SCK_STATS: u32 = 1 << 6;

    pub fn contains(self, bit: u32) -> bool {
        self.0 & bit != 0
    }

    pub fn mark(yes: bool) -> &'static str {
        if yes {
            "✓"
        } else {
            "✗"
        }
    }
}

/// Board / physical capabilities (`uint32` LE, DIAG_CAPS frames 2..3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoardCaps(pub u32);

impl BoardCaps {
    pub const TARGET_UART: u32 = 1 << 0;
    pub const SCK_JUMPER: u32 = 1 << 1;
    pub const PHYSICAL_CAPTURE: u32 = 1 << 2;

    pub fn contains(self, bit: u32) -> bool {
        self.0 & bit != 0
    }
}

/// Parsed capability advertisement from four DIAG_CAPS frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapsAdvert {
    pub firmware: DiagCaps,
    pub board: BoardCaps,
}

impl CapsAdvert {
    pub fn from_frames(frames: &[DiagFrame]) -> Option<Self> {
        if frames.len() != 4 {
            return None;
        }
        if frames[0].flags & EP_START == 0 || frames[3].flags & EP_END == 0 {
            return None;
        }
        let fcap = u32::from_le_bytes([frames[0].a, frames[0].b, frames[1].a, frames[1].b]);
        let bcap = u32::from_le_bytes([frames[2].a, frames[2].b, frames[3].a, frames[3].b]);
        Some(Self {
            firmware: DiagCaps(fcap),
            board: BoardCaps(bcap),
        })
    }

    /// Acceptance-style text for CLI / TUI (YEL0-shaped today).
    pub fn format_report(&self, schema_note: &str) -> String {
        let f = self.firmware;
        let b = self.board;
        let mut lines = Vec::new();
        lines.push(schema_note.to_string());
        lines.push(String::new());
        lines.push("Capabilities:".into());
        lines.push(format!(
            "    {} SESSION",
            DiagCaps::mark(f.contains(DiagCaps::SESSION))
        ));
        lines.push(format!(
            "    {} SNAPSHOT",
            DiagCaps::mark(f.contains(DiagCaps::SNAPSHOT))
        ));
        lines.push(format!(
            "    {} TIMESTAMP",
            DiagCaps::mark(f.contains(DiagCaps::TIMESTAMP))
        ));
        lines.push(format!(
            "    {} TRACE",
            DiagCaps::mark(f.contains(DiagCaps::TRACE))
        ));
        lines.push(format!(
            "    {} TRIGGER",
            DiagCaps::mark(f.contains(DiagCaps::TRIGGER))
        ));
        lines.push(format!(
            "    {} PRETRIGGER",
            DiagCaps::mark(f.contains(DiagCaps::PRETRIGGER))
        ));
        lines.push(format!(
            "    {} SCK_STATS",
            DiagCaps::mark(f.contains(DiagCaps::SCK_STATS))
        ));
        lines.push(String::new());
        lines.push("Board:".into());
        lines.push(format!(
            "    target UART       {}",
            DiagCaps::mark(b.contains(BoardCaps::TARGET_UART))
        ));
        lines.push(format!(
            "    sck jumper        {}",
            DiagCaps::mark(b.contains(BoardCaps::SCK_JUMPER))
        ));
        lines.push(format!(
            "    physical capture  {}",
            DiagCaps::mark(b.contains(BoardCaps::PHYSICAL_CAPTURE))
        ));
        lines.push(String::new());
        lines.push(format!(
            "raw: firmware=0x{:08x}  board=0x{:08x}",
            f.0, b.0
        ));
        lines.join("\n")
    }

    pub fn summary_line(&self) -> String {
        let mut diag = Vec::new();
        let f = self.firmware;
        if f.contains(DiagCaps::SESSION) {
            diag.push("SESSION");
        }
        if f.contains(DiagCaps::SNAPSHOT) {
            diag.push("SNAPSHOT");
        }
        if f.contains(DiagCaps::TIMESTAMP) {
            diag.push("TIMESTAMP");
        }
        if f.contains(DiagCaps::TRACE) {
            diag.push("TRACE");
        }
        if f.contains(DiagCaps::TRIGGER) {
            diag.push("TRIGGER");
        }
        if f.contains(DiagCaps::PRETRIGGER) {
            diag.push("PRETRIGGER");
        }
        if f.contains(DiagCaps::SCK_STATS) {
            diag.push("SCK_STATS");
        }
        let mut board = Vec::new();
        let b = self.board;
        if b.contains(BoardCaps::TARGET_UART) {
            board.push("TARGET_UART");
        }
        if b.contains(BoardCaps::SCK_JUMPER) {
            board.push("SCK_JUMPER");
        }
        if b.contains(BoardCaps::PHYSICAL_CAPTURE) {
            board.push("PHYSICAL_CAPTURE");
        }
        format!(
            "DIAG [{}]  BOARD [{}]",
            if diag.is_empty() {
                "—".into()
            } else {
                diag.join("+")
            },
            if board.is_empty() {
                "—".into()
            } else {
                board.join("+")
            }
        )
    }
}

/// Current USBasp2 / YEL0 advertisement (TRACE ring on; no trigger yet).
pub const YEL0_FCAP: u32 =
    DiagCaps::SESSION | DiagCaps::SNAPSHOT | DiagCaps::TIMESTAMP | DiagCaps::TRACE;
pub const YEL0_BCAP: u32 = BoardCaps::SCK_JUMPER;

/// Pack four DIAG_CAPS report payloads (8 bytes each) for demos / goldens.
pub fn caps_reports(fcap: u32, bcap: u32, tick0: u16) -> [[u8; 8]; 4] {
    let fb = fcap.to_le_bytes();
    let bb = bcap.to_le_bytes();
    [
        [
            CAPS,
            EP_START,
            (tick0 & 0xff) as u8,
            (tick0 >> 8) as u8,
            fb[0],
            fb[1],
            0,
            0,
        ],
        [
            CAPS,
            EP_CONT,
            ((tick0 + 1) & 0xff) as u8,
            ((tick0 + 1) >> 8) as u8,
            fb[2],
            fb[3],
            0,
            0,
        ],
        [
            CAPS,
            EP_CONT,
            ((tick0 + 2) & 0xff) as u8,
            ((tick0 + 2) >> 8) as u8,
            bb[0],
            bb[1],
            0,
            0,
        ],
        [
            CAPS,
            EP_END,
            ((tick0 + 3) & 0xff) as u8,
            ((tick0 + 3) >> 8) as u8,
            bb[2],
            bb[3],
            0,
            0,
        ],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yel0_masks() {
        let adv = CapsAdvert {
            firmware: DiagCaps(YEL0_FCAP),
            board: BoardCaps(YEL0_BCAP),
        };
        assert!(adv.firmware.contains(DiagCaps::TIMESTAMP));
        assert!(adv.firmware.contains(DiagCaps::TRACE));
        assert!(!adv.firmware.contains(DiagCaps::TRIGGER));
        assert!(adv.board.contains(BoardCaps::SCK_JUMPER));
        assert!(!adv.board.contains(BoardCaps::PHYSICAL_CAPTURE));
        let text = adv.format_report("USBASP2 DIAG v1");
        assert!(text.contains("✓ TIMESTAMP"));
        assert!(text.contains("✓ TRACE"));
        assert!(text.contains("✗ TRIGGER"));
        assert!(text.contains("physical capture  ✗"));
    }

    #[test]
    fn roundtrip_frames() {
        let reps = caps_reports(YEL0_FCAP, YEL0_BCAP, 10);
        let frames: Vec<_> = reps
            .iter()
            .map(|r| DiagFrame::from_report(r).unwrap())
            .collect();
        let adv = CapsAdvert::from_frames(&frames).unwrap();
        assert_eq!(adv.firmware.0, YEL0_FCAP);
        assert_eq!(adv.board.0, YEL0_BCAP);
    }
}
