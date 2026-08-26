# ACCEPTANCE-SCK-SWEEP-001

**Status:** isolation complete (2026-08-26, Linux)  
**Depends on:** [ACCEPTANCE-WIN11-USBASP-001](ACCEPTANCE-WIN11-USBASP-001.md)

## Formal state

```text
ACCEPTANCE-SCK-SWEEP-001

HW SPI
  AUTO  PASS
  B=1   PASS
  B=8   PASS
  B=10  PASS

SW SPI
  B=22  FAIL  ENABLEPROG=0x01
  B=50  FAIL  ENABLEPROG=0x01
  B=250 FAIL  ENABLEPROG=0x01

Control:
  same host
  same firmware
  same target
  same cable
  same USB path

Conclusion:
  failure isolated to SW-SCK path.
  Root cause unknown.
  No firmware modification after isolation.

Next evidence:
  RST/SCK/MOSI/MISO capture for B=8 vs B=22.
```

## Conclusion (prose)

Failure is **isolated to the software SCK (bitbang) path**. Root cause is **unknown**. Do **not** change `ispTransmit_sw` / related firmware until bus evidence exists — further source-only reasoning is speculation.

At this stage the hardware must speak: scope or Nano sniffer on **RST / SCK / MOSI / MISO**, PASS `-B 8` vs FAIL `-B 22`.

## Setup (this run)

| Field | Value |
|-------|--------|
| Date | 2026-08-26 |
| Host | Linux, **avrdude 8.2** |
| Programmer | yellow-dot classic NG (`bcdDevice` 2.03, MS OS `0x9E`) |
| Target | ATmega8 on ISP ribbon, signature `1E 93 07` |
| JP3 | open |
| Programmer id | `-c usbasp` |

## Results table

| `-B` | avrdude SCK | Path | ENABLEPROG | signature | flash read |
|------|-------------|------|------------|-----------|------------|
| AUTO | default | HW | PASS | `1E 93 07` | PASS (5530 B) |
| 1 | 750 kHz | HW | PASS | `1E 93 07` | — |
| 8 | 93.75 kHz | HW | PASS | `1E 93 07` | — |
| 10 | 93.75 kHz | HW | PASS | `1E 93 07` | — |
| **22** | **32 kHz** | **SW** | **FAIL `0x01`** | — | — |
| 50 | 16 kHz | SW | FAIL `0x01` | — | — |
| 250 | 4 kHz | SW | FAIL `0x01` | — | — |

```bash
avrdude -c usbasp -p atmega8 [-B N] -U signature:r:-:h
avrdude -c usbasp -p atmega8 -U flash:r:/tmp/usbasp-sck-auto.hex:i   # AUTO
```

## Next evidence only

| Capture | Purpose |
|---------|---------|
| `-B 8` (HW PASS) | Reference waveform |
| `-B 22` (SW FAIL) | Compare RST/SCK/MOSI/MISO timing and levels |

Background: [SOFTWARE_SCK.md](../SOFTWARE_SCK.md). Issue: [#1](https://github.com/minerdear0-jpg/usbasp_ng/issues/1). Nano sniffer: `host/isp-sniffer/`.

## Out of scope until capture

Firmware edits to SW SCK; widening Windows/Arduino matrix; TPI silicon.
