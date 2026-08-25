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
```

Or:

```text
cmake -S firmware -B build/clone -G Ninja -DBOARD=usbasp-atmega8-clone
cmake --build build/clone
```

Board files: [`firmware/boards/`](firmware/boards/). They set MCU, F_CPU, LED style, SCK jumper, 3 MHz, classic vs hiduart. Do not pile ad-hoc `-DTHIS=1` flags.

## Flash (another programmer, J2 / RESET)

ATmega8 example (fuses from the 2011 tree):

```text
avrdude -c <isp> -p atmega8 -U flash:w:usbasp.hex:i
avrdude -c <isp> -p atmega8 -U hfuse:w:0xc9:m -U lfuse:w:0xef:m
```

ATmega88: `hfuse=0xdd` `lfuse=0xff`.

## avrdude (this programmer)

Smoke without ATmega328P: second ATmega8 on the ISP header (other USBasp clone, J2 closed):

```text
avrdude -c usbasp -p atmega8
avrdude -c usbasp -p atmega8 -U signature:r:-:h
```

Checklist: [`firmware/tests/compatibility/avrdude/hw-smoke-atmega8.txt`](firmware/tests/compatibility/avrdude/hw-smoke-atmega8.txt).

```text
avrdude -c usbasp -p atmega328p -U flash:w:firmware.hex:i
```

From `firmware/`: `make flash` writes the hex onto a USBasp (J2 closed) using another USBasp. Linux udev: [`host/udev/`](host/udev/).

HIDUART image: on Windows use a **MinGW/libusb** avrdude, not the MSVC/libwinusb build.

## Compatibility

See [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md). Short version:

- VID/PID `16c0:05dc`, FUNC 1–16 and 127
- GETCAPABILITIES = TPI + 3 MHz bit, **not** dioannidis clock bytes
- Default SCK 1.5 MHz with auto-slowdown (same SETISPSCK wire)

Contract tests (no hardware):

```text
cd firmware && make test
```

## Trees

| Path | Role |
|------|------|
| [`firmware/`](firmware/) | Only build input |
| [`reference/`](reference/) | Immutable Fischl 2011 and dioannidis snapshots |
| [`docs/`](docs/) | Compatibility contract |

`usbasp.2011-05-28/` and `dioannidis/` in the working tree (if still present) are leftovers from import; prefer `reference/`.

## License

GPLv2, same as USBasp / V-USB. V-USB remains under its own license files in `firmware/third_party/vusb/`.
