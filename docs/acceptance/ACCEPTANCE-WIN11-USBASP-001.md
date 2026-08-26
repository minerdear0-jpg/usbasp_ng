# ACCEPTANCE-WIN11-USBASP-001

**Status:** PASS (destructive ISP cycle)  
**Date:** 2026-08-26  
**Kind:** accidental bench acceptance (Arduino IDE Burn Bootloader)

## Strict conclusion

USBasp NG Classic successfully completed a full destructive ISP programming cycle on Windows 11 using WinUSB and avrdude 6.3, including chip erase, fuse programming, ~8 KiB flash programming, and verification.

## What this proves

| Layer | Result |
|-------|--------|
| USB identity (Fischl strings, `bcdDevice` 2.03) | Opened by avrdude 6.3 `-c usbasp` |
| WinUSB binding | Device usable as programmer (no Zadig in this session) |
| USBasp FUNC path | CONNECT / erase / fuse / flash / verify |
| Classic HW SPI ISP (default AUTO / fast `-B`) | PASS for this workload |

## What this does **not** prove

- Software SCK (`-B 22` / slow ids) — **closed** later the same day on USBasp2 ([SOFTWARE_SCK.md](../SOFTWARE_SCK.md), [ACCEPTANCE-SCK-SWEEP-001](ACCEPTANCE-SCK-SWEEP-001.md))
- Arduino **Get Board Info** (Serial-only; irrelevant)
- Universal “avrdude 6.3 works on every Windows PC” — one bench PASS after string-index fix
- HIDUART / composite MSVC avrdude
- TPI (capability bit off until silicon proof)

Do **not** widen the Windows/Arduino compatibility matrix from this run alone. Record it. SW SCK is a separate criterion — later closed on USBasp2.

## Environment

| Field | Value |
|-------|--------|
| OS | Windows 11 x64 |
| Host tool | Arduino IDE 1.8.19 → bundled **avrdude 6.3-20190619** |
| USB driver | Microsoft WinUSB (classic NG metadata) |
| VID:PID | `16c0:05dc` |
| Programmer | yellow-dot USBasp NG **classic** |
| `bcdDevice` | **2.03** |
| MS OS 2.0 | device-level set **0x9E** (WINUSB) |
| Firmware | post–string-index / device-level MS OS line (see git around 2026-08-26; yellow flashed before this burn) |
| Target | no-dot ATmega8 on ISP ribbon (was known-good USBasp; became burn victim) |
| Target signature | `0x1e9307` (ATmega8) |
| Board profile selected in IDE | Arduino NG / ATmega8 (bootloader burn) |

## Exact operations (from IDE verbose log)

1. `avrdude … -patmega8 -cusbasp -Pusb -e -Ulock:w:0x3F:m -Uefuse:w::m -Uhfuse:w:0xca:m -Ulfuse:w:0xdf:m`  
   → erase, lock, hfuse `0xca`, lfuse `0xdf` verified  
2. `avrdude … -Uflash:w:…/ATmegaBOOT-prod-firmware-2009-11-07.hex:i -Ulock:w:0x0F:m`  
   → **8170 bytes** flash written and verified; lock `0x0F` verified  

## Aftermath / recovery

Target (no-dot) was left as Arduino bootloader + Arduino fuses. Restored on Linux via yellow:

- flash `BOARD=usbasp-atmega8-usbisp` classic hex  
- fuses `hfuse=0xc9` `lfuse=0xef`  
- smoke: no-dot USB → classic `bcdDevice` 2.03, strings OK, caps `00 00 00 01`

## Lesson (process)

Never **Burn Bootloader** while a second programmer MCU sits on the ISP ribbon. The IDE programs the ribbon target with the **selected Board** fuse/bootloader set.

## Next acceptance (separate)

**SW SCK** — closed: [ACCEPTANCE-SCK-SWEEP-001](ACCEPTANCE-SCK-SWEEP-001.md) (USBasp2 → mega8-on-Nano-PCB).
