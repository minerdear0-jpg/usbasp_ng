# USBasp NG

Firmware for the cheap, widespread USBasp AVR programmer.

![Common USBasp clone — USB stick, 10-pin ribbon, optional adapter](usbasp.webp)

- **Protocol:** Fischl USBasp 2011 (what avrdude / AVRDUDESS speak)
- **ISP internals:** dioannidis / nerdralph fixes, without their composite USB identity on the **classic** image
- **Products:** **classic** (Windows / Arduino on mega8), **HIDUART** Diagnostics Plane, and **[USBasp2](docs/USBASP2.md) beta** — first USBasp fork with an integrated lab instrument (L1 + Diagnostics Plane; [RELEASE-USBASP2-BETA.md](docs/RELEASE-USBASP2-BETA.md))

---

## USBasp2 (ATmega328P programmer)

**USBasp2 beta** = same clone PCB, MCU upgraded to **ATmega328P** (reflow, 12 MHz crystal), with HIDUART + Diagnostics Plane for lab work. Bench name for the yellow stick after the swap.

| | mega8 HIDUART | **USBasp2** |
|---|---|---|
| MCU | ATmega8 (~8 KiB, ~70 B free with diag) | ATmega328P (32 KiB / 2 KiB) |
| Board | `usbasp-hiduart-atmega8` | `usbasp-hiduart-atmega328p` |
| Role | Legacy lab image | **Preferred** Diagnostics Plane programmer |

```bash
SERIAL=YEL0 make -C firmware BOARD=usbasp-hiduart-atmega328p flash   # via another USBasp, J2 closed
# then USBasp2 in USB, target on ribbon, J2 open:
./diagplane.bin watch --serial YEL0
avrdude -c usbasp -p m8 -U signature:r:-:h
```

Details and smoke notes: [`docs/USBASP2.md`](docs/USBASP2.md).

---

## Demo: program a target and watch the real ISP log

Lab case: **USBasp2** (or mega8 HIDUART) in USB, target on the ISP ribbon (328P / mega8 / …), `avrdude` in one terminal and **`diagplane` watch** in the other. Firmware emits SESSION / SCK / RESET / ENABLEPROG / MEMOP on HID EP2.

<video
  src="https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane.mp4"
  poster="https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane-poster.webp"
  controls
  muted
  playsinline
  width="720">
  <a href="https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane.mp4">Watch demo (MP4)</a>
</video>

[MP4](https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane.mp4)
· [WebM](https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane.webm)
· copies in [`docs/media/`](docs/media/)

### Do this on Linux

1. Flash **HIDUART** onto the stick (**USBasp2** preferred: `usbasp-hiduart-atmega328p`; mega8: release `usbasp-ng-hiduart-atmega8.hex` + `.eep`). Classic has no diagnostics endpoint.
2. **udev** (once):

   ```bash
   sudo cp host/udev/70-usbasp.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```

