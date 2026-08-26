# USB descriptors (classic vs HIDUART)

USBasp protocol (FUNC 1–16 / 127) is shared. USB **topology** is a profile.

| | Classic | HIDUART |
|--|---------|---------|
| VID:PID | `16c0:05dc` | `16c0:05dc` |
| bcdDevice | **2.03** | **2.01** |
| bcdUSB | 2.01 (BOS) | 2.01 |
| Interfaces | 1 vendor | vendor + 2 HID |
| Endpoints | EP0 only | + interrupt HID |
| iSerial | none | 4 chars EEPROM |
| Strings | 1=`www.fischl.de` 2=`USBasp` | same + serial |
| MS OS 2.0 | **device-level** `0x9E` | **nested** `0xB2` (IF0) |

**bcdDevice** is deliberate: Windows hardware IDs include it, so classic **2.03** and HIDUART **2.01** stay separate. Treat **2.03** as classic release identity (not a rolling cache-bust counter).

**USB string indices** (`usbasp/usb_strings.h`) are Device Descriptor fields for legacy `avrdude -c usbasp`. They are not V-USB `USB_CFG_DESCR_PROPS_STRING_*` flags.

Windows WinUSB decision and A/B evidence: **[USB_WINDOWS.md](USB_WINDOWS.md)**.

### Classic MS OS 2.0 (non-composite)

```text
Set header (0x0A)
  ├─ Compatible ID WINUSB (0x14)
  └─ DeviceInterfaceGUID REG_SZ (0x80)
Total: 0x9E
```

No Configuration/Function subsets on classic (verified on Win11; nested `0xAE` left the device unbound).

### HIDUART MS OS 2.0 (composite)

```text
Set (0xB2)
  └─ Configuration subset
       └─ Function subset IF0
            ├─ WINUSB
            └─ DeviceInterfaceGUIDs REG_MULTI_SZ
```

Classic sources: `firmware/src/usb/ms_os_20.c`.  
HIDUART: `firmware/src_hid/usb_descriptors.h`.

Host: `python3 host/usbaspctl.py info`.  
Golden: `ms_os20_parse.py`, `test_classic_msos20.py`, `test_hiduart_desc.py`.
