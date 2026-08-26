# Diagnostics Plane → Development Probe (USBasp2)

**Status:** architecture direction for **ATmega328P / USBasp2**.  
**Does not replace** the shipping P0/RC contract in [DIAGNOSTICS.md](DIAGNOSTICS.md).  
**Does not claim** AVR-ICE / debugWIRE / JTAG / UPDI.

```text
USBasp  →  USBasp NG  →  USBasp NG Development Probe
```

Honest product: an **ISP development probe** with observability, measurement, and capture — while `avrdude -c usbasp` still sees ordinary USBasp (L1 unchanged).

## Why 328P changes the class of device

| MCU | Framing question |
|-----|------------------|
| ATmega8 (~8 KiB) | How do we add diagnostics **without breaking** USBasp? |
| **ATmega328P (32 KiB / 2 KiB)** | How do we build an **observability system around** USBasp while keeping L1? |

P0’s 6-byte `diag_frame` remains excellent for mega8 and for the RC wire. On USBasp2 we stop thinking “two more diagnostic bytes” and design **extension points** toward a probe — without rewriting P0 now.

## Three modes (one stick)

```text
                    USBasp NG 328P (USBasp2)
                              │
              ┌───────────────┼────────────────┐
              │               │                │
          USBasp L1       Diagnostics       Lab modes
          compatibility      Plane
              │               │
         avrdude              │
         FUNC 1–16/127        │
                              │
                    ┌─────────┼─────────┐
                    │         │         │
                 Observe   Record    Analyze
                  state     trace    timing
```

**L1 never knows** a lab instrument sits underneath. Diagnostics and lab modes ride HID EP2 (and optional board capabilities), not the Fischl vendor protocol.

## What we borrow from ICE / analyzers (ideas only)

| Take | Skip |
|------|------|
| Timestamped semantic timeline | debugWIRE / JTAG / UPDI |
| Circular buffers + **pre/post trigger** | Claiming full pin logic-analyzer |
| Trigger conditions (not “trace everything”) | Breakpoint / debug register machinery |
| Capability map (“what this probe can do”) | Pretending to be AVR-ICE |
| Dual truth: firmware event ↔ physical capture | Unlabeled intrusive measurement |

## Observability levels

| Level | Content | Default |
|-------|---------|---------|
| **L1 semantic** | CONNECT, RESET, SCK_CONFIG, ENABLEPROG, MEMOP, SESSION | always-on (today’s P0/RC) |
| **L2 transaction** | TX/RX, transport, coarse timing, errors | on demand / fault path |
| **L3 capture** | raw ISP transactions, trigger, pre/post freeze, replay | **opt-in**, may be intrusive |

## Extension roadmap (do not implement as one bang)

```text
semantic events (P0/RC — done)
      ↓
monotonic timestamp (Timer1 logical time) — done (`diag_clock`)
      ↓
capability bits — done (live YEL0 acceptance passed)
      ↓
unified TRACE ring + capture metadata — done
      ↓
trigger predicates + POST → FROZEN — **this PR**
      ↓
HID EP2 stream (versioned wire as needed)
      ↓
host record / replay / TUI
```

### Responsibility split (freeze)

| Layer | Owns |
|-------|------|
| clock | time (`diag_now`) |
| ring | history (one circular buffer) |
| trigger | condition only (later) |
| capture | lifecycle ARMED → POST → FROZEN |
| HID | transport |
| host | interpretation |
| TUI | presentation |

### Timer1 clock (`diag_clock`) — done

- `diag_clock_init()` / `diag_now()` → `uint32_t` ticks; **no** Timer1 overflow ISR (lazy TOV under `cli`).
- Prescaler `/8` @ 12 MHz → ≈0.667 µs/tick; 16-bit period ≈43.7 ms; soft epoch → ~40 min.
- **Wire unchanged:** P0/RC still uses `diag_now_wire16()` (low 16 bits). Full T reserved for firmware / DIAG v2.
- Must call `diag_now()` at least once per period while continuity matters (`diag_poll_drain` does).
- Host model tests: `firmware/tests/core/test_diag_clock.py`.

