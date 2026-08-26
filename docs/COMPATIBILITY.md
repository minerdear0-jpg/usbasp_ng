# USBasp NG compatibility contract

L0–L2 must not change without an explicit compatibility review.

## L0 USB identity

| Field | Classic NG |
|-------|------------|
| VID | `0x16c0` |
| PID | `0x05dc` |
| Device class | `0xFF` (vendor), as Fischl 2011 |
| Interface class | `0` (V-USB default; vendor is on the device, not the interface) |
| bcdUSB | `2.01` (BOS; still **low speed** on the wire) |
| bcdDevice | `2.02` (WinUSB bind; not Fischl `2.00`, not HIDUART `2.01`) |
| Configuration | 1 |
| Interfaces | 1 vendor interface, **no** extra class interfaces |
| Endpoints | Control EP0 only (`bNumEndpoints = 0`) |
| Serial string | none (`iSerial = 0`) |
| Max power | 50 mA (2011) |
| Host driver metadata | BOS + MS OS 2.0 Compatible ID `WINUSB` (not USBasp FUNC) |

Classic must keep the original USBasp **device/interface topology** (one vendor interface, EP0 only). BOS / MS OS 2.0 is **driver-binding metadata** for Windows; L1 FUNC 1–16 / 127 is unchanged. See [WINDOWS.md](WINDOWS.md).

`usbasp-hiduart` is a **separate product**: composite vendor + HID. It keeps L1/L2 ISP/TPI behaviour and the stock avrdude VID/PID `16c0:05dc`. It is **not** an L0 topology match. Windows MSVC avrdude (libwinusb) may not open the composite; use a MinGW/libusb build or prefer classic for Arduino. WCID WINUSB + `DeviceInterfaceGUIDs` apply only to vendor IF0; HID IF1/IF2 bind by class. `bcdDevice` is 2.01 so Windows UsbFlags does not share classic 2.02.

## L1 USB wire protocol

Unchanged:

- `bmRequestType` vendor/device, IN `0xC0` / OUT `0x40`
- `bRequest` = FUNC **1–16** and **127**
- `wValue` / `wIndex` packing as avrdude `usbasp_transmit()`:  
  `wValue = send[0] | send[1]<<8`, `wIndex = send[2] | send[3]<<8`
- Response sizes: TRANSMIT 4, ENABLEPROG 1, SETISPSCK 1, GETCAPABILITIES 4, TPI_RAWREAD 1, others 0 unless a data stage
- Data stages: `usbFunctionRead` / `usbFunctionWrite` for flash/EEPROM/TPI blocks

See [firmware/tests/compatibility/avrdude/spec.yaml](../firmware/tests/compatibility/avrdude/spec.yaml).

## L2 Programmer semantics

Preserve:

- CONNECT then ENABLEPROG
- SETISPSCK meaning (id 0 = AUTO; AUTO starts at 1.5 MHz then coarse auto-slow: 375 kHz, 93.75 kHz, 16 kHz, 500 Hz)
- JP3/PC2 applies 8 kHz software SCK on the wire (CONNECT, SETISPSCK, and idle LED) without overwriting the stored host SETISPSCK id; ENABLEPROG must not ramp back to 1.5 MHz while the jumper is closed
- paged flash FIRST/LAST and 12-bit page size packing
- EEPROM byte write with wait
- SETLONGADDRESS → little-endian u32, then ignore 16-bit addresses in later commands
- TPI connect delay, raw/block
- DISCONNECT
- `usbFunctionRead`/`Write` returning `0xFF` when not in the matching state

Rule: **fix a bug only if it is not observable compatibility behaviour** for avrdude.

NG-internal improvements that stay on the same wire:

- `usbasp_read_le32()` instead of unaligned `*(unsigned long*)&data[2]`
- `prog_nbytes` countdown on reads (dioannidis)
- do not reset requested SCK on DISCONNECT
- AT89S51/52 programming-enable probe
- board layer for LED polarity and optional PC2 jumper
- LEDs: PC0 1 Hz on USB/ISP traffic; idle PC0 breathes only when 8 kHz software SCK is applied (JP3 or SETISPSCK); otherwise idle 1 Hz while configured (USB host). PC1 ISP ~10 Hz
- software SCK: cycle-count **minimum** half-period (INT0 may stretch, must not shorten); LED stays out of `ispTransmit_sw` and `ispTransmit_hw`
- SETISPSCK stores **requested** id (`prog_sck`); jumper / AUTO slowdown only change **effective** wire clock (`effective_sck`)
- SETISPSCK applies the selected clock immediately (jumper still wins on the wire)

