# Known issues

Tracked for release notes. Compatibility contract remains [`COMPATIBILITY.md`](COMPATIBILITY.md).

## Existing libusbK / libusb0 bindings (Windows)

**Status:** host OS / driver-store behavior.

PCs that previously used Zadig/libusbK for `16c0:05dc` may keep that association after flashing classic NG. Uninstall the device in Device Manager (delete driver if offered), remove the OEM INF if needed, then replug. Clean first-plug WinUSB does not guarantee migration on every machine.

## Classic WinUSB + Arduino IDE 1.8.19 (Windows)

**Status:** see [ACCEPTANCE-WIN11-USBASP-001](acceptance/ACCEPTANCE-WIN11-USBASP-001.md) and [ARDUINO.md](ARDUINO.md). Prefer avrdude 7+/8.x; one bench Burn Bootloader PASS with stock 6.3 after string-index fix. Do **not** reinstall libusbK. Get Board Info is Serial-only.

## Software SCK ENABLEPROG (`-B 22` / slow ids)

**Status:** SW-SCK isolated — HW PASS, SW ENABLEPROG `0x01`. Root cause unknown; **capture next**, not more firmware guesses. [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md).

## TPI

**Status:** experimental — code present, **not** advertised.

FUNC 11–16 remain compiled. Board profiles set `USBASP_HAS_TPI=0`, so GETCAPABILITIES does **not** set `USBASP_CAP_TPI` until a real ATtiny4/5/10 acceptance pass. Flip to `1` only after that review.

## Windows 11 ARM64

Best-effort only. avrdude documents ARM64 USB limits.

## HIDUART on Windows MSVC avrdude

Composite device: prefer MinGW/libusb avrdude, or use **classic** for Arduino / WinUSB-only hosts.

## HIDUART USART on cheap clones (hardware)

**Status:** product limit of the common PCB, not a firmware bug.

Typical USBasp clone breakouts are **ISP 6-pin only** (MOSI/MISO/SCK/RST/VCC/GND). MCU USART **PD0/PD1** (TQFP 30–31) are **not** on that header. So:

- HIDUART is a **USB↔USART bridge on the stick MCU pins**, not “serial over the ISP ribbon”.
- Wiring a target’s `printf` UART to the stick needs flying leads to the TQFP (or a custom board). Loopback tests in this repo do the same (PD0–PD1 on the package).
- Firmware may still use HIDUART for iSerial, Linux/macOS ISP, and optional stick-side diagnostics; it does **not** turn a stock clone into a plug-and-play target console.

Release default remains **classic**. SW SCK (`-B 22` / ~32 kHz ENABLEPROG) stays the open functional gate: [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md).

## USB execution model

Documented invariant (INT0 vs `usbPoll`): [USB_EXECUTION.md](USB_EXECUTION.md).
