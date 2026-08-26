# Diagnostics client (host)

Companion to firmware [DIAGNOSTICS.md](DIAGNOSTICS.md). **Production client = Rust** (`tools/usbasp-ng-diag`); **lab = Python** under `host/`.

Telemetry rides **HID interrupt EP2** (composite IF2), not the USART bridge on PD0/PD1 and not `/dev/ttyUSB*`. Classic (`USBASP_HAS_DIAG=0`) has no diagnostics endpoint.

## Align with firmware contracts

| TZ wording | Actual contract |
|------------|-----------------|
| “HID UART as telemetry channel” | **EP2 IN** diagnostics frames; EP1 stays optional UART bridge |
| “Actual SCK frequency via timer” | **P2** only; RC reports `SCK_CONFIG` id + HW/SW |
| “RESET LOW/HIGH” | **`RESET_ASSERT` / `RESET_RELEASE`** (drive intent) |
| ENABLEPROG packing | **Four** 6-byte frames (`START`/`CONT`/`END\|RESULT`) |
| FAULT_SNAPSHOT | **Four** compact frames; END carries `rx[0]` + `sw_delay` + OK/FAIL |
| Capture file | `uint64_le host_ns` + 8-byte USB report (no versioned header yet) |

Ideal final function (TRIZ): *presentation works without the stick* — via `file` / `replay` / `demo` sources.

## Dual toolchain (RC)

```text
Python (lab)                         Rust (production P0)
host/usbasp-hidraw-log.py            usbasp-ng-diag record
host/usbasp-trace.py                 usbasp-ng-diag decode
host/usbasp-diag-monitor.py          usbasp-ng-diag monitor [--json]
host/golden/diag/                    same fixtures (parity test)
```

| | Python | Rust |
|--|--------|------|
| Role | Forensic, golden, bench | Single binary, end-user |
| Deps | pyusb (lab machines) | clap, rusb, serde, anyhow |
| TUI | not required | ratatui = **P2**, not RC |

```bash
cd tools/usbasp-ng-diag && cargo build --release
./target/release/usbasp-ng-diag monitor YEL0
./target/release/usbasp-ng-diag decode capture.bin
```

## Layers L0–L3

```text
L0 Wire          DiagFrame (6 B) + USB report pad
L1 Protocol      decoded human / JSON lines
L2 Application   AppState reducer   # P1+
L3 Presentation  stdout / JSON / TUI
```

## RC status

| Item | Status |
|------|--------|
| Firmware PR1 lifecycle | done |
| Firmware PR2 ENABLEPROG + snapshot | done (4-frame compact snapshot) |
| Firmware PR3 forensics | done (last-try `DIAG_ERROR`, SCK_CONFIG/step, ring 32) |
| Client P0 record/decode/monitor | Python + Rust |
| Golden parity Python↔Rust | `host/golden/diag/` |
| Client P1 replay/demo | open |
| Client P2 TUI | open |
| FX2 physical oracle | open ([SOFTWARE_SCK.md](SOFTWARE_SCK.md)) |

Success criteria for RC: one production binary; bugs reproducible from `.bin`; firmware+client decode the same DIAG v1 bytes.