### Capabilities — done (gate TRACE on live check)

Firmware advertises `DIAG_CAPS` (4 frames) after `HELLO`. Hosts gate features on bitsets, never `bcdDevice`.

**Live acceptance (YEL0 after flash + replug)** — demo/golden is not enough:

```bash
SERIAL=YEL0 make -C firmware BOARD=usbasp-hiduart-atmega328p flash   # via no-dot / other USBasp
# replug YEL0
cargo run -p usbasp-ng-diag -- capabilities --serial YEL0
```

Expect strictly:

| Field | Value |
|-------|-------|
| firmware | `0x00000007` |
| board | `0x00000002` |
| SESSION / SNAPSHOT / TIMESTAMP | ✓ |
| TRACE / TRIGGER / PRETRIGGER / SCK_STATS | ✗ |
| SCK_JUMPER | ✓ |
| TARGET_UART / PHYSICAL_CAPTURE | ✗ |

Also: old HELLO parsers must ignore the four unknown `DIAG_CAPS` frames without breaking CONNECT lifecycle.

Do **not** start TRACE until this live row is green. *(Closed: firmware `0x07` → then TRACE PR advances mask to `0x0F`.)*

### 2. Dual timestamp (firmware ↔ physical)

Keep firmware `T` **inside** the event so FX2/PulseView can align:

```text
firmware ENABLEPROG  T=123456
FX2      SCK edge    T=123459   → Δ ≈ 2.7 µs
```

That is engineering diagnosis, not printf.

### 3. Unified TRACE ring — landed (no trigger predicate)

**One** circular buffer (`USBASP_DIAG_TRACE_SLOTS`, default **64**). Semantic P0 events and future raw ISP types share `diag_trace_push()` — not twin rings.

```text
diag_try_emit / trace_event  →  TRACE RING  →  diag_trace_drain  →  HID EP2
```

- **Lossy:** overwrite oldest when full; never block ISP/USB.
- **OVERFLOW:** sticky flag + deferred `TRACE_OVERFLOW` marker on the next roomy push (never jammed into a full ring as a self-eviction).
- **Lifecycle:** `IDLE` ↔ `ARMED` only; `POST`/`FROZEN` reserved for trigger PR (ARMED never auto-freezes).
- **Metadata:** `TRACE_BEGIN` (slots, frame_size, state) / `TRACE_END` (valid, write_index, overflow).
- **Ownership:** producer and consumer both main-context (`hiduart_poll`); no `cli()` on the hot path.
- Host: decode/replay keep `overflow=YES`; TUI shows TRACE slots + overflow.

### 4. Trigger engine — landed (ENABLEPROG_FAIL first)

Predicate layer only: `diag_trigger_match()` after `diag_trace_push()` so the firing event is in the capture. Default: `DIAG_TRIG_ENABLEPROG_FAIL`.

```text
ARMED → (match) → POST_CAPTURE → (N=USBASP_DIAG_POST_CAPTURE_EVENTS, default 16) → FROZEN
```

`TRACE_END` (4 frames) reports: valid, write_index, overflow, triggered, kind, post_count, trigger_index, trigger_timestamp.

Non-intrusive: no extra ISP/GPIO/USB for the match.
### 5. Raw ISP capture (bounded)

Per-transaction `{T, TX, RX, transport}` — **not** every SCK edge in normal mode. Optional `DIAG_MODE_CAPTURE` may add instrumentation; must advertise:

```text
capture_mode = intrusive | non_intrusive
```

A measuring instrument must say when it perturbs the DUT timing.

### 6. Diagnostics capabilities (beyond TPI/3 MHz)

Host: `usbasp-ng-diag capabilities` → map. Gate UI/features on **capability bits**, never `firmware >= …`.

Two bitsets (LE `uint32`, advertised in `DIAG_CAPS` after HELLO on **ISP CONNECT** — not USB plug-in):

