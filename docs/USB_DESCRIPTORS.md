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
| Strings | 1=`www.fischl.de` 2=`USBasp` | same + serial |
| WinUSB | BOS + MS OS 2.0 (config → function IF0 → WINUSB) | MS OS 2.0 on vendor IF0 only |

**bcdDevice** is deliberate: same VID/PID, but Windows hardware IDs include bcdDevice, so classic **2.02** and HIDUART **2.01** get separate driver instances.

**USB string indices** (`usbasp/usb_strings.h`: `USB_STR_*`) are Device Descriptor fields for legacy `avrdude -c usbasp`. They are not V-USB `USB_CFG_DESCR_PROPS_STRING_*` flags (those select how usbdrv serves string blobs).

### Classic MS OS 2.0 layout (non-composite)

```text
Set header (0x0A)
  └─ Configuration subset (wTotalLength 0xA4)
       └─ Function subset IF0 (wTotalLength 0x9C)
            ├─ Compatible ID WINUSB (0x14)
            └─ DeviceInterfaceGUID REG_SZ (0x80)
Total set length: 0xAE
```

Classic sources: `firmware/src/usb/ms_os_20.c`, `firmware/src/usb_setup.c`.  
HIDUART: `firmware/src_hid/usb_descriptors.h`.

Host check: `python3 host/usbaspctl.py info`.

Structural golden tests: `firmware/tests/core/ms_os20_parse.py`, `test_classic_msos20.py`, `test_usb_descriptor_golden.py`.
