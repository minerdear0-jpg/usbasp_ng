# Arduino IDE integration

USBasp NG is not an Arduino core. Use stock programmer **USBasp** and flash **classic** (`usbasp.hex`), not HIDUART.

See [docs/ARDUINO.md](../docs/ARDUINO.md) and [docs/WINDOWS.md](../docs/WINDOWS.md).

## Windows + WinUSB (classic)

Device Manager should show Microsoft **WinUSB**. That is correct.

Arduino IDE **1.8.19** ships **avrdude 6.3**, which cannot open WinUSB. Burn Bootloader then fails with `cannot query manufacturer` even though AVRDUDESS works. Fix the **IDE avrdude**, not the firmware — details in [docs/ARDUINO.md](../docs/ARDUINO.md) and [docs/KNOWN_ISSUES.md](../docs/KNOWN_ISSUES.md).

## Tools → Port

Grayed out / no COM port is normal. Use **Upload Using Programmer** or **Burn Bootloader**, not the normal Upload button.
