# USB descriptors (classic vs HIDUART)

USBasp protocol (FUNC 1–16 / 127) is shared. USB **topology** is a profile.

| | Classic | HIDUART |
|--|---------|---------|
| VID:PID | `16c0:05dc` | `16c0:05dc` |
| bcdDevice | **2.02** | **2.01** |
| bcdUSB | 2.01 (BOS) | 2.01 |
| Interfaces | 1 vendor | vendor + 2 HID |
| Endpoints | EP0 only | + interrupt HID |
| iSerial | none | 4 chars EEPROM |
| WinUSB | BOS + MS OS 2.0 device-level | MS OS 2.0 on vendor IF0 only |

Classic sources: `firmware/src/usb/ms_os_20.c`, `firmware/src/usb_setup.c`.  
HIDUART: `firmware/src_hid/usb_descriptors.h`.

Host check: `python3 host/usbaspctl.py info`.

Source golden tests: `firmware/tests/core/test_usb_descriptor_golden.py`, `test_classic_msos20.py`.
