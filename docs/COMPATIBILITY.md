# USBasp NG compatibility contract

L0–L2 must not change without an explicit compatibility review.

## L0 USB identity

| Field | Classic NG |
|-------|------------|
| VID | `0x16c0` |
| PID | `0x05dc` |
| Device class | `0xFF` (vendor), as Fischl 2011 |
| Configuration | 1 |
| Interfaces | 1 vendor interface, **no** extra HID/WCID interfaces |
| Endpoints | Control EP0 only |
| Serial string | none |
| Max power | 50 mA (2011) |

Classic must keep the original USBasp **device/interface topology** enough that existing host stacks (avrdude + libusb / libusb-win32 / WinUSB-on-single-interface) still bind the same way.

`usbasp-hiduart` is a **separate product**: composite vendor + HID. It keeps L1/L2 ISP/TPI behaviour but is **not** an L0 topology match. Windows MSVC avrdude (libwinusb) may not open it; use a MinGW/libusb build.

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
- SETISPSCK meaning (id 0 = AUTO; AUTO in firmware is 1.5 MHz then auto-slow)
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

## L3 Host compatibility

Classic target: **current avrdude** `-c usbasp` with the stock programmer definition (no custom avrdude.conf).

Acceptance (hardware):

```text
avrdude -c usbasp -p atmega8
avrdude -c usbasp -p atmega88
avrdude -c usbasp -p atmega328p
```

plus read/write/verify of flash.

## Capability bytes (GETCAPABILITIES)

avrdude: `caps = b0 | b1<<8 | b2<<16 | b3<<24`

Classic:

- byte 0: `USBASP_CAP_TPI` (0x01)
- byte 1: 0
- byte 2: 0
- byte 3: `USBASP_CAP_3MHZ >> 24` (0x01) if the board can do 3 MHz SCK

No dioannidis clock-id bits in byte 1. No HID flags in the avrdude bitmap.

## What classic must not grow

BOS, MS OS 2.0, WCID, HID, serial EEPROM, composite descriptors — those belong only in `src_hid/` / hiduart boards.
