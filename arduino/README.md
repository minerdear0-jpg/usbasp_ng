# Replace Arduino IDE avrdude with a WinUSB-capable build (Windows x64)

Arduino IDE 1.8.19 ships avrdude 6.3, which cannot open Microsoft WinUSB.
USBasp NG classic needs avrdude 7+ / 8.x MSVC (or AVRDUDESS).

## Quick path

1. Download `avrdude-v8.*-windows-x64.zip` from
   https://github.com/avrdudes/avrdude/releases
2. Run PowerShell **as Administrator** (Program Files is protected):

```powershell
cd path\to\usbasp_NG\arduino
.\replace-avrdude.ps1 -ZipPath "$env:USERPROFILE\Downloads\avrdude-v8.0-windows-x64.zip"
```

3. Restart Arduino IDE. Enable verbose upload. Burn Bootloader / Upload Using Programmer.
4. Confirm the log shows avrdude **7.x or 8.x**, not `6.3-20190619`.

Default IDE tools dir:
`C:\Program Files (x86)\Arduino\hardware\tools\avr`

Override with `-ArduinoAvrRoot "D:\Arduino\hardware\tools\avr"` if needed.

The script backs up `bin\avrdude.exe` and `etc\avrdude.conf` to `*.bak-usbasp-ng` once.

## Prefer not to touch Program Files?

Use [AVRDUDESS](https://github.com/ZakKemble/AVRDUDESS) / standalone avrdude for ISP, and keep Arduino for editing only.
