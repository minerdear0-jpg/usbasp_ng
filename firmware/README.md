# Firmware

CMake + board profile. See the repository [README](../README.md) for classic vs USBHID.

Lightweight build from the repo root:

```text
../scripts/build.sh
../scripts/build.sh hiduart
```

Or from this directory:

```text
cmake -S . -B build/usbasp-atmega8-clone -G Ninja -DBOARD=usbasp-atmega8-clone
cmake --build build/usbasp-atmega8-clone
```

`src/` is classic USBasp (one vendor interface + WinUSB metadata). `src_hid/` is the USBHID product and is not compiled into `usbasp`.

**USBasp2** (ATmega328P MCU on the stick): `BOARD=usbasp-hiduart-atmega328p` — [`docs/USBASP2.md`](../docs/USBASP2.md).

```text
SERIAL=YEL0 make BOARD=usbasp-hiduart-atmega328p hex
```

Still open on silicon (does not block firmware work):

- TPI (FUNC 11–16) — ATtiny10, avrdude `-p t10` — [`hw-smoke-tpi.txt`](tests/compatibility/avrdude/hw-smoke-tpi.txt). Opcode/SETUP contract: [`tests/core/test_tpi.py`](tests/core/test_tpi.py).
- ATmega328P as an ISP *target* (Nano/Uno on ribbon) — [`hw-smoke-atmega328p.txt`](tests/compatibility/avrdude/hw-smoke-atmega328p.txt)