3. Stick in USB, target on the ISP header (power + GND). Self-program jumper (**J2**) **open**.
4. **Terminal A** — live TUI ([`diagplane.bin`](https://github.com/minerdear0-jpg/usbasp_ng/releases) or `tools/usbasp-ng-diag`):

   ```bash
   chmod +x diagplane.bin
   ./diagplane.bin watch --serial YEL0
   ```

5. **Terminal B** — program the target:

   ```bash
   avrdude -c usbasp -p m8 -U signature:r:-:h          # mega8 target
   avrdude -c usbasp -p m328p -U flash:w:firmware.hex:i  # or 328P target
   ```

Events appear in the TUI only while avrdude talks ISP. Client: [`docs/DIAGNOSTICS_CLIENT.md`](docs/DIAGNOSTICS_CLIENT.md). Design: [`docs/DIAGNOSTICS.md`](docs/DIAGNOSTICS.md).

> **Windows:** use **classic** to program chips. HIDUART may show drivers OK in Device Manager but ISP with AVRDUDESS / typical avrdude is **not** supported there. See below.

---

## Classic vs HIDUART

Same ISP wire protocol (FUNC 1–16 / 127). Different USB shape.

| | **Classic** (`usbasp`) | **HIDUART** (`usbasp-hiduart`) |
|---|---|---|
| USB | Vendor `0xFF`, EP0 only → WinUSB | Composite: WinUSB IF0 + HID (**EP2 = Diagnostics Plane**) |
| Windows ISP | **Works** — `usbasp` and `usbasp-clone` | Drivers may bind; **programming does not work properly** |
| Linux ISP | Works | Works + live telemetry |
| MCU | ATmega8 (default release) | mega8 or **USBasp2** (328P) |
| Role | Everyday / Arduino image | Instrumented lab stick |

```
firmware
   ├─ usbasp                 classic — WinUSB programmer
   └─ usbasp-hiduart         Diagnostics Plane (mega8 or atmega328p / USBasp2)
```

Windows / Arduino: [`docs/WINDOWS.md`](docs/WINDOWS.md), [`docs/ARDUINO.md`](docs/ARDUINO.md), [`ACCEPTANCE-WIN11-USBASP-001`](docs/acceptance/ACCEPTANCE-WIN11-USBASP-001.md).  
Software SCK: **closed** — [`ACCEPTANCE-SCK-SWEEP-001`](docs/acceptance/ACCEPTANCE-SCK-SWEEP-001.md) (USBasp2 → mega8-on-Nano-PCB, `-B 8`/`-B 22` PASS).  
Contract: [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md). Limits: [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md).

Classic must not grow HID, interrupt endpoints, EEPROM serial, or diagnostics.

---

## Build

Needs `avr-gcc`, `avr-libc`, CMake, and Ninja or `make`.

```bash
./scripts/build.sh                              # classic ATmega8 clone
./scripts/build.sh hiduart                      # HIDUART mega8
SERIAL=YEL0 ./scripts/build.sh usbasp-hiduart-atmega328p   # USBasp2
./scripts/build.sh --help
```

Hex: `firmware/build/<board>/usbasp.hex` or `usbasp-hiduart.hex`. Boards: [`firmware/boards/`](firmware/boards/).

```bash
cd firmware
make BOARD=usbasp-atmega8-clone
make BOARD=usbasp-hiduart-atmega8
make BOARD=usbasp-hiduart-atmega328p SERIAL=YEL0
make all-boards
make test
```

---

## Flash the programmer itself

Use another ISP on J2 / RESET (self-program jumper closed on the stick being written):

```bash
avrdude -c <isp> -p atmega8 -U flash:w:usbasp.hex:i          # mega8 stick
avrdude -c <isp> -p m328p -U flash:w:usbasp-hiduart.hex:i    # USBasp2
```

Do not change fuses unless you mean to. Fischl 2011 mega8: `hfuse=0xc9 lfuse=0xef`; many clones already have `hfuse=0xd9 lfuse=0xef`. USBasp2 crystal parts often already have `lfuse=0xff`. `make fuses` needs `CONFIRM_FUSES=1`.

HIDUART serial is 4 chars in EEPROM (`SERIAL=YEL0`). Classic has no iSerial.

From `firmware/`: `make flash` (HIDUART also writes EEPROM — chip-erase wipes EEPROM without EESAVE).

---

## Releases

| Asset | What |
|-------|------|
| `usbasp-ng-classic-*.hex` | Windows / Arduino daily driver (mega8) |
| `usbasp-ng-hiduart-*.hex` (+ `.eep`) | Linux Diagnostics Plane (mega8) |
| USBasp2 hex | build `usbasp-hiduart-atmega328p` (see [`USBASP2.md`](docs/USBASP2.md)) |
| `diagplane.bin` | Portable Linux x86-64 host TUI / monitor |
| source zip | `./scripts/pack-release.sh VERSION --hex --diag` |

→ [GitHub Releases](https://github.com/minerdear0-jpg/usbasp_ng/releases) · packaging [`docs/RELEASE.md`](docs/RELEASE.md)

---

## Repo layout

| Path | Role |
|------|------|
| [`firmware/`](firmware/) | Only build input |
| [`tools/usbasp-ng-diag/`](tools/usbasp-ng-diag/) | Production diag client (Rust) |
| [`host/`](host/) | Lab scripts, udev, goldens |
| [`docs/`](docs/) | Contracts and acceptance |
| [`docs/USBASP2.md`](docs/USBASP2.md) | ATmega328P programmer (USBasp2) |
| [`docs/DIAGNOSTICS_PROBE.md`](docs/DIAGNOSTICS_PROBE.md) | Development-probe philosophy (328P) |
| [`reference/`](reference/) | Fischl 2011 + dioannidis snapshots |

Inspect: [`host/usb-inspect-usbasp.sh`](host/usb-inspect-usbasp.sh), [`host/usbaspctl.py`](host/usbaspctl.py). Smoke: [`hw-smoke-atmega8.txt`](firmware/tests/compatibility/avrdude/hw-smoke-atmega8.txt), [`hw-smoke-atmega328p.txt`](firmware/tests/compatibility/avrdude/hw-smoke-atmega328p.txt).

---

## License

GPLv2, same as USBasp / V-USB. V-USB: `firmware/third_party/vusb/`.
