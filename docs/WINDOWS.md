# Windows (classic USBasp NG)

**Goal (x64):** plug in, no Zadig, no libusbK, no INF. Device Manager shows Microsoft **WinUSB**. Then `avrdude -c usbasp` programs the target.

Classic stays **one vendor-specific interface, EP0 only**. USBasp FUNC 1–16 / 127 is unchanged. Windows binds WinUSB because of BOS + Microsoft OS 2.0 (`WINUSB`).

HIDUART is a different topology. Use classic for Arduino and for MSVC avrdude.

## Acceptance (Win10/11 x64)

1. Fresh PC, no previous USBasp driver experiment if you can (old libusb0/libusbK for `16c0:05dc` can stick in the driver store).
2. Flash **classic** (`usbasp.hex`), plug in.
3. Device Manager: no unknown device; driver publisher Microsoft, WinUSB.
4. Modern avrdude (7.x / 8.x MSVC): `avrdude -c usbasp -p atmega328p` (or `atmega8`).
5. Arduino IDE 2.x: Tools → Programmer → **USBasp** → Burn Bootloader / Upload Using Programmer.

`bcdDevice` is **2.02**. A stick that still enumerates as 2.00 is old firmware without this bind.

If Windows already bound libusb-win32 / libusbK to `16c0:05dc`, uninstall that driver (and delete the OEM INF) or the OS may ignore WinUSB. That is a host cache problem, not a missing Zadig step.

## Bench 2026-08-26 (Win11 x64)

Yellow-dot classic NG, libusbK removed. **WinUSB bound on first plug** (Microsoft, no Zadig, no INF).

AVRDUDESS (current) opened the stick as **usbasp-clone** and read flash + EEPROM of the other programmer on the ribbon.

- USBasp is USB, not a COM port. The **115200** field in AVRDUDESS does not set ISP clock; ignore it (use `-B` / SCK in avrdude if you need a bit rate).
- `-c usbasp-clone` skips the Fischl vendor/product string check. NG still enumerates `www.fischl.de` / `USBasp`, so `-c usbasp` should also work; clone is the safer AVRDUDESS pick for random sticks.
- Arduino IDE **1.8.19** Burn Bootloader: bundled **avrdude 6.3-20190619** → `cannot query manufacturer` / cannot open WinUSB. Same stick works in AVRDUDESS. Fix: newer avrdude for the IDE, or use AVRDUDESS — see [ARDUINO.md](ARDUINO.md). Not a firmware rollback.

## ARM64

Best-effort. avrdude documents Windows ARM64 USB limitations. Do not treat ARM64 as a release gate.

## What we will not do

- Do not ship Zadig as the supported path.
- Do not add a second USB interface on classic.
- Do not invent an Arduino-specific USB protocol.

## Linux / macOS

Unchanged: libusb. BOS is ignored.

## Later

A host `usbaspctl info` tool (VID/PID, profile, WinUSB vs composite) is useful for Windows support; it is not required for Arduino.
