# ACCEPTANCE-SCK-SWEEP-001

**Status:** executed 2026-08-26 (Linux) — table filled; **scope capture still required** for SW FAIL  
**Depends on:** [ACCEPTANCE-WIN11-USBASP-001](ACCEPTANCE-WIN11-USBASP-001.md)

## Goal

Isolate **software SCK** failure with a clean control:

```text
same firmware, host, USB, target, cable, protocol
only: HW SPI  vs  SW bitbang
```

## Setup (this run)

| Field | Value |
|-------|--------|
| Date | 2026-08-26 |
| Host | Linux, **avrdude 8.2** |
| Programmer | yellow-dot classic NG (`bcdDevice` 2.03, MS OS `0x9E`) |
| Target | ATmega8 on ISP ribbon, signature `1E 93 07` |
| JP3 | open (assumed; not forced closed) |
| Programmer id | `-c usbasp` |

## Results

| `-B` | avrdude SCK | Expected path | ENABLEPROG | signature | flash read | notes |
|------|-------------|---------------|------------|-----------|------------|-------|
| AUTO / omit | (default) | HW ~1.5 MHz | ✅ | `1E 93 07` | ✅ 5530 B hex | full flash:r PASS |
| 1 | 750 kHz | HW | ✅ | `1E 93 07` | — | |
| 8 | 93.75 kHz | HW | ✅ | `1E 93 07` | — | historical PASS |
| 10 | 93.75 kHz | HW | ✅ | `1E 93 07` | — | still HW floor |
| **22** | **32 kHz** | **SW** | **❌ `0x01`** | — | — | reproduces SOFTWARE_SCK |
| 50 | 16 kHz | SW | **❌ `0x01`** | — | — | one earlier pass showed `FF FF FF` then fail; treat as SW fail |
| 250 | 4 kHz | SW | **❌ `0x01`** | — | — | |

### Commands used

```bash
avrdude -c usbasp -p atmega8 [-B N] -U signature:r:-:h
avrdude -c usbasp -p atmega8 -U flash:r:/tmp/usbasp-sck-auto.hex:i   # AUTO only
```

## Conclusion (so far)

- **HW SPI path** (AUTO, `-B 1`, `-B 8`, `-B 10`): ENABLEPROG + signature PASS; AUTO flash read PASS.  
- **SW bitbang path** (`-B 22`, `50`, `250`): ENABLEPROG **FAIL `0x01`** (target does not answer).  

Same stick, same target, same USB — only SCK mode changes. USB/WinUSB/protocol debt is not required to explain this; **software SCK remains the open defect**.

## Still needed

Waveform / Nano sniffer: **PASS `-B 8`** vs **FAIL `-B 22`** on RST / SCK / MOSI / MISO.  
Background: [SOFTWARE_SCK.md](../SOFTWARE_SCK.md). Issue: [#1](https://github.com/minerdear0-jpg/usbasp_ng/issues/1).

## Out of scope

Widening Windows/Arduino matrix; nested MS OS; TPI silicon.
