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
monotonic timestamp (Timer1 logical time)
      ↓
event ring (always-on) + optional TRACE ring
      ↓
trigger engine (e.g. ENABLEPROG_FAIL ∧ transport==SW)
      ↓
pre/post freeze around trigger
      ↓
HID EP2 stream (versioned wire)
      ↓
host record / replay
      ↓
TUI (Observe / Record / Analyze) — already started as `watch`
```

### 1. Timestamped event engine

Evolve beyond `t16 + a,b` toward a **monotonic 32-bit logical time** (Timer1-derived is enough; µs absolute not required). Timeline reads like a mini semantic analyzer:

```text
T=184203  RESET_ASSERT
T=184421  SCK_CONFIG SW 32k
T=185103  ENABLEPROG START
T=185111  SPI TX AC …
```

### 2. Dual timestamp (firmware ↔ physical)

Keep firmware `T` **inside** the event so FX2/PulseView can align:

```text
firmware ENABLEPROG  T=123456
FX2      SCK edge    T=123459   → Δ ≈ 2.7 µs
```

That is engineering diagnosis, not printf.

### 3. Dual rings + pre-trigger

Suggested SRAM split (illustrative, not a freeze):

```text
EVENT RING   ~256 semantic frames   always-on
TRACE RING   ~512 B raw ISP         opt-in, circular
```

On failure: **freeze**, keep N events **before** + M **after** trigger. First killer use-case historically: SW SCK ENABLEPROG (gate closed on USBasp2 for signature PASS; still the template for capture-mode science).

### 4. Trigger engine

Not `TRACE=everything`. Conditions such as:

- `event == ENABLEPROG && result == FAIL`
- `transport == SW`
- `SCK_ID == 7` (`-B 22`)
- combinations

Probe becomes a **condition-triggered recorder**.

### 5. Raw ISP capture (bounded)

Per-transaction `{T, TX, RX, transport}` — **not** every SCK edge in normal mode. Optional `DIAG_MODE_CAPTURE` may add instrumentation; must advertise:

```text
capture_mode = intrusive | non_intrusive
```

A measuring instrument must say when it perturbs the DUT timing.

### 6. Diagnostics capabilities (beyond TPI/3 MHz)

Host: `usbasp-ng-diag capabilities` → map, e.g.:

| Cap | Meaning |
|-----|---------|
| SESSION / SNAPSHOT | today’s semantic + fault snapshot |
| TIMESTAMP | monotonic T in stream |
| TRACE / PRETRIGGER / TRIGGER | L3 capture |
| SCK_STATS / ISP_TRACE | timing / transaction |
| **not** PHYSICAL_PIN_CAPTURE | honesty bit — we are not an FX2 |

### 7. Target monitor (UART) — tagged, separate source

Closed-loop bench (mega8-on-Nano + ttyUSB) is real: [USBASP2.md](USBASP2.md), `bench/mega8-nano-loop/`.

In the probe architecture, target UART is a **board capability** under Diagnostics, always tagged:

```text
source=PROGRAMMER | source=TARGET
```

Never merge into one unlabeled stream.

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
2. **Do** grow features behind `USBASP_HAS_DIAG` + future diag caps on **USBasp2** boards.  
3. **Do** keep L1 USBasp sacred.  
4. Next concrete increments: monotonic timestamp → capability bits → optional TRACE ring → trigger/pre-trigger — each with host/decode parity and replay.