## L2.5 Timing (software SCK)

USB execution: INT0 only clocks the bus; `usbPoll()` / `usbFunctionSetup()` run from main with I=1. ISP may be preempted by INT0.

- HW mode: hardware SPI semantics
- SW mode: `f_requested` is an upper bound; actual half-period >= requested half-period
- Interrupt latency may stretch SCK high/low; no ISR may shorten a phase
- PORTB RMW for MOSI/SCK/RST is `cli`/`SREG` vs V-USB `in`/`ori`/`out`

Waveform proof for ENABLEPROG at `-B 22` is still open: [SOFTWARE_SCK.md](SOFTWARE_SCK.md).

## L3 Host compatibility

Classic target: **current avrdude** `-c usbasp` (Fischl vendor/product strings) or `-c usbasp-clone` (VID/PID only). No custom avrdude.conf.

Acceptance (hardware):

```text
avrdude -c usbasp -p atmega8
avrdude -c usbasp -p atmega88
avrdude -c usbasp -p atmega328p
```

plus read/write/verify of flash.

Measured on ATmega8 clones (no-dot programmer, yellow-dot target / USB DUT):

- signature `1E 93 07`
- lfuse `0xef`, hfuse `0xd9` (not the 2011 documented `0xc9`)
- SETISPSCK `-B 8` → 93750 Hz, `-B 0.5` → 1.5 MHz; AUTO dump of no-dot OK
- software SCK `-B 22/50/250` (32/16/4 kHz): ENABLEPROG `0x01` on both NG classic and HIDUART (same pair). Not a composite-only IRQ issue. Report and wanted capture: [SOFTWARE_SCK.md](SOFTWARE_SCK.md).
- yellow as ISP target (no-dot programmer): EEPROM 512 B read; 16-byte write+verify+restore `0xFF`; `-B 0.25` → 3 MHz signature OK
- yellow NG as programmer, no-dot as target: EEPROM read 512 B (`0xFF`); SETISPSCK `-B 8` / `-B 0.5` / `-B 0.25` (3 MHz) signature OK. JP3 closed → 8 kHz software SCK, ENABLEPROG `0x01` (same SW-SCK bug).
- GETCAPABILITIES `01 00 00 01`
- classic L0 USB: one interface, EP0 only; BOS + MS OS 2.0 WinUSB; `bcdDevice` 2.02. HIDUART is USB 2.01 composite (not L0), same VID/PID, `bcdDevice` 2.01
- **Win11 x64 2026-08-26:** yellow classic, libusbK uninstalled → WinUSB immediately; AVRDUDESS latest `-c usbasp-clone` read flash+EEPROM of the other mega8. See [WINDOWS.md](WINDOWS.md).
- HIDUART yellow-dot: iSerial `YEL0`; ISP read of no-dot through composite (flash 4018 B, EEPROM 512 B `0xFF`, `-B 8`/`0.5`/`0.25`); UART loopback PD0–PD1

## Capability bytes (GETCAPABILITIES)

avrdude: `caps = b0 | b1<<8 | b2<<16 | b3<<24`

Classic:

- byte 0: `USBASP_CAP_TPI` (0x01)
- byte 1: 0
- byte 2: 0
- byte 3: `USBASP_CAP_3MHZ >> 24` (0x01) if the board can do 3 MHz SCK

No dioannidis clock-id bits in byte 1. No HID flags in the avrdude bitmap.

TPI (FUNC 11–16) is compiled and advertised (`USBASP_CAP_TPI`). It is **not** exercised on silicon in this repo yet (no tiny4/5/10 on the bench).

## What classic must not grow

HID interfaces, interrupt endpoints, serial EEPROM, composite configurations — those belong only in `src_hid/` / hiduart boards.

Classic **may** include BOS + MS OS 2.0 so Windows binds WinUSB without Zadig. That is not an L1/L2 protocol change.
