# Release packaging

USBasp NG ships **two kinds of artifacts**. Never put build trees or `.git` in a source zip.

## Source ZIP

Built only with:

```bash
./scripts/pack-release.sh 0.2.1          # source only
./scripts/pack-release.sh 0.2.1 --hex    # source + hex assets
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

## Hardware acceptance (next; not packaging)

USB identity / WinUSB / MS OS are frozen for classic. Validate on iron:

1. **Windows 11 clean VM** → plug classic → WinUSB → modern avrdude `-c usbasp` → signature / flash → Arduino Burn Bootloader  
2. **Linux** → `-c usbasp` and `-c usbasp-clone` both open and talk ISP  

Software SCK still needs waveform capture ([SOFTWARE_SCK.md](SOFTWARE_SCK.md)); that does not block the classic USB compatibility claim.
