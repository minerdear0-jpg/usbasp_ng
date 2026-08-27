# Канарейка — diag oracle (Channel 2)

**Канарейка** is the ATmega8 soldered on the Nano PCB. This firmware is one
costume it can wear in the cage (YEL0 ISP + CH340 UART).

After USBasp2 releases RESET, Канарейка says what actually landed. It cannot
see ISP bytes (those happen under reset).

```text
Клетка
├── USBasp2 YEL0  → ISP → Канарейка     Channel 1: diagplane EP2
└── CH340         → UART Канарейка      Channel 2: this firmware
```

Clock 16 MHz crystal, UART 115200 8N1 (`/dev/ttyUSB0`). LEDs: D13 + D2/D3/D4.

## What it reports

`@TTTTTTTT EVENT,k=v` — 8-digit ms since target start (wrap 1e8).

| Event | Why it exists |
|-------|----------------|
| `READY` | ident, `canary_off`, `sram_free` — dual-truth vs ihex |
| `RESET_CAUSE` | `MCUCSR` bits + `eeprom=chip_erased\|live` |
| `ISP_PINS,when=reset` | `PINB` **before** LED init (D13 is SCK) |
| `CANARY` ×8 | last 512 B of flash (`0x1E00`, 8×64 B pages) |
| `FLASH_CRC` | CRC-CCITT of all **8192** flash bytes (erased=0xFF) |
| `SELFTEST` | SRAM scratch + EEPROM 16 B + canary |

ISP chip-erase without EESAVE wipes EEPROM → `eeprom=chip_erased`, `boot=1`.
A RESET **button** after that → `eeprom=live`, `boot` increments. Do not
enable EESAVE just to keep a counter.

**Costume 2 fuses:** this cage often has Optiboot (`hfuse=0xC4`, BOOTRST).
ISP `make flash` overwrites `0x1E00` canary over the boot section — with
BOOTRST still set the MCU jumps into the canary blob and UART stays silent.
For oracle: `hfuse=0xC5` (BOOTRST off). Costume 1 (Optiboot + nano-loop):
burn bootloader again (`hfuse=0xC4`).

Harness drops **DTR** on the CH340 so opening `/dev/ttyUSB0` does not pulse
the Nano RESET pin and race the ISP session.

`wdt-test` is an explicit UART command (hangs, next boot `WATCHDOG_TEST`).
Not part of `selftest`.

## Simulated faults (no ribbon sabotage)

UART, persist in EEPROM (`EESAVE` on this cage). `inject=1` on the lied field.
`fault off` clears. Does **not** cause ENABLEPROG_FAIL.

| Command | Channel 2 lie |
|---------|----------------|
| `fault canary 7` | last canary page FAIL (0–7) |
| `fault crc` | `FLASH_CRC` xor `0xFFFF` |
| `fault pins` | MOSI+SCK stuck 1 on `ISP_PINS` |
| `fault reset-wdt` | `extrf=0 wdrf=1` |
| `fault reset-por` | `porf=1 extrf=0` |
| `fault` / `fault off` | show / clear |

Real MEMOP truncate (costume 2, ISP writes bad hex — still ENABLEPROG PASS):

```bash
python3 harness.py mangle last-page mega8-diag-oracle.hex > /tmp/trunc.hex
```

## Build / flash

```bash
# terminal A — programmer semantic log
./dist/diagplane.bin watch --serial YEL0
# or: ./dist/diagplane.bin record YEL0 /tmp/yel0.bin

# terminal B
make -C bench/mega8-diag-oracle
make -C bench/mega8-diag-oracle test          # no hardware
python3 bench/mega8-diag-oracle/harness.py crc bench/mega8-diag-oracle/mega8-diag-oracle.hex

make -C bench/mega8-diag-oracle flash         # or: python3 harness.py run
python3 bench/mega8-diag-oracle/harness.py monitor
```

`harness.py run` arms (if live), runs avrdude, waits for `READY`…`SELFTEST`,
compares `FLASH_CRC` to the hex. Optional `--diag-jsonl` attaches Channel 1
lines into `last_report.json`.

```bash
./dist/diagplane.bin decode /tmp/yel0.bin --jsonl > /tmp/yel0.jsonl
python3 bench/mega8-diag-oracle/harness.py run --diag-jsonl /tmp/yel0.jsonl
```

## Dual-truth vs diagplane

| Programmer (EP2) | Target UART | Anomaly |
|------------------|-------------|---------|
| `ENABLEPROG` END\|OK | `READY` + `FLASH_CRC` match hex | truncated write / wrong page |
| `RESET_ASSERT` | next boot `extrf=1` | missing EXTRF → not that reset |
| `MEMOP` FLASH END\|OK | all `CANARY` PASS | last pages dropped (avrdude often omits LAST) |
| `SESSION_END` | `APP_START` a few ms later | target never came up |
| `ispDisconnect` Hi-Z | `ISP_PINS,when=reset` | SCK/MOSI still driven |

Default trigger `ENABLEPROG_FAIL` is **not** produced by this firmware. Unplug
MISO / starve SCK / power — then watch POST→FROZEN on YEL0.

UART console: `help`, `selftest`, `flash-crc`, `canary`, `arm`, `wdt-test`, `time`, …
Prefix `>` optional. `time` / READY include `tcnt1` (Timer1). `harness.py monitor`
prefixes `host_ns` so correlate can print dt ± doubt.
