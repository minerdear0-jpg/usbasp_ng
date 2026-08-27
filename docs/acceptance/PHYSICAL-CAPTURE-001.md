# PHYSICAL-CAPTURE-001

**Status:** **PARTIAL** (USBASP_INTERNAL baseline 2026-08-27; PHYSICAL_CAPTURE still missing)  
**Depends on:** Evidence v1 frozen ([EVIDENCE.md](../EVIDENCE.md)), YEL0 HIDUART, канарейка intact  
**Hardware:** [USBASP2.md](../USBASP2.md) — YEL0 → канарейка. Independent probe on ISP lines (FX2 / `fx2lafw` / PulseView). Do **not** flash sniffer firmware onto no-dot, YEL0, or the canary.

Firmware stays frozen until class **A** has a real dual recording. No new EP2 events for this gate.

```text
USBasp2
   │
   ├── Diagplane ──→ USBASP_INTERNAL  (.bin / .usbasp2e)
   │
   └── ISP ── target
             │
             └── FX2 ──→ PHYSICAL_CAPTURE
```

CAPS bit `PHYSICAL_CAPTURE` is **not** this experiment. Evidence exists only when a capture file with `capture_id` is stored.

## Evidence hierarchy ≠ source hierarchy

Do not rank sources:

```text
FX2 > firmware     forbidden
firmware > FX2     forbidden
```

Each source has a **scope**:

| Source | Knows |
|--------|--------|
| USBASP_INTERNAL | what firmware attempted / observed at the MCU |
| PHYSICAL_CAPTURE | what happened on the probed line |
| HOST_PROTOCOL | what the host requested |
| TARGET_UART | what the target reported |

Conflict is disagreement between scopes, not a verdict of who is lying.

```text
firmware: "I drove RESET (intent / GPIO)"
FX2:      "the wire was LOW"  or  "the wire never moved"
```

Possible explanations include GPIO config, pin map, electrical fault, capture error, timestamp mismatch. **Unknown stays unknown.** Diagplane must emit `EVIDENCE.CONFLICT` (when both sources are recorded and disagree) and **must not assign blame**.

## A. Baseline dual capture — OPEN (do this first)

Ordinary programming, **not** an injected fault.

```text
listener:  diagplane record/watch --serial YEL0  → capture.bin
sniffer:   FX2 on RST / SCK / MOSI / MISO
host:      avrdude -c usbasp -P usb:YEL0 -p m8 -B 8 -U signature:r:-:h
           CONNECT → ENABLEPROG → read signature → DISCONNECT
```

Record together:

| Artifact | Source |
|----------|--------|
| Diagplane `.bin` / `.usbasp2e` | USBASP_INTERNAL |
| FX2 / PulseView session | PHYSICAL_CAPTURE |

Fill after the run:

| Field | Value |
|-------|--------|
| Capture tool | FX2 — **not connected** this run |
| Sample rate | — |
| Channels | — |
| session_id (EP2) | `18cfba1d86d5e362` (last session in file) |
| capture_id (diagplane) | `ceb468f7` |
| Diagplane file | [`host/goldens/evidence/captures/cage_b8_20260827_233651.bin`](../../host/goldens/evidence/captures/) |
| FX2 file | — |
| Host command | `avrdude -c usbasp -P usb:YEL0 -p m8 -B 8 -U signature:r:-:h` → `1E9307` |
| Host analyze | ENABLEPROG PASS; file VERDICT **PASS** (2 sessions; first had LINE FAIL pin=0x14 then EP PASS) |

**Expected (manual correlation is enough — no FX2 analyzer required):**

- RESET assertion visible on the wire in a window that can be lined up with `DIAG_RESET` ASSERT (USBASP timestamp ↔ FX2 timestamp)
- SCK activity on the wire during ENABLEPROG
- ENABLEPROG PASS on EP2 (`RX` contains `0x53`) correlatable with that burst

**Claims this class may support later (not invented from CAPS):**

- `RESET_ASSERTION_CONFIRMED`
- `SCK_ACTIVITY_CONFIRMED`
- ENABLEPROG correlation

**Negative:** no automatic blame assignment. Success here is **time-aligned dual observation**, not a new verdict enum.

## B. Conflict — OPEN (only after A)

Prefer a **real** disagreement of observations, not a forged EP2 frame.

Controlled example (if the bench can produce it without killing the canary):

```text
USBASP_INTERNAL:  RESET ASSERT
PHYSICAL_CAPTURE: RESET did not assert (stayed HIGH / never an edge)
```

**Expected:**

```text
EVIDENCE.CONFLICT
  source A = USBASP_INTERNAL
  source B = PHYSICAL_CAPTURE
  claim    = RESET_ASSERTION
  confidence = UNKNOWN
  causal_relevance = UNKNOWN
```

Engine must **not** decide which source is wrong. `FAIL_CONFIRMED` is not this class unless a later, separate protocol failure is independently confirmed.

Do not sabotage the Nano canary to invent FAIL ([ACCEPTANCE-DIAG-TRIGGER-001](ACCEPTANCE-DIAG-TRIGGER-001.md) class C).

## C. Engine ingest — NOT THIS FILE

Attaching FX2 as `sources.physical` inside `.usbasp2e` is a later host grain. Class A may be closed with sidecar files and a lab note of Δt.

## Closed when

- [ ] Class A: one real `-B 8` signature session with both recordings and a timestamp alignment note
- [ ] Class B: one conflict pair that yields `EVIDENCE.CONFLICT` without blame
- [ ] This file filled with tool / rate / channels / ids / paths

Then `firmware_build_id` on the wire is allowed to be designed. EEPROM still waits until independent observations are proven storeable and comparable.
