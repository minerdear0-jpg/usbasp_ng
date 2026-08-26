# USBasp2

**USBasp2** is a USBasp NG programmer whose MCU is an **ATmega328P** (typically TQFP-32 reflowed onto a common mega8 clone PCB, same 12 MHz crystal and ISP/USB wiring).

Same host protocol as Fischl USBasp (`-c usbasp` / `usbasp-clone`). More flash/RAM than mega8 → room to grow from “a few diagnostic bytes” into an **ISP development probe** (observability / measurement / capture) while L1 stays ordinary USBasp. Architecture: [DIAGNOSTICS_PROBE.md](DIAGNOSTICS_PROBE.md). Shipping diag wire today: [DIAGNOSTICS.md](DIAGNOSTICS.md).

**Not** a cheap AVR-ICE: no debugWIRE/JTAG/UPDI claim.

## Hardware identity (bench)

| Name | Role |
|------|------|
| **USBasp2** (yellow-dot) | ATmega328P on the stick; USB to host when programming targets |
| **no-dot** | Classic mega8 stick; used to ISP-flash USBasp2 (J2 closed on USBasp2) |
| **Канарейка** | ATmega8 soldered on a Nano PCB, on the ISP ribbon; J2 on USBasp2 **open**. Closed-loop DUT (CH340 UART + LEDs). |

USB when running HIDUART: `bcdDevice` **2.01**, iSerial often `YEL0`, composite + EP2 diag.

## Board profiles

| CMake board | Image |
|-------------|--------|
| `usbasp-hiduart-atmega328p` | **Default USBasp2 lab image** — HIDUART + Diagnostics Plane |
| `usbasp-atmega328p` | Classic (no HID/diag) on 328P |

```bash
SERIAL=YEL0 make -C firmware BOARD=usbasp-hiduart-atmega328p hex
# ISP from no-dot, J2 closed on USBasp2:
avrdude -c usbasp -p m328p -B 8 \
  -U flash:w:firmware/build/usbasp-hiduart-atmega328p/usbasp-hiduart.hex:i \
  -U eeprom:w:firmware/build/usbasp-hiduart-atmega328p/usbasp-hiduart.eep:i
```

Size budget (typical): ~8.5 KiB flash / ~0.6 KiB RAM of 32 KiB / 2 KiB.

Fuses on a working crystal part are often already `lfuse=0xff`; board recipe documents `hfuse=0xde` `lfuse=0xff`. Do not blast fuses blindly.

## Smoke (USBasp2 → mega8 target)

```bash
python3 host/usbaspctl.py info          # expect hiduart, serial YEL0
avrdude -c usbasp -p m8 -B 8  -U signature:r:-:h   # 1E 93 07
avrdude -c usbasp -p m8 -B 22 -U signature:r:-:h   # SW SCK
avrdude -c usbasp-clone -p m8 -U signature:r:-:h
./diagplane.bin watch --serial YEL0     # other terminal during avrdude
```

Measured 2026-08-26 (yellow **USBasp2** + **ATmega8 on Nano PCB** on ribbon): signature `-B 8` / `-B 22` PASS; fuses/flash read/EEPROM W+V+restore PASS; diag ENABLEPROG PASS; `-c usbasp` and `usbasp-clone` OK. Closes [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md).

## vs mega8 HIDUART

| | mega8 HIDUART | **USBasp2** (328P) |
|---|---|---|
| Flash headroom | ~70 B left | tens of KiB free |
| Role | legacy lab stick | preferred Diagnostics Plane stick |
| Windows daily ISP | still use **classic** mega8 | same — classic for Windows |

TPI remains gated (`USBASP_HAS_TPI=0`) until tiny acceptance — extra flash does not by itself enable TPI.

## Closed-loop bench (Канарейка)

**Канарейка** = ATmega8 on a Nano PCB + CH340 (`/dev/ttyUSB0`) + 4 LEDs + RESET.
Клетка: YEL0 USB → ISP ribbon → Канарейка; CH340 → UART Канарейки.

```bash
./dist/diagplane.bin watch --serial YEL0          # terminal A
make -C bench/mega8-nano-loop flash               # terminal B (wipes Optiboot)
# or: make -C bench/mega8-diag-oracle flash       # Channel 2 oracle
screen /dev/ttyUSB0 115200                        # expect banner / @READY
```

See [`bench/mega8-nano-loop/`](../bench/mega8-nano-loop/) (smoke) and [`bench/mega8-diag-oracle/`](../bench/mega8-diag-oracle/) (dual-truth). Do not burn Optiboot and the oracle canary pages onto the same chip: both own `0x1E00`.
