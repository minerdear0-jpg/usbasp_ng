# Software SCK ENABLEPROG failure

**Status:** known, parked. Do not patch `firmware/src/sck.c` until a waveform exists.

**Wanted:** a capture (FX2 `fx2lafw` / PulseView, or a third mega8 sniffer) of the same pair at **PASS `-B 8`** vs **FAIL `-B 22`**. Attach it to [issue #1](https://github.com/minerdear0-jpg/usbasp_ng/issues/1) or open a PR.

## Symptom

`USBASP_FUNC_ENABLEPROG` returns `0x01` (programming enable failed). avrdude exits non-zero. USB stays enumerated.

## Where it happens

Measured 2026-08-25 / 2026-08-26 on two cheap ATmega8 USBasp clones, 12 MHz, ISP: RST PB2, MOSI PB3, MISO PB4, SCK PB5. USB D− PB0, D+ PB1 (INT0 on D+).

Programmer = yellow-dot NG (classic or HIDUART). Target = no-dot mega8, J2 closed. Signature of the target is `1E 93 07`. JP3 open unless noted.

| Mode | SETISPSCK id | Approx SCK | Path | ENABLEPROG |
|------|----------------|------------|------|------------|
| AUTO | 0 → 1.5 MHz | HW SPI | `ispTransmit_hw` | PASS |
| `-B 0.25` | 13 | 3 MHz | HW | PASS |
| `-B 0.5` | 12 | 1.5 MHz | HW | PASS |
| `-B 8` | 8 | 93.75 kHz | HW (`>= USBASP_ISP_SCK_93_75`) | PASS |
| `-B 22` | 7 | 32 kHz | **software** | **FAIL `0x01`** |
| `-B 50` | 6 | 16 kHz | software | FAIL |
| `-B 250` | 1 | 4 kHz (`USBASP_ISP_SCK_0_5`) | software | FAIL |
| JP3 closed (PC2) | wire id 5 | ~8 kHz (`sck_sw_delay = 12`) | software | FAIL (AUTO and `-B 0.5`) |

Same FAIL on **classic L0** and **HIDUART**. It is not composite HID IRQ.

JP3 applies 8 kHz on the **wire** without storing that id as the host SETISPSCK value. Avrdude may still print 1.5 MHz after `-B 0.5` while the pins run ~8 kHz.

## What we already ruled out

- `usbFunctionSetup` runs from `usbPoll()` with I=1, not inside INT0.
- Software bitbang compiles to sbi/cbi; LED is off the `ispTransmit_sw` path.
- INT0 TX does RMW on whole PORTB/DDRB; ISP DDR is touched one pin at a time. USB stayed up after FAIL.
- `USB_COUNT_SOF` is off (would need INT0 on D−).

Root cause among RST / MOSI / MISO / SCK vs USB `out PORTB` during bitbang delays is **not scoped**.

## Reproduce (signature only)

JP3 **open**. Do not write the known-good programmer.

```text
avrdude -c usbasp -p atmega8 -B 8 -U signature:r:-:h    # PASS
avrdude -c usbasp -p atmega8 -B 22 -U signature:r:-:h   # FAIL ENABLEPROG
```

## Capture that would unblock `sck.c`

Probe programmer **RST, MOSI, MISO, SCK**, common GND. Trigger **RST falling** (CONNECT). Record:

1. `-B 8`  (hardware SPI, working programming-enable)
2. `-B 22` (software SPI, failing)

Need enough time for the ENABLEPROG `AC 53 00 00` exchange. PASS if byte 2 on MISO is `0x53`. At `-B 22` software half-period is on the order of 16 µs.

Third mega8 sniffer in this tree: [`host/isp-sniffer/`](../host/isp-sniffer/README.md) (`plot_capture.py`). Do not flash the sniffer onto the two bench clones unless replacing NG.

Cheap LA: sigrok `fx2lafw` + PulseView.

## Firmware

Threshold: `ispSetSCKOption()` uses hardware SPI for id `>= USBASP_ISP_SCK_93_75` (8). Ids 1–7 are software.

Release with this bug still present: [v0.1.2](https://github.com/minerdear0-jpg/usbasp_ng/releases/tag/v0.1.2).
