# ISP edge sniffer (third ATmega8)

Throwaway firmware: listen to RST/MOSI/MISO/SCK, dump CSV on UART. Not part of classic or HIDUART images.

## Bench constraint

This repo’s table has **two** mega8 clones (no-dot, yellow-dot). You cannot sniff the ISP ribbon while both are busy as programmer + target. Need a **third** mega8, or a USB FX2 logic analyzer (`fx2lafw` / PulseView), or later a PD1 dump from yellow itself (no extra USBasp FUNC).

Do **not** flash `capture_sniffer.hex` onto no-dot. Do not flash it onto yellow unless you intend to replace NG.

## Hardware

Clone crystal is **12 MHz**. `F_CPU=16000000` only if that chip really has 16 MHz.

Taps on the **programmer** (yellow) ISP pins, inputs, **no pull-ups**, common GND:

| Sniffer | Yellow |
|---------|--------|
| PB2 | RST |
| PB3 | MOSI |
| PB4 | MISO |
| PB5 | SCK |
| GND | GND |

Sniffer TX **PD1** → USB-UART 38400 8N1. Leave sniffer USB unplugged if the chip is a USBasp clone (USB is PB0/PB1 on NG boards).

## Build / flash (third chip only)

```text
make -C host/isp-sniffer
avrdude -c usbasp -p atmega8 -U flash:w:host/isp-sniffer/capture_sniffer.hex:i
```

Open the UART, wait until `armed`, then on yellow (JP3 **open**):

```text
avrdude -c usbasp -p atmega8 -B 22 -U signature:r:-:h
avrdude -c usbasp -p atmega8 -B 8 -U signature:r:-:h
```

Save **both** terminal logs (`capture_b8.txt`, `capture_b22.txt`), decode:

```text
python3 host/isp-sniffer/plot_capture.py --file capture_b8.txt --no-plot
python3 host/isp-sniffer/plot_capture.py --file capture_b22.txt --no-plot
```

Score RST / MOSI `AC 53 00 00` / MISO third byte / SCK half-period / leftover HW SPI as in [`docs/SOFTWARE_SCK.md`](../../docs/SOFTWARE_SCK.md).

Expect after ~20 ms: MOSI `AC 53 00 00`. PASS if byte 2 MISO is `0x53`. Software SCK half-period at `-B 22` is on the order of 16 µs.

`-B 8` / `-B 0.5` may miss edges on this busy-loop (HW SPI is fast). For the five-point PASS vs FAIL compare, prefer FX2/`fx2lafw` so `-B 8` is actually in the log. This mega8 sniffer is still useful for **software SCK** (`-B 22`).
