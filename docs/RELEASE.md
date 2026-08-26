# Release packaging

USBasp NG ships **two kinds of artifacts**. Never put build trees or `.git` in a source zip.

## Source ZIP

Built only with:

```bash
./scripts/pack-release.sh 0.2.1                 # source only
./scripts/pack-release.sh 0.2.1 --hex           # source + hex assets
./scripts/pack-release.sh 0.2.1 --hex --diag    # + portable Linux diag client
```

`git archive` of `HEAD` → `dist/usbasp-ng-src-vVERSION.zip`.

**Include:** tracked source, tests, `reference/`, `docs/`, `scripts/`, `arduino/`, `.github/`, …

**Never include:**

| Forbidden | Why |
|-----------|-----|
| `.git/` | VCS, not distribution |
| `firmware/build/` | local CMake/Ninja trees, absolute paths |
| `__pycache__/`, `*.pyc` | bytecode |
| `*.o`, `*.obj`, `*.elf`, `*.map` | intermediates |
| `*.hex`, `*.eep` | ship as separate assets |

A dirty “zip the working folder” RC is not a release. GitHub’s automatic “Source code (zip)” on a tag is fine (also an archive of the tree); prefer the named `usbasp-ng-src-v*.zip` asset for humans.

## Firmware HEX (separate assets)

| Asset | Board profile |
|-------|----------------|
| `usbasp-ng-classic-atmega8.hex` | `usbasp-atmega8-clone` |
| `usbasp-ng-classic-atmega88.hex` | `usbasp-atmega88` |
| `usbasp-ng-hiduart-atmega8.hex` (+ `.eep`) | `usbasp-hiduart-atmega8` |
| `usbasp-ng-hiduart-atmega88.hex` (+ `.eep`) | `usbasp-hiduart-atmega88` |

Default Windows/Arduino image: **classic ATmega8**. HIDUART EEPROM in release builds uses serial `0000` (override locally with `SERIAL=`).

## Host client (`diagplane.bin`)

| Asset | Platform | Notes |
|-------|----------|--------|
| `diagplane.bin` | Linux x86-64 | Renamed release binary of `tools/usbasp-ng-diag` |

Built by `./scripts/build-diagplane.sh` (also `--diag` on `pack-release.sh`):

- **Preferred:** musl + `crt-static` → runs on any x86-64 Linux (no `libusb-1.0.so`, no host glibc pin). Needs `musl-gcc` / `x86_64-linux-musl-gcc` and `rustup target add x86_64-unknown-linux-musl`.
- **Fallback:** host glibc + vendored libusb (no system libusb; still needs a glibc ≥ the build host).

CI publishes the musl build as a workflow artifact / release asset. Usage:

```bash
chmod +x diagplane.bin
./diagplane.bin demo enableprog_fail_sw --faults
./diagplane.bin watch --serial YEL0
```

Contracts: [`DIAGNOSTICS_CLIENT.md`](DIAGNOSTICS_CLIENT.md).

## Hardware acceptance (next; not packaging)

USB identity / WinUSB / MS OS are frozen for classic. Validate on iron:

1. **Windows 11 clean VM** → plug classic → WinUSB → modern avrdude `-c usbasp` → signature / flash → Arduino Burn Bootloader  
2. **Linux** → `-c usbasp` and `-c usbasp-clone` both open and talk ISP  

Software SCK gate is **closed** for the lab path (USBasp2 → mega8-on-Nano-PCB): [SOFTWARE_SCK.md](SOFTWARE_SCK.md). That does not change the classic USB compatibility claim.
