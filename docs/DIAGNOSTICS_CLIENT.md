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
| Capture file | Optional **`USBDIAGv`** 16-byte header + `uint64_le host_ns` + 8-byte report; legacy (no header) still decodes |

Ideal final function (TRIZ): *presentation works without the stick* — via `file` / `replay` / `demo` sources.

## Dual toolchain

```text
Python (lab)                         Rust (production)
host/usbasp-hidraw-log.py            usbasp-ng-diag record
host/usbasp-trace.py                 usbasp-ng-diag decode / replay
host/usbasp-diag-monitor.py          usbasp-ng-diag monitor [--json]
                                     usbasp-ng-diag demo <scenario>
host/golden/diag/                    same fixtures (parity test)
```

```bash
cd tools/usbasp-ng-diag && cargo build --release
./target/release/usbasp-ng-diag demo --list
./target/release/usbasp-ng-diag demo enableprog_fail_sw --faults
./target/release/usbasp-ng-diag demo enableprog_fail_sw --jsonl | lnav
./target/release/usbasp-ng-diag decode capture.bin --jsonl > capture.jsonl
./target/release/usbasp-ng-diag decode capture.bin --faults
```

### Capture header (`USBDIAGv`)

| Offset | Field |
|--------|-------|
| 0..7 | magic `USBDIAGv` |
| 8 | format_version (=1) |
| 9 | diag_schema (=1) |
| 10 | record_size (=16) |
| 11 | flags |
| 12..15 | reserved |

`record` / `hidraw-log` write the header on new files. Decoders accept header or legacy.

### lnav (direct)

```bash
# no intermediate file required
cargo run --manifest-path tools/usbasp-ng-diag/Cargo.toml -- \
  demo enableprog_fail_sw --jsonl | lnav

# or from a capture
cargo run --manifest-path tools/usbasp-ng-diag/Cargo.toml -- \
  decode capture.bin --jsonl | lnav
```

`--faults` shows ERROR / OVERFLOW / FAIL sequences + summary (human, not lnav).

### Capture header note

Python lab: `python3 host/usbasp-trace.py capture.bin --jsonl` or `--faults`.

## Layers L0–L3

```text
L0 Wire          DiagFrame (6 B) + USB report pad
L1 Protocol      decoded human / JSON lines
L2 Application   AppState reducer   # P2
L3 Presentation  stdout / JSON / TUI
```

## Status

| Item | Status |
|------|--------|
| Firmware PR1–PR3 + MEMOP | done |
| Client P0 record/decode/monitor | done |
| Client P1 replay/demo + header + `--jsonl`/`--faults` | done |
| Golden parity Python↔Rust | `host/golden/diag/` |
| Client P2 TUI | open |
| FX2 physical oracle | open ([SOFTWARE_SCK.md](SOFTWARE_SCK.md)) |

Success: bugs reproducible from `.bin` / `demo` without hardware.
