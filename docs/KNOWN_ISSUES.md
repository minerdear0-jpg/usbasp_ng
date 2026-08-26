# Known issues

Tracked for release notes. Compatibility contract remains [`COMPATIBILITY.md`](COMPATIBILITY.md).

## Existing libusbK / libusb0 bindings (Windows)

**Status:** host OS / driver-store behavior.

PCs that previously used Zadig/libusbK for `16c0:05dc` may keep that association after flashing classic NG. Uninstall the device in Device Manager (delete driver if offered), remove the OEM INF if needed, then replug. Clean first-plug WinUSB does not guarantee migration on every machine.

## Classic WinUSB + Arduino IDE 1.8.19 (Windows)

**Status:** see [ACCEPTANCE-WIN11-USBASP-001](acceptance/ACCEPTANCE-WIN11-USBASP-001.md) and [ARDUINO.md](ARDUINO.md). Prefer avrdude 7+/8.x; one bench Burn Bootloader PASS with stock 6.3 after string-index fix. Do **not** reinstall libusbK. Get Board Info is Serial-only.

## Software SCK ENABLEPROG (`-B 22` / slow ids)

**Status:** **CLOSED.** USBasp2 → ATmega8 on Nano PCB: `-B 8` and `-B 22` signature PASS (2026-08-26). Record: [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md), [SOFTWARE_SCK.md](SOFTWARE_SCK.md). Historical mega8↔mega8 FAIL remains documented there as isolation only.

## USBasp2 (ATmega328P programmer)

**Status:** lab default for Diagnostics Plane. Reflow onto clone PCB; board `usbasp-hiduart-atmega328p`. See [USBASP2.md](USBASP2.md).

## TPI

**Status:** experimental — code present, **not** advertised.

FUNC 11–16 remain compiled. Board profiles set `USBASP_HAS_TPI=0`, so GETCAPABILITIES does **not** set `USBASP_CAP_TPI` until a real ATtiny4/5/10 acceptance pass. Flip to `1` only after that review.

## Windows 11 ARM64

Best-effort only. avrdude documents ARM64 USB limits.

## HIDUART on Windows (AVRDUDESS / avrdude)

**Status:** drivers often bind (WinUSB IF0 + HID), but **ISP programming does not work properly** on typical Windows hosts (confirmed with AVRDUDESS). Use **classic** for Windows (`-c usbasp` and `-c usbasp-clone` both OK). HIDUART remains a Linux diagnostics stick.

## HIDUART USART on cheap clones (hardware)

**Status:** product limit of the common PCB, not a firmware bug.

Typical USBasp clone breakouts are **ISP 6-pin only** (MOSI/MISO/SCK/RST/VCC/GND). MCU USART **PD0/PD1** (TQFP 30–31) are **not** on that header. So target-console wiring needs flying leads or a custom board.

**HIDUART’s primary product goal is the Diagnostics Plane** (EP2 binary telemetry: SESSION / SCK / RESET / ENABLEPROG / fault snapshots) — independent of PD0/PD1 and of the USBasp wire protocol. See [DIAGNOSTICS.md](DIAGNOSTICS.md). USART bridge remains optional secondary hardware.

Release default remains **classic** for Windows. Lab Diagnostics Plane: **USBasp2**. SW SCK gate closed: [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md).

## USB execution model

Documented invariant (INT0 vs `usbPoll`): [USB_EXECUTION.md](USB_EXECUTION.md).
