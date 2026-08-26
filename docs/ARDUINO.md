# Arduino IDE (classic USBasp NG)

Arduino does not talk USBasp itself. It runs **avrdude** with `-c usbasp`. Flash **classic** firmware. Do not require HIDUART.

USBasp is not a COM port. **Tools → Port** grayed out is normal.

## Arduino IDE 1.8.19 + WinUSB (bench 2026-08-26)

Stock IDE ships **avrdude 6.3-20190619**. That build does **not** open Microsoft WinUSB. Burn Bootloader fails even when Device Manager shows WinUSB and AVRDUDESS works:

```text
avrdude: Warning: cannot query manufacturer for device: Invalid argument
avrdude: Warning: cannot query product for device: Invalid argument
avrdude: error: could not find USB device with vid=0x16c0 pid=0x5dc
         vendor='www.fischl.de' product='USBasp'
```

This is **not** “needs Fischl 2011 firmware”. NG already has those USB strings. The host tool is too old for WinUSB.

**Do not** reinstall libusbK just to please Arduino 1.8 — that undoes the zero-driver path.

### Fixes (pick one)

1. **Preferred:** use AVRDUDESS / standalone **avrdude 7.x or 8.x MSVC** for ISP (already proven on this bench).
2. **Keep IDE 1.8:** replace the IDE’s avrdude with a modern MSVC build:
   - Download [avrdude Windows x64](https://github.com/avrdudes/avrdude/releases) (`avrdude-v8.*-windows-x64.zip`).
   - Backup, then overwrite:
     - `C:\Program Files (x86)\Arduino\hardware\tools\avr\bin\avrdude.exe`
     - and matching `avrdude.conf` under `...\avr\etc\` if the release ships one.
   - Re-run Burn Bootloader with verbose upload enabled.
3. **IDE 2.x:** nicer UI, but still check verbose for the **bundled** avrdude version. If it is still 6.3-era, apply the same replace or use AVRDUDESS.

`-c usbasp-clone` only skips the Fischl string check. It does **not** make avrdude 6.3 speak WinUSB.

## Arduino IDE 2.x (when bundled avrdude is new enough)

1. Tools → Board → target MCU board (not the programmer).
2. Tools → Programmer → **USBasp**.
3. Sketch → Upload Using Programmer, or Tools → Burn Bootloader.

No custom “USBasp NG” programmer type.

## This tree

This repo is programmer firmware. It is not an Arduino board package. Do not vendor an Arduino core here.
