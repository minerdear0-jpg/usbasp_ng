# Diagnostics Plane (design)

Architectural separation for HIDUART / research builds. **Not implemented yet.** Does not change the Fischl USBasp wire protocol. Classic stays telemetry-free.

Companion physical truth for SW SCK remains an FX2 / Nano capture: [SOFTWARE_SCK.md](SOFTWARE_SCK.md), [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md). Telemetry is **firmware truth**; the two must be comparable, not substitutes.

Sweet spot to keep:

```text
classic     → zero diagnostics cost
hiduart     → semantic diagnostics + fault snapshots + optional TRACE
host        → dumb recorder + smart decoder
physical    → FX2/Nano = independent truth
```

Pipeline (iron rule):

```text
ISP timing → event → RAM → USB when convenient
```

Never: `ISP timing → USB → pray`.

## Three planes

```text
                         USB
                          │
              ┌───────────┴───────────┐
              │                       │
        V-USB EP0                HID interrupt EP
              │                       │
       USBasp Protocol Plane    Diagnostics Transport
              │                       │
              ▼                       ▼
       ISP Protocol Engine      ring → HID task
              │                       ▲
              ▼                       │
       ISP Data Plane (HW/SW SCK) ────┘  diag_try_emit() only
              │
              ▼
          Target AVR
```

| Plane | Role |
|-------|------|
| **Protocol** | FUNC CONNECT…GETCAPABILITIES on EP0 (unchanged) |
| **ISP Data** | Bitbang / HW SPI / RST / MOSI / MISO timing |
| **Diagnostics** | Compact binary events → RAM ring → HID drain |

Target UART bridge (`printf` of the DUT) is a **separate product mode** and a board capability (`BOARD_HAS_TARGET_UART`). Stock clones do not route PD0/PD1 to the ISP header — see [KNOWN_ISSUES.md](KNOWN_ISSUES.md). Do not mix that with this plane.

## Architectural invariants

### 1. Emit is lossy by design

API name must remind callers that events may vanish and ISP must not care:

```c
/* Returns false if dropped. Callers MUST NOT retry or wait. */
bool diag_try_emit(uint8_t type, uint8_t flags, uint8_t a, uint8_t b);
```

Forbidden:

```c
if (!diag_try_emit(...))
    retry();   /* NO */
while (!diag_try_emit(...))
    ;          /* NO — blocks ISP */
```

ISP pass/fail must be identical with diagnostics OFF or ring full.

### 2. `DIAG_FAULT_SNAPSHOT` is an atomic snapshot

On failure, **first** freeze state into a RAM snapshot struct (SCK config, RESET, last SPI bytes, metrics, `prog_state`), **then** publish one snapshot event (or a fixed multi-frame sequence from that frozen copy). Do not assemble the snapshot piecemeal after returning to normal flow — HID drain must never see half-old / half-new fields.

```text
failure detected
      ↓
snapshot current state / SPI / SCK metrics   (atomic in RAM)
      ↓
publish DIAG_FAULT_SNAPSHOT from that copy
      ↓
continue normal ISP return path
```

### 3. Timestamp is formal and local

| Property | Spec |
|----------|------|
| Source | Monotonic **local** tick (not wall clock, not USB SOF required) |
| Width | `uint16_t` |
| Wrap | **Expected**; host reconstructs relative deltas across wrap |
| Unit | Defined per schema (v1 may use Timer0 overflow / ~ms-class tick — freeze before shipping captures). Need not be 1 µs |

Host wall-clock stamps belong only in the **recorder** metadata around raw frames, not inside firmware events.

### 4. Semantic ENABLEPROG ≠ optional SPI_BYTE

| Event | Role |
|-------|------|
| `DIAG_ENABLEPROG` | **Semantic transaction**: TX[4], RX[4], result. Sufficient for SESSION/TRANSACTION diagnostics alone |
| `DIAG_SPI_BYTE` | **Optional transport TRACE** of underlying bytes |

Host may show:

```text
ENABLEPROG  TX AC 53 00 00  RX 00 53 00 00  PASS
```

with **zero** `DIAG_SPI_BYTE` frames. TRACE level may *additionally* emit per-byte `SPI AC→00` … Changing TRACE detail must not break semantic diagnostics.

## Wire format (binary, host presents)

Stable on the wire; human text lives only on the host.

```c
typedef enum {
    DIAG_HELLO = 1,         /* schema + feature bits — not USBasp FUNC 127 */
    DIAG_SESSION_BEGIN,
    DIAG_SESSION_END,
    DIAG_RESET,             /* assert / release in flags */
    DIAG_SCK_CONFIG,        /* requested id + HW/SW */
    DIAG_ENABLEPROG,        /* semantic TX×4/RX×4/result — may be multi-frame */
    DIAG_SPI_BYTE,          /* optional TRACE only: TX, RX */
    DIAG_SCK_STATS,         /* min/max/sum periods after N edges */
    DIAG_FAULT_SNAPSHOT,
    DIAG_TRACE_OVERFLOW,    /* includes dropped_count (saturating 0..255) */
    DIAG_ERROR
} diag_event_type_t;

struct diag_frame {
    uint8_t type;
    uint8_t flags;
    uint16_t timestamp;     /* see invariant 3 */
    uint8_t  a, b;          /* type-specific */
};
```

