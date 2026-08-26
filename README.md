# USBasp NG

Firmware for the common USBasp AVR ISP programmer. Fischl wire protocol; maintained builds for Windows daily use and a Linux lab instrument.

![USBasp clone — USB stick, ribbon cable](usbasp.webp)

| Product | MCU | Role |
|---------|-----|------|
| **Classic** | ATmega8 | Windows / Arduino ISP (`-c usbasp` / `usbasp-clone`) |
| **[USBasp2](docs/USBASP2.md)** beta | ATmega328P | Same L1 ISP + **Diagnostics Plane** (`diagplane`) |

Protocol: Fischl USBasp 2011. ISP core: dioannidis / nerdralph fixes. Classic keeps a simple WinUSB identity (no composite HID).

**Current release:** [`usbasp2-beta.1`](https://github.com/minerdear0-jpg/usbasp_ng/releases/tag/usbasp2-beta.1) — [release notes](docs/RELEASE-USBASP2-BETA.md).

---

## Two images

| | Classic | USBasp2 (lab) |
|---|---|---|
| Board | `usbasp-atmega8-clone` | `usbasp-hiduart-atmega328p` |
| USB | Vendor class, EP0 only → WinUSB | Composite WinUSB + HID **EP2** |
| Windows ISP | Supported | Not supported (use classic) |
| Linux ISP | Supported | Supported + EP2 telemetry |
| Host tool | avrdude / AVRDUDESS | avrdude + [`diagplane`](tools/usbasp-ng-diag/) |

```
USBasp (Fischl)
  └─ USBasp NG classic     — daily programmer
  └─ USBasp2 (328P)        — programmer + Diagnostics Plane
```

mega8/88 HIDUART hex from beta.1 remains buildable and is **frozen** (no further lab-grain work). Full Diagplane = **328P only**.

---

## Diagnostics Plane

While `avrdude` programs a target, firmware emits a semantic ISP timeline on HID EP2: SESSION, SCK, RESET, ENABLEPROG, MEMOP, TRACE, … Host `diagplane` watches, records, correlates.

<video
  src="https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane.mp4"
  poster="https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane-poster.webp"
  controls
  muted
  playsinline
  width="720">
  <a href="https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane.mp4">Demo (MP4)</a>
</video>

[MP4](https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane.mp4)
· [WebM](https://github.com/minerdear0-jpg/usbasp_ng/releases/download/demo-assets/demo-diagplane.webm)
· [`docs/media/`](docs/media/)
· beta.1 TUI + cage flash/eeprom/fuses reads: [`demo-diagplane-beta1.cast`](docs/media/demo-diagplane-beta1.cast) (`asciinema play`)

### Linux (USBasp2)

1. Flash `usbasp-ng-hiduart-atmega328p.hex` + `.eep` (or `SERIAL=YEL0 make -C firmware BOARD=usbasp-hiduart-atmega328p flash` via another USBasp, J2 closed).
2. Install udev once:

   ```bash
   sudo cp host/udev/70-usbasp.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```

3. Stick in USB, target on the ISP header, **J2 open**.
4. Terminal A:

   ```bash
   chmod +x diagplane.bin
   ./diagplane.bin watch --serial YEL0
   ```

5. Terminal B:

   ```bash
   avrdude -c usbasp -p m8 -U signature:r:-:h
   ```

Wire contract: [`docs/DIAGNOSTICS.md`](docs/DIAGNOSTICS.md). Client: [`docs/DIAGNOSTICS_CLIENT.md`](docs/DIAGNOSTICS_CLIENT.md). Probe model: [`docs/DIAGNOSTICS_PROBE.md`](docs/DIAGNOSTICS_PROBE.md).

Windows ISP: flash **classic** only. See [`docs/WINDOWS.md`](docs/WINDOWS.md).

---

## Build

Requires `avr-gcc`, `avr-libc`, CMake, Ninja or Make.

```bash
./scripts/build.sh                                    # classic ATmega8
SERIAL=YEL0 ./scripts/build.sh usbasp-hiduart-atmega328p   # USBasp2
./scripts/build.sh --help
```

```bash
cd firmware
make BOARD=usbasp-atmega8-clone
make BOARD=usbasp-hiduart-atmega328p SERIAL=YEL0
make test
```

Hex output: `firmware/build/<board>/`. Board list: [`firmware/boards/`](firmware/boards/).

### Flash the stick

Self-program jumper **closed** on the device being written:

```bash
avrdude -c usbasp -p atmega8 -U flash:w:usbasp.hex:i
avrdude -c usbasp -p m328p   -U flash:w:usbasp-hiduart.hex:i \
  -U eeprom:w:usbasp-hiduart.eep:i
```

Do not change fuses unless required. HIDUART iSerial is four EEPROM chars (`SERIAL=YEL0`). Classic has no iSerial.

---

## Releases

| Asset | Use |
|-------|-----|
| `usbasp-ng-classic-atmega8.hex` | Windows / Arduino daily |
| `usbasp-ng-hiduart-atmega328p.hex` + `.eep` | USBasp2 lab |
| `diagplane.bin` | Linux host client |
| source zip | `./scripts/pack-release.sh VERSION --hex --diag` |

→ [GitHub Releases](https://github.com/minerdear0-jpg/usbasp_ng/releases) · [`docs/RELEASE.md`](docs/RELEASE.md)

Packaging after beta.1 defaults to **classic + 328P**. Frozen mega8/88 HIDUART: `--legacy-hiduart`.

---

## Documentation

| Doc | Content |
|-----|---------|
| [`docs/USBASP2.md`](docs/USBASP2.md) | USBasp2 hardware and smoke |
| [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) | Host / protocol contract |
| [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md) | Limits |
| [`docs/WINDOWS.md`](docs/WINDOWS.md) / [`ARDUINO.md`](docs/ARDUINO.md) | Classic on Windows |
| [`docs/acceptance/`](docs/acceptance/) | Hardware gates |

| Path | Role |
|------|------|
| [`firmware/`](firmware/) | Firmware build |
| [`tools/usbasp-ng-diag/`](tools/usbasp-ng-diag/) | `diagplane` (Rust) |
| [`host/`](host/) | udev, `usbaspctl`, goldens |
| [`reference/`](reference/) | Fischl / dioannidis snapshots |

---

## License

GPLv2 (USBasp / V-USB). V-USB: `firmware/third_party/vusb/`.