**Firmware (diagnostics):** `SESSION`, `SNAPSHOT`, `TIMESTAMP`, `TRACE`, `TRIGGER`, `PRETRIGGER`, `SCK_STATS`  
**Board (physical):** `TARGET_UART`, `SCK_JUMPER`, `PHYSICAL_CAPTURE`

Today (USBasp2 / YEL0): TIMESTAMP / TRACE / TRIGGER / PRETRIGGER **yes**; SCK_STATS **no**; `PHYSICAL_CAPTURE` **no**. Firmware mask `0x0000003F`.

```bash
usbasp-ng-diag capabilities --demo capabilities_yel0
# live: start listener first, then avrdude (CAPS on CONNECT)
usbasp-ng-diag capabilities --serial YEL0 --timeout 30
```

Acceptance: [ACCEPTANCE-DIAG-TRIGGER-001](acceptance/ACCEPTANCE-DIAG-TRIGGER-001.md).

### Timestamp note

`timestamp` is a local Timer1 tick counter; resolution is approximately 0.67 µs at 12 MHz / ÷8. Event-to-event observed deltas include firmware overhead at `diag_now()` — not a pure physical-edge stopwatch.

### 7. Target monitor (UART) — tagged, separate source

Closed-loop bench (**Канарейка** = mega8-on-Nano + ttyUSB) is real: [USBASP2.md](USBASP2.md), `bench/mega8-nano-loop/`, `bench/mega8-diag-oracle/`.

In the probe architecture, target UART is a **board capability** under Diagnostics, always tagged:

```text
source=PROGRAMMER | source=TARGET
```

Never merge into one unlabeled stream.

**Host (first slice):** `diagplane correlate --diag ep2.jsonl --uart oracle.txt`  
aligns target `@ms` onto `host_ns` using **RESET RELEASE ↔ READY/APP_START** (Timer1 path). FX2/PulseView physical edges come later — same `T` in the event, third column.

### 8. Snapshot-now

Host command → one coherent dump: USB/ISP/SCK/RESET, last transaction, ring occupancy, drops, trigger/trace state. Better than a pile of GET opcodes.

### 9. Wire protocol evolution (v1 header)

Keep P0 logical 6-byte events. For USBasp2 extensions, plan a versioned header:

```text
type | flags | timestamp | len | payload[]
```

So ENABLEPROG, FAULT_SNAPSHOT, TRACE, TARGET_UART, SCK_STATS can grow without yearly format breakage. **Ship P0/RC unchanged until an explicit DIAG v2 bump.**

### 10. Host / TUI

`diagplane` Observe / Record / Analyze over live EP2 **and** replay files (no stick). TUI is justified once timestamps + triggers exist; today’s `watch` is the seed.

## Non-goals

- Cheap AVR-ICE clone  
- Edge-accurate logic analyzer at MHz rates on V-USB  
- Breaking classic mega8 images or `USBASP_HAS_DIAG=0`  
- Intrusive capture without advertising it  

## Relation to existing docs

| Doc | Role |
|-----|------|
| [DIAGNOSTICS.md](DIAGNOSTICS.md) | **Frozen P0/RC** wire + contracts |
| **This file** | USBasp2 **probe** philosophy + roadmap |
| [DIAGNOSTICS_CLIENT.md](DIAGNOSTICS_CLIENT.md) | Host layers L0–L3 |
| [USBASP2.md](USBASP2.md) | 328P hardware / boards / smoke |
| [SOFTWARE_SCK.md](SOFTWARE_SCK.md) | Historical SW-SCK gate (closed); still a capture-mode use-case |

## Immediate discipline

1. **Do not** rewrite P0 on mega8 for this vision.  
2. **Do** grow features behind `USBASP_HAS_DIAG` + diag caps on **USBasp2** boards.  
3. **Do** keep L1 USBasp sacred.  
4. **Do** pass live YEL0 `capabilities` acceptance before TRACE code. *(done)*  
5. Next: richer predicates / host-selectable arm only if needed — cycle ARM→trigger→post→freeze→replay is closed.
