# ACCEPTANCE-DIAG-TRIGGER-001

**Status:** **PARTIAL** (live PASS half closed 2026-08-26)  
**Depends on:** capabilities (`0x3f`), TRACE ring, trigger predicates  
**Hardware:** [USBASP2.md](../USBASP2.md) — YEL0 → **Канарейка** (ATmega8-on-Nano; do **not** sabotage this bench for FAIL)

## Contract split

| Class | What it proves | How |
|-------|----------------|-----|
| **A. Live hardware** | USBasp2 works; CAPS/session/trigger-no on PASS | Real YEL0 + target |
| **B. Synthetic fault** | `push → match → POST → FROZEN → TRACE_END` | Demo / harness through real trigger path |
| **C. Physical fault** *(later)* | Diag matches real ISP physics | Separate fault fixture — **not** the Nano PASS target |

Do not invent FAIL by breaking the working Nano ribbon.

## A. Live hardware — CLOSED (2026-08-26)

```text
YEL0 (USB) ── ISP ── Канарейка (ATmega8-on-Nano)
  CONNECT
    CAPS firmware=0x0000003f  board=0x00000002
    TRACE ✓  TRIGGER ✓  PRETRIGGER ✓  TIMESTAMP ✓
  SCK -B 8  (HW)  → ENABLEPROG PASS  → triggered=no
  SCK -B 22 (SW)  → ENABLEPROG PASS  → triggered=no
```

### Host semantics (normative)

`DIAG_CAPS` is emitted on **ISP `USBASP_FUNC_CONNECT`**, not on USB enumeration.

```text
usb plug-in     → device present (HIDUART)
avrdude CONNECT → diagnostic session + CAPS / TRACE_BEGIN / …
```

Host `capabilities` must wait for an ISP session (or use `--demo` / `--file`).  
Absence of CAPS while the stick is only on the bus means **no diag session**, not “no TRACE capability”.

```bash
# listener first, then ISP:
cargo run -p usbasp-ng-diag -- capabilities --serial YEL0 --timeout 15 &
avrdude -c usbasp -P usb:YEL0 -p m8 -B 8
```

## B. Synthetic FAIL — OPEN (demo path closed; firmware inject optional)

Must exercise the **same** path as a real FAIL:

```text
inject / demo ENABLEPROG FAIL frame
      ↓
trace_push(event)
      ↓
diag_trigger_match(event)   # ENABLEPROG_FAIL
      ↓
POST (N = USBASP_DIAG_POST_CAPTURE_EVENTS, default 16)
      ↓
FROZEN
      ↓
TRACE_END (triggered=YES, kind=ENABLEPROG_FAIL, post=…)
```

**Forbidden:** `state = FROZEN` without going through match.

### B1 — Host demo (available now)

```bash
cargo run -p usbasp-ng-diag -- demo enableprog_fail_sw
# expect: >> TRACE_END … triggered=YES  kind=ENABLEPROG_FAIL
cargo run -p usbasp-ng-diag -- capabilities --demo capabilities_yel0
# expect: firmware=0x0000003f
```

Model goldens: `firmware/tests/core/test_diag_trigger.py`.

### B2 — Firmware `USBASP_DIAG_TEST_INJECT` *(optional later)*

`diag_test_inject(DIAG_ENABLEPROG, FAIL)` only in TEST/DEMO builds, still via `diag_try_emit` → push → match. Not required to close class A.

## C. Physical FAIL fixture — NOT STARTED

Separate target (switchable RESET/SCK/MISO or dedicated fault board). Never the PASS Nano acceptance target.

## Checklist

- [x] A: CAPS `0x3f` on CONNECT  
- [x] A: ENABLEPROG PASS @ `-B 8` and `-B 22`, `triggered=no`  
- [x] B: demo `enableprog_fail_sw` → triggered YES  
- [ ] B: optional firmware inject build  
- [ ] C: physical fault fixture  

## Next product step (after this fixture)

Correlate USBasp2 Timer1 ↔ target UART ↔ FX2 physical capture — [PHYSICAL-CAPTURE-001](PHYSICAL-CAPTURE-001.md). Firmware frozen until class A (baseline dual capture) exists.
