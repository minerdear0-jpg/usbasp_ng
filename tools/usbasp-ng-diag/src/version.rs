//! Independent version lines for the host tool vs the EP2 wire protocol.
//!
//! - **diagplane** — this binary / crate (`CARGO_PKG_VERSION`)
//! - **protocol** — USBASP-NG DIAG schema on HID EP2 (`DIAG_SCHEMA_V1` in firmware)
//!
//! Bump them separately. Feature gates stay on CAPS bits, never on these numbers alone.

/// Host client (diagplane) version — Cargo package version.
pub const DIAGPLANE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Wire protocol major (matches firmware `DIAG_SCHEMA_V1` / HELLO schema byte).
#[allow(dead_code)] // public API; equals `protocol::SCHEMA_V1`
pub const PROTOCOL_VERSION: u8 = 1;

/// Human-readable protocol id for banners and `--version`.
pub const PROTOCOL_VERSION_STR: &str = "1";

/// `diagplane -V` text. Clap prefixes the binary name on the first line.
pub const VERSION_LONG: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\nprotocol  ",
    "1",
    "  (USBASP-NG DIAG schema)"
);

/// One-line identity for TUI / CLI headers.
pub fn banner_short() -> String {
    format!("diagplane {DIAGPLANE_VERSION}  protocol {PROTOCOL_VERSION_STR}")
}
