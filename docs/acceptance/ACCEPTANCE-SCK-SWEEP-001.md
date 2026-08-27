# ACCEPTANCE-SCK-SWEEP-001

**Status:** **CLOSED** (2026-08-26 evening)  
**Depends on:** [ACCEPTANCE-WIN11-USBASP-001](ACCEPTANCE-WIN11-USBASP-001.md)  
**Closing hardware:** [USBASP2.md](../USBASP2.md)

## Formal state (closed)

```text
ACCEPTANCE-SCK-SWEEP-001  CLOSED

Closing run (USBasp2 → mega8-on-Nano-PCB):
  B=8   HW   PASS  signature 1E 93 07
  B=22  SW   PASS  signature 1E 93 07

Earlier isolation (classic mega8 programmer → mega8 clone target):
  HW SPI   PASS
  SW SPI   FAIL ENABLEPROG=0x01
  → isolated to SW-SCK on that pair; not a permanent product blocker.

Conclusion:
  Software SCK works with USBasp2 (ATmega328P programmer) against
  ATmega8 mounted on a Nano PCB. Gate closed — no further firmware
  speculation or mandatory FX2 capture for **USBasp2** release.

Classic mega8 programmer remains a shipping daily image; SW ENABLEPROG
FAIL on mega8↔mega8 clone is a known limitation of that pair, not an
open USBasp2 blocker. Do not treat compact mega8 HIDUART TRACE as a
replay of the classic failure (different USB image; HIDUART mega8 frozen).
```

## Closing evidence (2026-08-26)

| Field | Value |
|-------|--------|
| Programmer | **USBasp2** — yellow-dot, ATmega328P + HIDUART (`YEL0`, `bcdDevice` 2.01) |
| Target | **ATmega8 soldered on a Nano PCB** (not a stock Nano 328P — that board failed earlier) |
| Host | Linux, avrdude, `-c usbasp` |
| `-B 8` | PASS — signature `1E 93 07` |
| `-B 22` | PASS — signature `1E 93 07` |

Diag on the same stick showed ENABLEPROG **PASS** (RX `… 53 00`) during signature sessions.

## Historical isolation (same day, earlier)

Programmer was yellow **classic mega8** NG; target another mega8 clone on ribbon. HW PASS / SW FAIL `0x01`. That record remains below for archaeology; it does **not** keep the acceptance gate open.

### Historical results table

| `-B` | Path | ENABLEPROG (mega8 prog → mega8 target) |
|------|------|----------------------------------------|
| AUTO / 1 / 8 / 10 | HW | PASS |
| 22 / 50 / 250 | SW | FAIL `0x01` |

Background notes: [SOFTWARE_SCK.md](../SOFTWARE_SCK.md). Issue: [#1](https://github.com/minerdear0-jpg/usbasp_ng/issues/1).
