# Diagnostics Plane (design)

**RC (PR1–PR3).** Lifecycle + ENABLEPROG + compact fault snapshot + last-try `DIAG_ERROR` (check + `sck_sw_delay`) + `SCK_CONFIG` each SCK step. Ring 32. Classic remains `USBASP_HAS_DIAG=0`.

Architectural separation for HIDUART / research builds when `USBASP_HAS_DIAG=1`. Does not change the Fischl USBasp wire protocol. Classic stays telemetry-free (`USBASP_HAS_DIAG=0`).

**USBasp2 (ATmega328P):** same L1 USBasp; Diagnostics evolves toward an **ISP development probe** (observability + measurement + capture) — not AVR-ICE. Philosophy and roadmap: **[DIAGNOSTICS_PROBE.md](DIAGNOSTICS_PROBE.md)**. P0/RC wire below stays frozen until an explicit DIAG v2 bump.

Companion physical truth for SW SCK was scoped then **closed** on USBasp2: [SOFTWARE_SCK.md](SOFTWARE_SCK.md), [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md). Telemetry is **firmware truth**; optional FX2 remains interesting for dual-timestamp science, not a release gate.

Sweet spot:

```text
classic     → zero diagnostics cost
hiduart*    → USBASP_HAS_DIAG → semantic + snapshots (+ probe roadmap on 328P)
host        → dumb recorder + smart decoder / TUI
physical    → FX2 = independent truth (dual-T with firmware)
```

\* Gate is **`USBASP_HAS_DIAG`**, not `profile == hiduart`. Profile selects features; later `hiduart-debug` / `hiduart-trace` can flip the same flag.

**Host client architecture** (Python lab now, Rust production planned): [DIAGNOSTICS_CLIENT.md](DIAGNOSTICS_CLIENT.md).

Pipeline (iron rule):

```text
ISP timing → diag_try_emit() → RAM ring → hiduart_poll() → EP2 IN
```

Timestamps come from Timer1 (`diag_clock` / `diag_now()`); P0/RC wire still carries **low 16 bits** only (`diag_now_wire16`). See [DIAGNOSTICS_PROBE.md](DIAGNOSTICS_PROBE.md).

Never: `ISP timing → USB → pray`. EP1 UART bridge untouched. FUNC 1–16 / 127 unchanged. Telemetry is **not** part of the USBasp protocol.

## Three planes

| Plane | Role |
|-------|------|
| **Protocol** | FUNC on EP0 (unchanged) |
| **ISP Data** | HW/SW SCK, RST drive, MOSI/MISO |
| **Diagnostics** | Binary frames → SPSC ring → EP2 drain |

Target UART bridge is a separate board capability (`BOARD_HAS_TARGET_UART`). Stock clones: [KNOWN_ISSUES.md](KNOWN_ISSUES.md).

## Contracts (freeze before PR1)

### C1 — SPSC ownership (P0)

```c
/*
 * SPSC ring:
 *   producer = ISP / foreground vendor_isp context only
 *   consumer = hiduart_poll() / diag_poll_drain()
 *   no ISR producer in P0
 */
volatile uint8_t head, tail;
diag_frame_t frames[DIAG_RING_SIZE]; /* 32, power of two */
```

Cheap single-producer/single-consumer. No locks. Document in `diag_ring.h`.

### C2 — Lossy emit; overflow is deferred

`diag_try_emit()` on full ring:

1. saturating `dropped++` (`0..255`, `255` = 255+)  
2. set `overflow_pending = 1`  
3. return `false`  
4. **do not** try to push `DIAG_TRACE_OVERFLOW` (no room → overflow-of-overflow)

When `diag_poll_drain()` can write a frame and `overflow_pending`:

```text
emit DIAG_TRACE_OVERFLOW (a = dropped, then clear / reduce count)
overflow_pending = 0
```

Name stays `DIAG_TRACE_OVERFLOW` for future TRACE; P0 uses the same type for any drop.

Callers **must not** retry or wait on `false`. Prefer API name `diag_try_emit`.

### C3 — Snapshot copy semantics

```c
void diag_publish_snapshot(const diag_snapshot_t *s);
/* Copies *s into persistent diag RAM immediately.
 * Never retains the caller's pointer. local may go out of scope after return. */
```

Flow:

```text
ENABLEPROG failure
      → fill local diag_snapshot_t
      → capture into persistent fault_snapshot (memcpy)
      → emit snapshot frames from persistent copy
      → return to ISP path
```

