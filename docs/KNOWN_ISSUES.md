# Known issues

Tracked for release notes. Compatibility contract remains [`COMPATIBILITY.md`](COMPATIBILITY.md).

## Existing libusbK / libusb0 bindings (Windows)

**Status:** host OS / driver-store behavior.

PCs that previously used Zadig/libusbK for `16c0:05dc` may keep that association after flashing classic NG. Uninstall the device in Device Manager (delete driver if offered), remove the OEM INF if needed, then replug. Clean first-plug WinUSB does not guarantee migration on every machine.

## Classic WinUSB + Arduino IDE 1.8.19 (Windows)

**Status:** host-tool limitation, not a firmware bug.

Classic NG binds **Microsoft WinUSB** (BOS / MS OS 2.0: config → function IF0 → WINUSB). AVRDUDESS and avrdude **7.x / 8.x MSVC** work.

Arduino IDE **1.8.19** still ships **avrdude 6.3-20190619**, which cannot open WinUSB:

```text
avrdude: Warning: cannot query manufacturer for device: Invalid argument
avrdude: Warning: cannot query product for device: Invalid argument
avrdude: error: could not find USB device with vid=0x16c0 pid=0x5dc
         vendor='www.fischl.de' product='USBasp'
```

USB strings are already Fischl (`www.fischl.de` / `USBasp`). Do **not** roll back to pre-WinUSB classic or reinstall libusbK for this.

**Workaround:** AVRDUDESS / standalone modern avrdude, or run [`arduino/replace-avrdude.ps1`](../arduino/replace-avrdude.ps1). See [`ARDUINO.md`](ARDUINO.md), [`WINDOWS.md`](WINDOWS.md).

## Software SCK ENABLEPROG (`-B 22` / slow ids)

**Status:** open; waiting on waveform (Nano sniffer / FX2).

`-B 8` / HW SPI PASS; software SCK ENABLEPROG often `0x01` on classic and HIDUART. See [`SOFTWARE_SCK.md`](SOFTWARE_SCK.md).

## TPI

Advertised in GETCAPABILITIES; not exercised on silicon in this tree yet (no tiny4/5/10 on the bench).

## Windows 11 ARM64

Best-effort only. avrdude documents ARM64 USB limits.

## HIDUART on Windows MSVC avrdude

Composite device: prefer MinGW/libusb avrdude, or use **classic** for Arduino / WinUSB-only hosts.

## USB execution model

Documented invariant (INT0 vs `usbPoll`): [USB_EXECUTION.md](USB_EXECUTION.md).
