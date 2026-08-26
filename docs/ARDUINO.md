# Arduino IDE (classic USBasp NG)

Arduino does not talk USBasp itself. It runs **avrdude** with `-c usbasp`. Flash **classic** firmware. Do not require HIDUART.

USBasp is not a COM port. **Tools → Port** grayed out is normal.

## Get Board Info vs Burn Bootloader

**Tools → Get Board Info** talks to an Arduino **over Serial** (running sketch + CDC/UART). It does **not** use USBasp.

When the stick is only a programmer (no Arduino sketch/COM port), Get Board Info stays empty or fails. That says nothing about WinUSB or classic NG. **Burn Bootloader** / **Upload Using Programmer** use avrdude → USBasp ISP — a different path.

## Arduino IDE 1.8.19 + WinUSB

Stock IDE ships **avrdude 6.3-20190619**.

### Earlier failure (pre string-index fix)

With classic Device Descriptor `iManufacturer`/`iProduct` = 0, 6.3 failed discovery even when Device Manager showed WinUSB:

```text
avrdude: Warning: cannot query manufacturer for device: Invalid argument
avrdude: Warning: cannot query product for device: Invalid argument
avrdude: error: could not find USB device with vid=0x16c0 pid=0x5dc
         vendor='www.fischl.de' product='USBasp'
```

That was empty USB string indices (V-USB PROP flags mistaken for indices), not “needs Fischl 2011 firmware.” Fixed by `USB_STR_*` → `www.fischl.de` / `USBasp`.

### Accidental acceptance (2026-08-26, after string fix)

Canonical record: **[ACCEPTANCE-WIN11-USBASP-001](acceptance/ACCEPTANCE-WIN11-USBASP-001.md)**.

Strict conclusion: classic NG completed a full destructive ISP cycle on Windows 11 with WinUSB and avrdude 6.3 (erase, fuses, ~8 KiB flash, verify). That does **not** close software SCK and does **not** by itself widen the compatibility matrix.

**Do not** reinstall libusbK just to please Arduino 1.8 — that undoes the zero-driver path.

`-c usbasp-clone` only skips the Fischl string check. Prefer fixing strings (done on classic) over relying on clone.

### Fixes if 6.3 still fails on your machine

1. **Preferred:** AVRDUDESS / standalone **avrdude 7.x or 8.x MSVC**.
2. **Keep IDE 1.8:** [`arduino/replace-avrdude.ps1`](../arduino/replace-avrdude.ps1) / [`arduino/README.md`](../arduino/README.md).
3. **IDE 2.x:** check verbose for bundled avrdude version; replace if still 6.3-era and broken on your host.

## Safety: what sits on the ISP ribbon

Burn Bootloader programs **whatever ATmega is on the ISP cable**, with the fuses/bootloader of the **selected Board** (e.g. Arduino NG / ATmega8 → `lfuse=0xdf`, `hfuse=0xca`, ATmegaBOOT).

Never leave a second USBasp (or any “spare programmer”) on the ribbon and hit Burn Bootloader for an Arduino board profile. Bench lesson: no-dot was wiped that way and had to be restored (classic usbisp hex + `lfuse=0xef` / `hfuse=0xc9`) through the yellow stick.

Recovery needs a second working programmer. Keep one known-good stick off the ribbon during IDE burn.

## Arduino IDE 2.x (when bundled avrdude is new enough)

1. Tools → Board → target MCU board (not the programmer).
2. Tools → Programmer → **USBasp**.
3. Sketch → Upload Using Programmer, or Tools → Burn Bootloader.

No custom “USBasp NG” programmer type.

## This tree

This repo is programmer firmware. It is not an Arduino board package. Do not vendor an Arduino core here.