No `snapshot_ptr = s`.

### C4 — RESET is semantic drive intent

Events: **`RESET_ASSERT` / `RESET_RELEASE`** (flags on `DIAG_RESET`).

Meaning: *programmer drove* RST asserted/released — **not** measured pin level. No input sense in P0 → never name events `RESET_LOW` / `RESET_HIGH` (that would claim physical truth).

### C5 — HELLO once per CONNECT, not keep-alive

```text
CONNECT → DIAG_HELLO → DIAG_CAPS ×4 → SESSION_BEGIN → …
```

No idle HELLO spam. No heartbeat in P0. If needed later: separate `DIAG_HEARTBEAT`.

`DIAG_HELLO` (6-byte frame sketch):

| Field | Content |
|-------|---------|
| `a` | schema (`1`) |
| `b` | profile / build id (`DIAG_PROFILE_COMPOSITE`, …) |
| `flags` | legacy uint8: `SESSION \| TRANSACTION \| SNAPSHOT \| TIMESTAMP` (P0/RC); TRACE/SCK_STATS when present |

Full capability discovery is **`DIAG_CAPS`** (type 13): four sequence frames carrying little-endian `uint32` firmware bitset then `uint32` board bitset. Hosts **must** gate features on these masks, never on `bcdDevice` / firmware version alone. Unknown bits are ignored.

**Not** USBasp `FUNC_GETCAPABILITIES` (127). Host must not guess features across hiduart generations.

### Timestamp semantics (Timer1)

Wire `timestamp` remains `uint16_t` (low 16 bits of the local Timer1 counter). Wrap is expected.

Resolution is approximately **0.67 µs** at 12 MHz / ÷8. Observed event-to-event deltas include **firmware execution overhead** between `diag_now()` call sites — a delta of 12 ticks is not a claim that two physical edges were exactly 8 µs apart.

## Frame layout

```c
struct diag_frame {
    uint8_t type;
    uint8_t flags;
    uint16_t timestamp;   /* monotonic local tick; wrap expected; unit frozen per schema */
    uint8_t a, b;
};
```

Schema label: `USBASP-NG DIAG v1`.

### `DIAG_SCK_CONFIG` (semantic, not Hz)

```c
/* a = sck_id (requested / applied id), b = transport (HW/SW), flags = extras */
```

Report what firmware **chose**. No “actual frequency” in P0 (that is P2 stats).

### `DIAG_ENABLEPROG` — four ordinary frames

Avoid clever 3-frame packing. Same 6-byte frame; `flags` carry sequence:

| Frame | flags | a, b |
|-------|-------|------|
| 0 | `START` | tx0, tx1 |
| 1 | `CONT` | tx2, tx3 |
| 2 | `CONT` | rx0, rx1 |
| 3 | `END \| RESULT_OK` or `END \| RESULT_FAIL` | rx2, rx3 |

Host reassembles one semantic transaction. Optional later `DIAG_SPI_BYTE` TRACE must not be required for this.

Capture TX/RX **inside** `ispEnterProgrammingMode` without calling `diag_try_emit` from `ispTransmit_sw`. Emit only after the attempt (and snapshot on fail).

### `DIAG_MEMOP` (load markers)

Compact flash/eeprom block markers — **not** per-byte TRACE.

| flags | `a` | `b` |
|-------|-----|-----|
| `START` | mem (`FLASH=0`, `EEPROM=1`, `READFLASH=2`) | `pagesize` (sat 255) |
| `CONT \| OK` or `CONT \| FAIL` | page base addr hi | page base addr lo (byte, low 16) |
| `END \| OK` or `END \| FAIL` | mem | pages flushed (sat 255) |

Emit: `FIRST` → START; each interesting `ispFlushPage` → CONT with **page base** and poll result (`ispFlushPage` ≠ 0 → FAIL); `LAST` / byte-mode / DISCONNECT → END with **authoritative** page count. avrdude often omits `LAST` — a following `READFLASH` or DISCONNECT closes the write first. Successive verify `READFLASH` chunks coalesce into one MEMOP (`START READFLASH`, CONT@chunk ×N, END on write/disconnect).

**CONT subsample** (ring default **64** on mega8; USBasp2/328P **128**): always emit FAIL, first page, last page (forced on END), every 8th page, and any page in `0x1E00..0x1FFF` (ATmega8 oracle canary band). Host must not assume CONT count == END pages. `TRACE_BEGIN.a` advertises slots.

