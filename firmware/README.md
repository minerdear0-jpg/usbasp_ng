# Firmware

CMake + board profile. See the repository [README](../README.md).

```text
cmake -S . -B build/usbasp-atmega8-clone -G Ninja -DBOARD=usbasp-atmega8-clone
cmake --build build/usbasp-atmega8-clone
```

`src/` is classic USBasp only. `src_hid/` is the HIDUART product and is not compiled into `usbasp`.

Still open on silicon:

- TPI (FUNC 11–16) — ATtiny10 (SOT-23-6 + adapter), avrdude `-p t10`
- ATmega328P as an ISP target (`-p atmega328p`)
