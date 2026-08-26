# Diagnostics client (host)

Companion to firmware [DIAGNOSTICS.md](DIAGNOSTICS.md). **Production client = Rust** (`usbasp-ng-diag`); **lab = Python** under `host/`.

Telemetry rides **HID interrupt EP2** (composite IF2), not the USART bridge on PD0/PD1 and not `/dev/ttyUSB*`. Classic (`USBASP_HAS_DIAG=0`) has no diagnostics endpoint.

## Align with firmware contracts

The 2026-08-26 client TZ is accepted with these corrections (do not reintroduce):

| TZ wording | Actual contract |
|------------|-----------------|
| “HID UART as telemetry channel” | **EP2 IN** diagnostics frames; EP1 stays optional UART bridge |
| “Actual SCK frequency via timer” | **P2** only; P0/PR1 report `SCK_CONFIG` id + HW/SW |
| “RESET LOW/HIGH” | **`RESET_ASSERT` / `RESET_RELEASE`** (drive intent) |
| ENABLEPROG packing | **Four** 6-byte frames (`START`/`CONT`/`END|RESULT`) — firmware PR2 |
| Capture file | Lab recorder today: `uint64_le host_ns` + 8-byte USB report; freeze a versioned header later for Rust |

Ideal final function (TRIZ): *presentation works without the stick* — via `file` / `replay` / `demo` sources.

## Dual toolchain

```text
Python (lab)                         Rust (production)
host/usbasp-hidraw-log.py            usbasp-ng-diag record
host/usbasp-trace.py                 usbasp-ng-diag decode
host/usbasp-diag-monitor.py          usbasp-ng-diag monitor [--json]
golden fixtures                      same fixtures in CI
```

| | Python | Rust |
|--|--------|------|
| Role | Forensic, golden, bench | Single binary, end-user |
| Deps | pyusb (lab machines) | clap, hidapi, serde, anyhow |
| TUI | not required | ratatui = **P2**, not P0 |

Do **not** start with TUI. Do **not** bind UI to HID.

## Layers L0–L3

```text
L0 Wire          DiagFrame (6 B) + USB report pad
L1 Protocol      DecodedEvent (type-safe)
L2 Application   AppState = reduce(state, event)   # P1/P2
L3 Presentation  stdout / JSON / TUI
```

Dependencies only downward. Sources implement the same stream of L0 frames:

- HID (live EP2)
- File (`.bin` capture)
- Stdin
- Synthetic (demo scenarios)

## Planned Rust layout

```text
tools/usbasp-ng-diag/          # when Rust toolchain is available
  Cargo.toml
  src/
    main.rs
    cli.rs
    protocol.rs                # L0 only
    decoder.rs                 # L0 → L1
    events.rs
    state.rs                   # L2 reducer (P1+)
    source/{hid,file,synthetic}.rs
    formatter.rs
    recorder.rs
    tui.rs                     # P2
  tests/golden_*
```

### P0 commands (Rust)

1. `record capture.bin` — HID → raw frames (+ metadata header when schema frozen)  
2. `decode capture.bin` — `.bin` → human events  
3. `monitor` / `monitor --json` — live HID → decoder → stdout  

### P1

`replay` (`--speed`, `--step`), `demo --scenario …`, golden vector CI, fault-oriented formatting.

### P2

`monitor --watch` (ratatui), full `AppState` reducer, interactive replay.

## Lab status (now)

| Tool | Role |
|------|------|
| [`host/usbasp-hidraw-log.py`](../host/usbasp-hidraw-log.py) | Dumb recorder (host ns + 8 B) |
| [`host/usbasp-trace.py`](../host/usbasp-trace.py) | Offline decode |
| [`host/usbasp-diag-monitor.py`](../host/usbasp-diag-monitor.py) | Live EP2 → human lines |

Firmware PR1 emits: `HELLO`, `SESSION_*`, `SCK_CONFIG`, `RESET_*`, `TRACE_OVERFLOW`.  
Firmware PR2 will add: 4-frame `ENABLEPROG`, `FAULT_SNAPSHOT`.

## Golden vectors

Keep dual-decoder discipline:

```text
firmware/tests/core/test_diag_v1.py     # header constants ↔ C
host/golden/diag/                       # raw snippets (lab)
tools/usbasp-ng-diag/tests/             # Rust (when present)
```

Same bytes must decode identically in Python and Rust.

## Roadmap vs firmware

| Track | Owner |
|-------|--------|
| Firmware PR2 ENABLEPROG + snapshot | AVR |
| Client P0 record/decode/monitor | Python now → Rust when cargo available |
| Client P1 replay/demo/golden | after ENABLEPROG frames exist |
| Client P2 TUI | last |
| FX2 capture `-B 8` vs `-B 22` | physical oracle (unchanged) |

Success criteria: replay/demo without hardware; one production binary; CI `--json`; bugs reproducible from `.bin`.
