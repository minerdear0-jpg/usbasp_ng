# Firmware

CMake + board profile. See the repository [README](../README.md).

```text
cmake -S . -B build/usbasp-atmega8-clone -G Ninja -DBOARD=usbasp-atmega8-clone
cmake --build build/usbasp-atmega8-clone
```

`src/` is classic USBasp only. `src_hid/` is the HIDUART product and is not compiled into `usbasp`.

Still open on silicon (does not block firmware work):

- TPI (FUNC 11–16) — ATtiny10, avrdude `-p t10` — [`hw-smoke-tpi.txt`](tests/compatibility/avrdude/hw-smoke-tpi.txt)
- ATmega328P as an ISP target — [`hw-smoke-atmega328p.txt`](tests/compatibility/avrdude/hw-smoke-atmega328p.txt)
