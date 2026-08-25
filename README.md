# USBasp NG

Firmware for the cheap, widespread USBasp AVR programmer.

- **Protocol:** Fischl USBasp 2011 (what avrdude speaks)
- **ISP/TPI internals:** dioannidis / nerdralph fixes, without their composite USB identity on the default image
- **Build:** CMake + avr-gcc, one **board profile** per configure

## Two products

```
firmware
   ├─ usbasp          classic — L0 topology of 2011, avrdude `-c usbasp`
   └─ usbasp-hiduart  separate composite device (HID UART + WCID)
```

Classic sources never mention HID. HID lives in `firmware/src_hid/` only.

## Build

Needs `avr-gcc`, `avr-libc`, CMake, Ninja.

```text
cd firmware
make BOARD=usbasp-atmega8-clone
# hex: firmware/build/usbasp-atmega8-clone/usbasp.hex

make BOARD=usbasp-atmega88
make BOARD=usbasp-atmega8-usbisp
make BOARD=usbasp-hiduart-atmega8
make all-boards
make test
```

Or:

```text
cmake -S firmware -B build/clone -G Ninja -DBOARD=usbasp-atmega8-clone
cmake --build build/clone
```

Board files: [`firmware/boards/`](firmware/boards/). They set MCU, F_CPU, LED style, SCK jumper, 3 MHz, classic vs hiduart. Do not pile ad-hoc `-DTHIS=1` flags.

## Flash (another programmer, J2 / RESET)

ATmega8 example:

```text
avrdude -c <isp> -p atmega8 -U flash:w:usbasp.hex:i
```

Do not program fuses unless you mean to. Fischl 2011 documents `hfuse=0xc9 lfuse=0xef`; cheap clones on this bench already had `hfuse=0xd9 lfuse=0xef`. `make fuses` is blocked until `CONFIRM_FUSES=1`.

ATmega88 (if you actually need fuses): `hfuse=0xdd` `lfuse=0xff`.

## avrdude (this programmer)

**no-dot** — clone with NG classic, USB to the host (`-c usbasp`).  
**yellow-dot** — experiment target: same ATmega8 clone on the ISP header (J2 closed). Flash HIDUART and other trials only onto yellow-dot.

```text
avrdude -c usbasp -p atmega8
avrdude -c usbasp -p atmega8 -U signature:r:-:h
```

Checklist: [`firmware/tests/compatibility/avrdude/hw-smoke-atmega8.txt`](firmware/tests/compatibility/avrdude/hw-smoke-atmega8.txt), 328P: [`hw-smoke-atmega328p.txt`](firmware/tests/compatibility/avrdude/hw-smoke-atmega328p.txt).

```text
avrdude -c usbasp -p atmega328p -U flash:w:firmware.hex:i
```

From `firmware/`: `make flash` writes the hex onto a USBasp (J2 closed) using another USBasp. Linux udev: [`host/udev/`](host/udev/). Inspect: [`host/usb-inspect-usbasp.sh`](host/usb-inspect-usbasp.sh), [`host/usbasp-getcaps.py`](host/usbasp-getcaps.py). HIDUART loopback: [`host/usbasp-hiduart-loopback.py`](host/usbasp-hiduart-loopback.py) (TQFP pins 30–31).

HIDUART image: on Windows use a **MinGW/libusb** avrdude, not the MSVC/libwinusb build.

HIDUART USB serial is 4 characters in EEPROM (`make BOARD=usbasp-hiduart-atmega8 SERIAL=YEL0 eeprom`). Classic has no serial. Bench yellow-dot uses `YEL0`.

## Compatibility

See [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md). Short version:

- VID/PID `16c0:05dc`, FUNC 1–16 and 127
- GETCAPABILITIES = TPI + 3 MHz bit, **not** dioannidis clock bytes
- Default SCK 1.5 MHz with auto-slowdown (same SETISPSCK wire)

Contract tests (no hardware):

```text
cd firmware && make test
```

Hex for ATmega8 clone and HIDUART: [Releases](https://github.com/minerdear0-jpg/usbasp_ng/releases). `v0.1.0` classic, `v0.1.1` HIDUART.

Still waiting on silicon (not blocking the protocol): ATmega328P ISP target, ATtiny10 TPI (`-p t10`). Checklists: [`hw-smoke-atmega328p.txt`](firmware/tests/compatibility/avrdude/hw-smoke-atmega328p.txt), [`hw-smoke-tpi.txt`](firmware/tests/compatibility/avrdude/hw-smoke-tpi.txt).

## Trees

| Path | Role |
|------|------|
| [`firmware/`](firmware/) | Only build input |
| [`reference/`](reference/) | Immutable Fischl 2011 and dioannidis snapshots |
| [`docs/`](docs/) | Compatibility contract |

`usbasp.2011-05-28/` and `dioannidis/` in the working tree (if still present) are leftovers from import; prefer `reference/`.

## License

GPLv2, same as USBasp / V-USB. V-USB remains under its own license files in `firmware/third_party/vusb/`.
