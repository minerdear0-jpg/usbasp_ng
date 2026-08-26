# USBasp NG

Firmware for the cheap, widespread USBasp AVR programmer.

![Common USBasp clone — USB stick, 10-pin ribbon, optional adapter](usbasp.webp)

- **Protocol:** Fischl USBasp 2011 (what avrdude / AVRDUDESS speak)
- **ISP internals:** dioannidis / nerdralph fixes, without their composite USB identity on the **classic** image
- **Two products:** everyday **classic** (Windows + Arduino) and **HIDUART** (Linux Diagnostics Plane)

---

## Demo: flash a 328P and watch the real ISP log

Typical lab case: HIDUART stick in USB, ATmega328P on the ISP ribbon, `avrdude` in one terminal and **`diagplane` watch** in the other. Firmware emits SESSION / SCK / RESET / ENABLEPROG / MEMOP on HID EP2 while you program the target.

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

1. **Flash HIDUART** onto the stick (not classic — classic has no diagnostics endpoint). Release assets: `usbasp-ng-hiduart-atmega8.hex` + `.eep`.
2. **udev** (once):

   ```bash
   sudo cp host/udev/70-usbasp.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```

3. Plug the stick into USB, wire the **328P** to the ISP header (power + GND). Leave the self-program jumper (J2) **open**.
4. **Terminal A** — live TUI ([`diagplane.bin`](https://github.com/minerdear0-jpg/usbasp_ng/releases) from Releases, or build `tools/usbasp-ng-diag`):

   ```bash
   chmod +x diagplane.bin
   ./diagplane.bin watch                 # first composite stick
   # ./diagplane.bin watch --serial YEL0 # if your EEPROM serial is YEL0
   ```

5. **Terminal B** — program the target:

   ```bash
   avrdude -c usbasp -p m328p -U signature:r:-:h
   avrdude -c usbasp -p m328p -U flash:w:firmware.hex:i
   ```

Events appear in the TUI only while avrdude talks ISP. Full client notes: [`docs/DIAGNOSTICS_CLIENT.md`](docs/DIAGNOSTICS_CLIENT.md). Design: [`docs/DIAGNOSTICS.md`](docs/DIAGNOSTICS.md).

> **Windows:** use **classic** to program chips. HIDUART may show drivers OK in Device Manager but ISP with AVRDUDESS / typical avrdude is **not** supported there. See below.

---

## Classic vs HIDUART

Same ISP wire protocol (FUNC 1–16 / 127). Different USB shape.

| | **Classic** (`usbasp`) | **HIDUART** (`usbasp-hiduart`) |
|---|---|---|
| USB | Vendor `0xFF`, EP0 only → WinUSB | Composite: WinUSB IF0 + HID (**EP2 = Diagnostics Plane**) |
| Windows ISP | **Works** — `usbasp` and `usbasp-clone` | Drivers may bind; **programming does not work properly** |
| Linux ISP | Works | Works + live telemetry |
| Role | Default everyday / Arduino image | Instrumented lab stick |
| Size (ATmega8) | ~5610 B | ~8040 B (`USBASP_HAS_DIAG=1`) |

```
firmware
   ├─ usbasp          classic — WinUSB programmer
   └─ usbasp-hiduart  research + Diagnostics Plane
```

Windows / Arduino acceptance: [`docs/WINDOWS.md`](docs/WINDOWS.md), [`docs/ARDUINO.md`](docs/ARDUINO.md), [`ACCEPTANCE-WIN11-USBASP-001`](docs/acceptance/ACCEPTANCE-WIN11-USBASP-001.md).  
Open gate (software SCK on some targets): [`ACCEPTANCE-SCK-SWEEP-001`](docs/acceptance/ACCEPTANCE-SCK-SWEEP-001.md).  
Contract: [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md). Known limits: [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md).

Classic must not grow HID, interrupt endpoints, EEPROM serial, or diagnostics.

---

## Build

Needs `avr-gcc`, `avr-libc`, CMake, and Ninja or `make`.

```bash
./scripts/build.sh                 # classic ATmega8 clone
./scripts/build.sh hiduart         # HIDUART composite
SERIAL=YEL0 ./scripts/build.sh usbhid
./scripts/build.sh --help
```

Hex: `firmware/build/<board>/usbasp.hex` or `usbasp-hiduart.hex`. Boards: [`firmware/boards/`](firmware/boards/).

```bash
cd firmware
make BOARD=usbasp-atmega8-clone
make BOARD=usbasp-hiduart-atmega8
make all-boards
make test
```

---

## Flash the programmer itself

Use another ISP on J2 / RESET (self-program jumper closed on the stick being written):

```bash
avrdude -c <isp> -p atmega8 -U flash:w:usbasp.hex:i
```

Do not change fuses unless you mean to. Fischl 2011: `hfuse=0xc9 lfuse=0xef`; many clones already have `hfuse=0xd9 lfuse=0xef`. `make fuses` needs `CONFIRM_FUSES=1`.

HIDUART serial is 4 chars in EEPROM (`SERIAL=YEL0` in the board recipe). Classic has no iSerial. Release HIDUART `.eep` often uses `0000`.

From `firmware/`: `make flash` (then EEPROM for HIDUART in the same recipe — chip-erase wipes EEPROM without EESAVE).

---

## Releases

| Asset | What |
|-------|------|
| `usbasp-ng-classic-*.hex` | Windows / Arduino daily driver |
| `usbasp-ng-hiduart-*.hex` (+ `.eep`) | Linux Diagnostics Plane stick |
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
| [`reference/`](reference/) | Fischl 2011 + dioannidis snapshots |

Inspect helpers: [`host/usb-inspect-usbasp.sh`](host/usb-inspect-usbasp.sh), [`host/usbaspctl.py`](host/usbaspctl.py). Smoke checklists: [`hw-smoke-atmega8.txt`](firmware/tests/compatibility/avrdude/hw-smoke-atmega8.txt), [`hw-smoke-atmega328p.txt`](firmware/tests/compatibility/avrdude/hw-smoke-atmega328p.txt).

---

## License

GPLv2, same as USBasp / V-USB. V-USB: `firmware/third_party/vusb/`.
