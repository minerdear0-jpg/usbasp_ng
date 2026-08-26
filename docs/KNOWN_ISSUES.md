# Known issues

Tracked for release notes. Compatibility contract remains [`COMPATIBILITY.md`](COMPATIBILITY.md).

## Classic WinUSB + Arduino IDE 1.8.19 (Windows)

**Status:** host-tool limitation, not a firmware bug.

Classic NG binds **Microsoft WinUSB** (BOS / MS OS 2.0). AVRDUDESS and avrdude **7.x / 8.x MSVC** work.

Arduino IDE **1.8.19** still ships **avrdude 6.3-20190619**, which cannot open WinUSB:

```text
avrdude: Warning: cannot query manufacturer for device: Invalid argument
avrdude: Warning: cannot query product for device: Invalid argument
avrdude: error: could not find USB device with vid=0x16c0 pid=0x5dc
         vendor='www.fischl.de' product='USBasp'
```

USB strings are already Fischl (`www.fischl.de` / `USBasp`). Do **not** roll back to pre-WinUSB classic or reinstall libusbK for this.

**Workaround:** AVRDUDESS / standalone modern avrdude, or replace the IDE’s `hardware/tools/avr` avrdude with an 8.x MSVC build. See [`ARDUINO.md`](ARDUINO.md), [`WINDOWS.md`](WINDOWS.md).

## Software SCK ENABLEPROG (`-B 22` / slow ids)

**Status:** open; waiting on waveform (Nano sniffer / FX2).

`-B 8` / HW SPI PASS; software SCK ENABLEPROG often `0x01` on classic and HIDUART. See [`SOFTWARE_SCK.md`](SOFTWARE_SCK.md).

## TPI

Advertised in GETCAPABILITIES; not exercised on silicon in this tree yet (no tiny4/5/10 on the bench).

## Windows 11 ARM64

Best-effort only. avrdude documents ARM64 USB limits.

## HIDUART on Windows MSVC avrdude

Composite device: prefer MinGW/libusb avrdude, or use **classic** for Arduino / WinUSB-only hosts.
