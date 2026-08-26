//! Versioned capture file I/O (DIAG host records).
//!
//! Header (16 bytes) when present:
//!   0..7   magic `USBDIAGv`
//!   8      format_version (=1)
//!   9      diag_schema (=1)
//!  10      record_size (=16: u64 host_ns LE + 8-byte report)
//!  11      flags (0)
//! 12..15   reserved
//!
//! Legacy lab files have no header — entire blob is records.

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::protocol::DiagFrame;

pub const MAGIC: &[u8; 8] = b"USBDIAGv";
pub const FORMAT_V1: u8 = 1;
pub const SCHEMA_V1: u8 = 1;
pub const RECORD_SIZE: usize = 16;
pub const HEADER_SIZE: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureHeader {
    pub format_version: u8,
    pub diag_schema: u8,
    pub record_size: u8,
    pub flags: u8,
}

impl Default for CaptureHeader {
    fn default() -> Self {
        Self {
            format_version: FORMAT_V1,
            diag_schema: SCHEMA_V1,
            record_size: RECORD_SIZE as u8,
            flags: 0,
        }
    }
}

impl CaptureHeader {
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut out = [0u8; HEADER_SIZE];
        out[0..8].copy_from_slice(MAGIC);
        out[8] = self.format_version;
        out[9] = self.diag_schema;
        out[10] = self.record_size;
        out[11] = self.flags;
        out
    }

    pub fn parse(buf: &[u8]) -> Result<Option<(Self, usize)>> {
        if buf.len() < 8 || &buf[0..8] != MAGIC {
            return Ok(None);
        }
        if buf.len() < HEADER_SIZE {
            bail!("truncated USBDIAGv header");
        }
        let h = Self {
            format_version: buf[8],
            diag_schema: buf[9],
            record_size: buf[10],
            flags: buf[11],
        };
        if h.format_version != FORMAT_V1 {
            bail!("unsupported capture format_version {}", h.format_version);
        }
        if h.record_size as usize != RECORD_SIZE {
            bail!("unsupported record_size {}", h.record_size);
        }
        Ok(Some((h, HEADER_SIZE)))
    }
}

#[derive(Clone, Debug)]
pub struct CaptureRecord {
    pub host_ns: u64,
    pub report: [u8; 8],
}

impl CaptureRecord {
    pub fn frame(&self) -> Option<DiagFrame> {
        DiagFrame::from_report(&self.report)
    }

    pub fn to_bytes(&self) -> [u8; RECORD_SIZE] {
        let mut out = [0u8; RECORD_SIZE];
        out[0..8].copy_from_slice(&self.host_ns.to_le_bytes());
        out[8..16].copy_from_slice(&self.report);
        out
    }
}

#[derive(Clone, Debug)]
pub struct CaptureFile {
    pub header: Option<CaptureHeader>,
    pub records: Vec<CaptureRecord>,
}

impl CaptureFile {
    pub fn load(path: &Path) -> Result<Self> {
        let mut blob = Vec::new();
        File::open(path)
            .with_context(|| format!("open {path:?}"))?
            .read_to_end(&mut blob)?;
        Self::parse_bytes(&blob)
    }

    pub fn parse_bytes(blob: &[u8]) -> Result<Self> {
        let (header, offset) = match CaptureHeader::parse(blob)? {
            Some((h, n)) => (Some(h), n),
            None => (None, 0),
        };
        let body = &blob[offset..];
        if body.len() % RECORD_SIZE != 0 {
            eprintln!(
                "warning: trailing {} bytes after records",
                body.len() % RECORD_SIZE
            );
        }
        let mut records = Vec::with_capacity(body.len() / RECORD_SIZE);
        for chunk in body.chunks_exact(RECORD_SIZE) {
            let host_ns = u64::from_le_bytes(chunk[0..8].try_into().unwrap());
            let mut report = [0u8; 8];
            report.copy_from_slice(&chunk[8..16]);
            records.push(CaptureRecord { host_ns, report });
        }
        Ok(Self { header, records })
    }

    pub fn write(&self, path: &Path, with_header: bool) -> Result<()> {
        let mut f = File::create(path).with_context(|| format!("create {path:?}"))?;
        if with_header {
            let h = self.header.clone().unwrap_or_default();
            f.write_all(&h.to_bytes())?;
        }
        for r in &self.records {
            f.write_all(&r.to_bytes())?;
        }
        Ok(())
    }
}

pub fn write_header<W: Write>(w: &mut W) -> Result<()> {
    w.write_all(&CaptureHeader::default().to_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_header_and_legacy() {
        let mut body = Vec::new();
        body.extend_from_slice(&CaptureHeader::default().to_bytes());
        let rec = CaptureRecord {
            host_ns: 1_000_000,
            report: [1, 7, 100, 0, 1, 1, 0, 0],
        };
        body.extend_from_slice(&rec.to_bytes());
        let cap = CaptureFile::parse_bytes(&body).unwrap();
        assert!(cap.header.is_some());
        assert_eq!(cap.records.len(), 1);
        assert_eq!(cap.records[0].host_ns, 1_000_000);

        let legacy = CaptureFile::parse_bytes(&rec.to_bytes()).unwrap();
        assert!(legacy.header.is_none());
        assert_eq!(legacy.records.len(), 1);
    }
}
