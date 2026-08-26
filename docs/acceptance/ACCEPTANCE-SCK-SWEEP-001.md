# ACCEPTANCE-SCK-SWEEP-001 (plan)

**Status:** planned — not executed  
**Depends on:** [ACCEPTANCE-WIN11-USBASP-001](ACCEPTANCE-WIN11-USBASP-001.md) (HW ISP / WinUSB pipeline already PASS)

## Goal

Isolate **software SCK** failure with a clean control:

```text
same firmware, host, USB, target, cable, protocol
only: HW SPI  vs  SW bitbang
```

Not Burn Bootloader. Not Arduino IDE UI. Command-line avrdude + optional scope.

## Setup

| Role | Stick |
|------|--------|
| Programmer | yellow-dot classic NG (current) **or** restored no-dot |
| Target | ATmega8 (or 328P) on ribbon with known-good signature |
| JP3 | **open** unless a dedicated JP3 row is filled |
| Host | Linux or Win11; record avrdude version |
| Scope / sniffer | RST, SCK, MOSI, MISO — PASS `-B 8` vs FAIL `-B 22` minimum |

## Commands (fill results)

Use one programmer id throughout (`-c usbasp` or `usbasp-clone`). Example target `atmega8`:

```bash
avrdude -c usbasp -p atmega8 -B <N> -v -U signature:r:-:h
avrdude -c usbasp -p atmega8 -B <N> -U flash:r:/tmp/t.hex:i   # after ENABLEPROG works
```

| `-B` | Expected path | ENABLEPROG | signature | flash read | notes / measured SCK |
|------|---------------|------------|-----------|------------|----------------------|
| AUTO / omit | HW ~1.5 MHz | | | | |
| 1 | HW | | | | |
| 10 | HW / boundary | | | | |
| 22 | **SW** | | | | historical FAIL `0x01` |
| 50 | SW | | | | |
| 250 | SW | | | | |

Waveform: attach capture or link to issue [#1](https://github.com/minerdear0-jpg/usbasp_ng/issues/1). Background: [SOFTWARE_SCK.md](../SOFTWARE_SCK.md).

## Pass criteria

- Documented table for the rows above  
- At least one HW row PASS and one SW row measured (PASS or FAIL with log)  
- Scope or Nano sniffer for SW FAIL vs HW PASS if SW still fails  

## Out of scope

Widening Windows/Arduino matrix; nested MS OS; TPI silicon.