> Wire change (2026-08-26): page ticks are `CONT@addr`, not duplicate `END` with only a page counter. READFLASH lifecycle + CONT subsample.

### `DIAG_ISP_PINS` (Hi-Z claim after disconnect)

Emitted on DISCONNECT **after** `ispDisconnect()` and `RESET_RELEASE`. Dual-truth vs target `ISP_PINS,when=reset`.

| flags | `a` | `b` |
|-------|-----|-----|
| `AFTER_DISC \| OK` | `DDRB & ISP_mask` | `PINB & ISP_mask` |
| `AFTER_DISC \| FAIL` | same | same |

`ISP_mask` = RST(PB2)|MOSI(PB3)|MISO(PB4)|SCK(PB5). **FAIL** if RST/MOSI/SCK still have DDR=1 (still driving). Host prints `ddr=` / `pin=` / per-line bits.

### `DIAG_FAULT_SNAPSHOT` fields (P0)

RAM copy (`diag_snapshot_t`) still holds full TX/RX + `sw_delay`. Wire emit is **4 frames** (compact):

| # | flags | `a` | `b` |
|---|-------|-----|-----|
| 0 | `START` | `(sck_req << 4) \| (effective_sck & 0x0f)` | `transport` |
| 1 | `CONT` | `reset_driven` | `state` |
| 2 | `CONT` | `tx[0]` | `tx[1]` |
| 3 | `END \| OK/FAIL` | `rx[0]` | `sw_delay` |

Full TX/RX also on `ENABLEPROG`; last-try notes on `DIAG_ERROR`. Emit from **persistent** copy after `diag_publish_snapshot`.

## Levels

| Level | What |
|-------|------|
| OFF | Nothing |
| ERROR | Drops / overflow when drained |
| SESSION | HELLO, SESSION_*, RESET_*, SCK_CONFIG |
| TRANSACTION | Semantic ENABLEPROG (4 frames) |
| TRACE | Opt-in `DIAG_SPI_BYTE` — never default; **not PR1/PR2** |

## EP2 drain

One `diag_frame` per ready interrupt (pad to 8 if HID report size requires). Prefer EP2 for diagnostics; **do not** steal EP1 UART. No empty HELLO keep-alive when ring empty — skip or send idle status byte only if existing monitor already needs it; prefer silence.

## CMake

```text
profile → features → USBASP_HAS_DIAG=0|1
```

Classic: `0` → macros no-op, no `diag.o`. HIDUART (and future variants): `1`.

## Implementation split (approved)

### PR1 — Skeleton + lifecycle (no ISP timing change)

- `include/diag/`, `src/diag/` (ring + emit + drain hooks)  
- `USBASP_HAS_DIAG` gate + classic stubs  
- SPSC contracts C1–C2 in headers  
- `DIAG_HELLO`, `SESSION_BEGIN/END`, `SCK_CONFIG`, `RESET_ASSERT/RELEASE`  
- EP2 drain from `hiduart_poll`  
- Golden Python frame constants / decoder stubs  
- Optional: dumb `hidraw-log` (schema-agnostic)

### PR2 — ENABLEPROG + snapshot — **done**

- Local TX/RX capture in `ispEnterProgrammingMode`  
- 4-frame semantic ENABLEPROG  
- Persistent fault snapshot + publish (C3)  
- Deferred overflow accounting on drain  

### PR3 / RC — forensics + ring headroom — **done**

- Last-try `DIAG_ERROR` (check + `sck_sw_delay`) per SCK step  
- `SCK_CONFIG` on each ENABLEPROG step  
- Compact 4-frame `FAULT_SNAPSHOT` (END: `rx[0]` + `sw_delay` + OK/FAIL)  
- Ring size 32  

Bench HIDUART `-B 8` vs `-B 22` for firmware truth; FX2 remains physical oracle.

## Out of P0 / do not expand

- Interrupt-per-SCK, timer capture ISR  
- Actual SCK Hz / period stats (P2)  
- TRACE SPI_BYTE stream  
- Target UART  
- Changing FUNC 127 or classic USB  
- Heartbeat / HELLO keep-alive  
- Claiming physical RESET level without sense  

## Relation to SW SCK

| Source | Truth |
|--------|--------|
| FX2 / sniffer | Wire RST/SCK/MOSI/MISO |
| Diagnostics | Firmware intent + reported TX/RX/transport |

Agreement **localizes off-stick**; it does not prove “target bug”. Disagreement is evidence. HID TRACE/snapshots do **not** replace the planned `-B 8` vs `-B 22` capture.
