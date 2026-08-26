# Windows (classic USBasp NG)

**Goal (x64):** plug in, no Zadig, no libusbK, no INF. Device Manager shows Microsoft **WinUSB**. Then modern avrdude `-c usbasp` (or `usbasp-clone`) programs the target.

Classic stays **one vendor-specific interface, EP0 only**. USBasp FUNC 1–16 / 127 is unchanged. Windows binds WinUSB via BOS + Microsoft OS 2.0 (`WINUSB`). MS OS layout decision and Win11 A/B: [USB_WINDOWS.md](USB_WINDOWS.md).

Use **classic** for Arduino and for Windows ISP (AVRDUDESS / avrdude). HIDUART is a Linux diagnostics stick — drivers may install on Windows, programming does not.

## Compatibility matrix

| Host | Tool | Driver | Classic NG | HIDUART |
|------|------|--------|------------|---------|
| Win10/11 x64 | avrdude 8.x MSVC | WinUSB | ✅ `usbasp` / `usbasp-clone` | ❌ ISP unreliable (drivers may still bind) |
| Win10/11 x64 | avrdude 8.x MinGW | WinUSB | ✅ | ❌ / unsupported for daily use |
| Win10/11 x64 | AVRDUDESS (current) | WinUSB | ✅ `usbasp` and `usbasp-clone` | ❌ drivers OK, programming does not work properly |
| Win10/11 x64 | Arduino IDE 1.8.19 bundled avrdude **6.3** | WinUSB | see [ACCEPTANCE-WIN11-USBASP-001](acceptance/ACCEPTANCE-WIN11-USBASP-001.md); not a matrix expansion | ❌ |
| Win10/11 x64 | Arduino IDE + replaced avrdude 8.x MSVC | WinUSB | ✅ expected | ❌ use classic |
| Linux | current avrdude | libusb | ✅ | ✅ lab / diagnostics |
| macOS | current avrdude | libusb | ✅ | ⚠️ |
| Win11 ARM64 | current | WinUSB | ⚠️ best-effort | ❌ |

Details for the Arduino 6.3 failure: [ARDUINO.md](ARDUINO.md), [KNOWN_ISSUES.md](KNOWN_ISSUES.md). Helper: [`arduino/replace-avrdude.ps1`](../arduino/replace-avrdude.ps1).

## Acceptance (Win10/11 x64)

1. Prefer a PC without a leftover libusb0/libusbK binding for `16c0:05dc` (or uninstall that OEM INF first).
2. Flash **classic** (`usbasp-ng-classic-atmega8.hex` from [releases](https://github.com/minerdear0-jpg/usbasp_ng/releases); packaging: [RELEASE.md](RELEASE.md)), plug in.
3. Device Manager: no unknown device; publisher **Microsoft**, driver **WinUSB**.
4. `bcdDevice` **2.03** (2.00 = pre-WinUSB; 2.02 = nested-MS-OS experiment; HIDUART uses **2.01** — same VID/PID, separate Windows hardware ID by design).
5. Modern avrdude: `avrdude -c usbasp -p atmega328p` (or `-c usbasp-clone`).
6. Arduino: Tools → Programmer → **USBasp** → Burn Bootloader / Upload Using Programmer. Prefer modern avrdude; stock 6.3 may work after Fischl string indices ([ARDUINO.md](ARDUINO.md)). Never Burn Bootloader with another programmer MCU on the ISP ribbon.

**First-plug WinUSB** on a clean machine is not proof that every PC with an old USBasp OEM INF will migrate automatically. Existing libusbK/libusb0 associations for `16c0:05dc` may stick until the device (and optionally the OEM INF) is removed. That is Windows driver-store behavior, not an NG bug.

Device Manager may still show the generic label **“WinUSB Device”**. That is Microsoft’s class name. USB product string remains `USBasp` (Bus reported device description). A custom INF just to rename the node is intentionally out of scope.

## Troubleshooting WinUSB

| Symptom | Likely cause | Action |
|---------|--------------|--------|
| Unknown device / needs Zadig | Old firmware / stale bind / wrong image | Flash classic with device-level MS OS `0x9E`, `bcdDevice` 2.03 ([USB_WINDOWS.md](USB_WINDOWS.md)). Uninstall device first. **Not** libusbK |
| WinUSB OK, avrdude 6.3 `cannot query manufacturer` | Arduino IDE bundled tool | Replace avrdude or use AVRDUDESS — **not** libusbK |
| Still libusbK/libusb0 on `16c0:05dc` | Stale driver store | Uninstall device + delete OEM INF, replug |
| `!` on 16C0:05DC, no publisher after nested MS OS `0xAE` | Classic nested layout failed Win11 auto-bind on this hardware | Use device-level classic; uninstall + replug ([USB_WINDOWS.md](USB_WINDOWS.md)) |
| `usbasp` fails, `usbasp-clone` works | Vendor string check | Normal for some clones; NG has Fischl strings so both should work |
| COM port missing | Expected | USBasp is not CDC |

## Bench 2026-08-26 (Win11 x64)

Yellow-dot classic NG, libusbK removed. **WinUSB on first plug** with device-level MS OS `0x9E`. Nested `0xAE` on the same stick → unbound (`!`, no publisher); see [USB_WINDOWS.md](USB_WINDOWS.md). AVRDUDESS read flash/EEPROM via `usbasp-clone`.

Later the same day: full destructive ISP via IDE Burn Bootloader through yellow — recorded as **[ACCEPTANCE-WIN11-USBASP-001](acceptance/ACCEPTANCE-WIN11-USBASP-001.md)**. Do not widen this matrix from that single run. **SW SCK** remains open: [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md).

USBasp is USB, not a COM port. AVRDUDESS **115200** is irrelevant; use `-B` / SCK for ISP speed. Arduino **Get Board Info** is Serial-only — irrelevant for USBasp.

## HIDUART on Windows

**Not supported as a Windows programmer.** Composite (vendor IF0 + HID): Device Manager often shows drivers installed correctly, but AVRDUDESS / typical avrdude builds do **not** program targets reliably. Flash **classic** for Windows ISP. Use HIDUART on Linux for the Diagnostics Plane.

Kits that include a small TXD/RXD adapter board can reach USART pins; the ISP ribbon alone still is not a target console. See [KNOWN_ISSUES.md](KNOWN_ISSUES.md).

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
