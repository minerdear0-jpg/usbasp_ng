# ISP edge sniffer

Throwaway firmware: listen to RST/MOSI/MISO/SCK, dump CSV on UART. Not part of classic or HIDUART images.

## Why a third chip

This bench has **two** mega8 clones (no-dot programmer / yellow-dot DUT). You cannot sniff the ribbon while both are busy. Use a **Nano ATmega328P**, another mega8, or a USB FX2 (`fx2lafw` / PulseView).

Do **not** flash this onto no-dot. Do not flash it onto yellow unless you intend to replace NG.

## Arduino Nano (ATmega328P, 16 MHz) — preferred

USB-UART is already on PD0/PD1 (CH340/CP2102). Keep Nano USB plugged in. Crystal is 16 MHz.

| Nano | AVR | Yellow ISP (programmer) |
|------|-----|-------------------------|
| D10 | PB2 | RST |
| D11 | PB3 | MOSI |
| D12 | PB4 | MISO |
| D13 | PB5 | SCK (on-board LED shares this pin) |
| GND | GND | GND |

Wire **GND + four taps only**. If Nano and USBasp are both USB-powered, do not also jumper 5V to 5V.

```text
make -C host/isp-sniffer nano
# bootloader (typical Nano):
avrdude -c arduino -P /dev/ttyUSB0 -b 115200 -p atmega328p \
    -U flash:w:host/isp-sniffer/capture_sniffer-nano.hex:i
# or ISP from a USBasp, J2 closed on the Nano if it has a header:
avrdude -c usbasp -p atmega328p \
    -U flash:w:host/isp-sniffer/capture_sniffer-nano.hex:i
```

Serial 38400 8N1. Wait for `armed`, then reset the Nano between captures.

```text
python3 host/isp-sniffer/plot_capture.py --port /dev/ttyUSB0 --no-plot
# or save the terminal, then:
python3 host/isp-sniffer/plot_capture.py --file capture_b22.txt --no-plot
```

`plot_capture.py` reads `# F_CPU=` from the banner (16 MHz /8 → 0.5 µs/tick).

328P buffer is 480 edges (mega8 clone: 200). `-B 22` should fit. `-B 8` HW SPI is still easy to undersample in a busy loop; FX2 is better for the PASS reference.

## Spare ATmega8 clone (12 MHz)

Leave that clone’s USB unplugged (PB0/PB1 are D−/D+). Same PB2–PB5 taps. UART TX = PD1 @ 38400.

```text
make -C host/isp-sniffer
avrdude -c usbasp -p atmega8 -U flash:w:host/isp-sniffer/capture_sniffer.hex:i
```

## Capture sequence (yellow in USB, no-dot on the ribbon, JP3 open)

```text
avrdude -c usbasp -p atmega8 -B 8  -U signature:r:-:h    # PASS
avrdude -c usbasp -p atmega8 -B 22 -U signature:r:-:h    # FAIL
```

Save **both** logs. Score RST / MOSI `AC 53 00 00` / MISO third byte / SCK half-period / leftover HW SPI as in [`docs/SOFTWARE_SCK.md`](../../docs/SOFTWARE_SCK.md).

Expect after ~20 ms: MOSI `AC 53 00 00`. PASS if byte 2 MISO is `0x53`. Software SCK half-period at `-B 22` is on the order of 16 µs.
