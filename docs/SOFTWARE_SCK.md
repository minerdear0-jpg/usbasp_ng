# Software SCK ENABLEPROG failure

**Status:** open; next controlled test [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md). Baseline WinUSB/HW ISP pipeline: [ACCEPTANCE-WIN11-USBASP-001](acceptance/ACCEPTANCE-WIN11-USBASP-001.md).

RST PORTB RMW is `cli`/`SREG` (same as MOSI/SCK). **Bench 2026-08-26:** wrap did **not** change the symptom. Programmer yellow-dot HIDUART `YEL0`, target no-dot `1E 93 07`, JP3 open. `-B 8` / `-B 0.5` PASS; `-B 22` / `-B 50` still ENABLEPROG `0x01`. Bitbang algorithm unchanged. Next: **HW vs SW sweep + waveform**, not more IRQ wrapping.

**Wanted:** a capture (FX2 `fx2lafw` / PulseView, or Nano 328P sniffer) of **PASS `-B 8`** vs **FAIL `-B 22`**. Attach to [issue #1](https://github.com/minerdear0-jpg/usbasp_ng/issues/1) or a PR.

Review notes: [`reports/2026-08-26-master-review-2-rst-rmw.md`](../reports/2026-08-26-master-review-2-rst-rmw.md). Sample-MISO-before-SCK-rise matches Fischl 2011; that is not the bug to chase.

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

JP3 applies 8 kHz on the **wire** without storing that id as the host SETISPSCK value (`requested_sck` vs `effective_sck` on the pins). Avrdude may still print 1.5 MHz after `-B 0.5` while the pins run ~8 kHz.

## What we already ruled out

- `usbFunctionSetup` runs from `usbPoll()` with I=1, not inside INT0.
- Software bitbang compiles to sbi/cbi; LED is off the `ispTransmit_sw` path.
- INT0 TX does RMW on whole PORTB/DDRB; ISP DDR is touched one pin at a time. USB stayed up after FAIL.
- `USB_COUNT_SOF` is off (would need INT0 on D−).
- RST `PORTB` writes now use `isp_out_set_bit` / `isp_out_clr_bit` (`cli` around RMW). MOSI/SCK bitbang already did. Idle SCK/MOSI/MISO levels on connect/disconnect are still bare RMW.

Root cause among remaining PORTB races vs USB `out PORTB` during bitbang delays is **not scoped**. RST `cli` is ruled out as the ENABLEPROG failure (2026-08-26).

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

When the traces exist, score them (do not stop at “FAIL confirmed”):

1. **RST** — one clean fall on both runs, same amplitude? Difference → RST/`clockWait(62)`, not SCK.
2. **MOSI** — `AC 53 00 00` bit-correct on `-B 22`? Yes → RX or target. Extra/missing bits only on SW → bitbang path.
3. **MISO third byte** — stuck 0 (target not in ISP); toggles but not `0x53` (sample phase); smooth wrong byte (bit/byte slip).
4. **SCK** — ~16 µs half-period at `-B 22`? Glitches at `cli`/`SREG` edges?
5. **Not leftover HW SPI** — pins actually bitbang, not frozen.

Checklist copy: [`reports/2026-08-26-sw-sck-capture-plan.md`](../reports/2026-08-26-sw-sck-capture-plan.md).

Third mega8 sniffer in this tree: [`host/isp-sniffer/`](../host/isp-sniffer/README.md) (`plot_capture.py`). Arduino Nano 328P is the intended third chip (keep its USB plugged). Do not flash the sniffer onto the two bench clones unless replacing NG.

Cheap LA: sigrok `fx2lafw` + PulseView.

## Firmware

Threshold: `ispSetSCKOption()` uses hardware SPI for id `>= USBASP_ISP_SCK_93_75` (8). Ids 1–7 are software.

**SW SCK contract** (until a capture fills the table): requested frequency is an **upper bound**. Cycle-count delay is the **minimum** half-period. INT0 may only stretch a phase.

| Requested | min half | measured high | measured low | max jitter |
|-----------|----------|---------------|--------------|------------|
| 32 kHz (`-B 22`) | ~16 µs | ? | ? | ? |
| 16 kHz (`-B 50`) | ~32 µs | ? | ? | ? |
| 4 kHz (`-B 250`) | ~128 µs | ? | ? | ? |
| 8 kHz JP3 | ~62.5 µs | ? | ? | ? |

USB: INT0 ISR returns → main `usbPoll()` → `usbFunctionSetup()` → ISP. PORTB RMW for MOSI/SCK/RST is atomic vs that ISR.

Release with this bug still present: [v0.1.2](https://github.com/minerdear0-jpg/usbasp_ng/releases/tag/v0.1.2).