Schema label: `USBASP-NG DIAG v1`. Bump when layout or tick unit changes; keep old decoders.

### `DIAG_HELLO` / internal capabilities

**Not** USBasp `FUNC_GETCAPABILITIES` (127) — that EP0 contract stays untouched.

On diagnostics open (or first frames), firmware emits HID-only:

```text
DIAG_HELLO
  schema   = 1
  features = SESSION | TRANSACTION | SNAPSHOT | TRACE | SCK_STATS  (bitmask)
```

Host tools must not guess whether `DIAG_SCK_STATS` exists across hiduart v1/v2.

### Levels

| Level | What |
|-------|------|
| **OFF** | No events |
| **ERROR** | Failures, overflow markers |
| **SESSION** | CONNECT / DISCONNECT / RESET edges / SCK config / HELLO |
| **TRANSACTION** | Semantic `DIAG_ENABLEPROG` (not mandatory SPI_BYTE) |
| **TRACE** | Opt-in raw stream (`DIAG_SPI_BYTE`, …). Never default |

Do **not** log every SCK edge. At 32 kHz that is tens of thousands of edges/s.

SCK frequency (P2): accumulate period samples (e.g. N=64) → min/max/sum. Prefer **cycle-counted / selected-edge** sampling — no capture ISR on every SW-SCK edge. Telemetry must not change the object under test. V-USB / Timer0: [USB_EXECUTION.md](USB_EXECUTION.md).

### Overflow

`DIAG_TRACE_OVERFLOW` carries at least a **saturating** `dropped_count` (`0..255`, `255` = 255+). Prefer **drop-oldest** so the end of the operation remains visible.

## Fault snapshot (P0)

Atomic RAM freeze then publish (invariant 2). Typical fields:

- `requested_sck`, effective transport (HW/SW), RESET level  
- last ENABLEPROG TX[4]/RX[4]  
- last few SCK periods if available  
- `prog_state`  

Cheap on the happy path.

## ISP mirror / TRACE

Optional mirror into the same ring (avrdude still stock USBasp). First generation is **trace**, not a protocol proxy.

## Safety

- Telemetry must not change ISP pass/fail.  
- Full TRACE arming: only before CONNECT (or explicit host enable); during WRITEFLASH → best-effort.  
- Ring full → drop (+ overflow with count). Never wait, retry-spin, disable USB, or delay ISP for HID.

```text
ISP engine → diag_try_emit() → ring → HID drain
```

Never `ISP → HID` directly.

## Product split

| Image | Telemetry |
|-------|-----------|
| **classic** | None (closest to stock USBasp) |
| **hiduart** | Semantic diagnostics + fault snapshots + optional TRACE |
| **hiduart-debug** (optional later) | Max TRACE + snapshots; still no requirement for target UART |

## Host tools

```text
host/usbasp-hidraw-log.py   # discover → read interrupt → host stamp → raw .bin
host/usbasp-trace.py        # .bin → human (schema-aware via DIAG_HELLO)
```

Recorder is **dumb**: it must not interpret frames. Implement recorder **with P0** even while TRACE is off — future firmware changes stay replayable from real `.bin` files.

Not `/dev/ttyUSB*` — HIDUART is **hidraw**, not CDC. Existing status/loopback scripts stay functional checks.

## Suggested tree (when implemented)

```text
firmware/include/diag/     diag.h, diag_events.h, diag_trace.h
firmware/src/diag/         diag.c, diag_ring.c, diag_metrics.c
firmware/src_hid/          hid_diag.c   (drain only)
```

Classic: omit link or empty stubs.

## Priority

| Pri | Item |
|-----|------|
| **P0** | Atomic fault snapshot; semantic `DIAG_ENABLEPROG`; RESET transitions; transport/SCK config; `DIAG_HELLO` |
| **P1** | **hidraw recorder + offline decoder** (with P0); binary ISP TRACE |
| **P2** | SCK period statistics (non-invasive) |
| **P3** | Target UART bridge (board capability only) |

P0 firmware stays cheap: no interrupt-per-SCK, no USBasp protocol change, no classic bloat. P1 recorder ships early because it is schema-agnostic.

## Relation to SW SCK work

| Source | Truth |
|--------|--------|
| FX2 / sniffer | RST / SCK / MOSI / MISO on the wire |
| Diagnostics Plane | What firmware believes (state, TX/RX, transport, periods) |

Disagreement is evidence.

**Agreement does not prove target-side correctness.** It only proves the programmer **emitted what firmware reports**. Example: firmware TX=`AC`, FX2 MOSI=`AC`, MISO=`FF` strongly **localizes off-stick** (power, RESET, MISO contention, phase, wiring, target clock, …) — not necessarily a “target bug”. Prefer the phrase **off-stick**, not “target fault”.

Do **not** treat HID TRACE as a substitute for the planned `-B 8` vs `-B 22` capture.
