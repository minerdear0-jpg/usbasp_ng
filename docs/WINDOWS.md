# Windows (classic USBasp NG)

**Goal (x64):** plug in, no Zadig, no libusbK, no INF. Device Manager shows Microsoft **WinUSB**. Then modern avrdude `-c usbasp` (or `usbasp-clone`) programs the target.

Classic stays **one vendor-specific interface, EP0 only**. USBasp FUNC 1–16 / 127 is unchanged. Windows binds WinUSB via BOS + Microsoft OS 2.0 (`WINUSB`).

Use **classic** for Arduino and for MSVC avrdude. HIDUART is composite — different story (see below).

## Compatibility matrix

| Host | Tool | Driver | Classic NG | HIDUART |
|------|------|--------|------------|---------|
| Win10/11 x64 | avrdude 8.x MSVC | WinUSB | ✅ bench | ⚠️ prefer MinGW/libusb |
| Win10/11 x64 | avrdude 8.x MinGW | WinUSB | ✅ | ✅ / check |
| Win10/11 x64 | AVRDUDESS (current) | WinUSB | ✅ bench (`usbasp-clone`) | ⚠️ |
| Win10/11 x64 | Arduino IDE 1.8.19 bundled avrdude **6.3** | WinUSB | ❌ host tool too old | ❌ |
| Win10/11 x64 | Arduino IDE + replaced avrdude 8.x MSVC | WinUSB | ✅ expected | use classic |
| Linux | current avrdude | libusb | ✅ | ✅ |
| macOS | current avrdude | libusb | ✅ | ✅ |
| Win11 ARM64 | current | WinUSB | ⚠️ best-effort | ⚠️ |

Details for the Arduino 6.3 failure: [ARDUINO.md](ARDUINO.md), [KNOWN_ISSUES.md](KNOWN_ISSUES.md). Helper: [`arduino/replace-avrdude.ps1`](../arduino/replace-avrdude.ps1).

## Acceptance (Win10/11 x64)

1. Prefer a PC without a leftover libusb0/libusbK binding for `16c0:05dc` (or uninstall that OEM INF first).
2. Flash **classic** (`usbasp.hex` from [releases](https://github.com/minerdear0-jpg/usbasp_ng/releases)), plug in.
3. Device Manager: no unknown device; publisher **Microsoft**, driver **WinUSB**.
4. `bcdDevice` **2.02** (2.00 = pre-WinUSB classic; HIDUART uses **2.01** — same VID/PID, separate Windows hardware ID by design).
5. Modern avrdude: `avrdude -c usbasp -p atmega328p` (or `-c usbasp-clone`).
6. Arduino: only after step 5’s avrdude is new enough — Tools → Programmer → **USBasp** → Burn Bootloader / Upload Using Programmer.

**First-plug WinUSB** on a clean machine is not proof that every PC with an old USBasp OEM INF will migrate automatically. Existing libusbK/libusb0 associations for `16c0:05dc` may stick until the device (and optionally the OEM INF) is removed. That is Windows driver-store behavior, not an NG bug.

Device Manager may still show the generic label **“WinUSB Device”**. That is Microsoft’s class name. USB product string remains `USBasp` (Bus reported device description). A custom INF just to rename the node is intentionally out of scope.

## Troubleshooting WinUSB

| Symptom | Likely cause | Action |
|---------|--------------|--------|
| Unknown device / needs Zadig | Old firmware without BOS/MS OS 2.0 | Flash classic ≥ v0.2.0 |
| WinUSB OK, avrdude 6.3 `cannot query manufacturer` | Arduino IDE bundled tool | Replace avrdude or use AVRDUDESS — **not** libusbK |
| Still libusbK/libusb0 on `16c0:05dc` | Stale driver store | Uninstall device + delete OEM INF, replug |
| `usbasp` fails, `usbasp-clone` works | Vendor string check | Normal for some clones; NG has Fischl strings so both should work |
| COM port missing | Expected | USBasp is not CDC |

## Bench 2026-08-26 (Win11 x64)

Yellow-dot classic NG, libusbK removed. **WinUSB on first plug.** AVRDUDESS read flash/EEPROM via `usbasp-clone`. Arduino 1.8.19 + avrdude 6.3 failed as in the matrix.

USBasp is USB, not a COM port. AVRDUDESS **115200** is irrelevant; use `-B` / SCK for ISP speed.

## HIDUART on Windows

Composite: vendor IF0 + HID. MSVC avrdude/libwinusb often cannot open it. Prefer classic for programming on Windows, or MinGW/libusb avrdude. HID UART uses the built-in HID class driver (no Zadig).

## ARM64

Best-effort. avrdude documents Windows ARM64 USB limits. Not a release gate.

## What we will not do

- Do not ship Zadig as the supported path.
- Do not add a second USB interface on classic.
- Do not invent an Arduino-specific USB protocol.
- Do not reinstall libusbK to “fix” Arduino 1.8.

## Linux / macOS

Unchanged: libusb. BOS is ignored. Diagnostics: `python3 host/usbaspctl.py info`.

## Host tools

- [`host/usbaspctl.py`](../host/usbaspctl.py) — `info` / `info --json` / `windows-hints` (Linux pyusb; hints work without a stick)
- Descriptors: [USB_DESCRIPTORS.md](USB_DESCRIPTORS.md), execution model: [USB_EXECUTION.md](USB_EXECUTION.md)
