# Classic MS OS 2.0 and Windows WinUSB

This is a **Windows compatibility decision** for USBasp NG, not a claim that one MS OS 2.0 layout is universally “correct.”

## Profiles differ because topologies differ

| | Classic | HIDUART |
|--|---------|---------|
| USB topology | Non-composite, one vendor IF, EP0 only | Composite: vendor IF0 + HID |
| MS OS 2.0 layout | **Device-level** | **Configuration / Function subsets** |
| Set length | `0x9E` | `0xB2` |
| WinUSB scope | Whole device | Vendor IF0 only |
| Registry property | `DeviceInterfaceGUID` (REG_SZ) | `DeviceInterfaceGUIDs` (REG_MULTI_SZ) |
| `bcdDevice` | **2.03** (release identity) | **2.01** |

Same VID/PID `16c0:05dc`. Windows hardware IDs include `bcdDevice`, so the two profiles stay distinct in the driver store.

## Classic MS OS 2.0 descriptor

USBasp NG classic is a non-composite device with one vendor-specific interface.

The classic profile **intentionally** uses the device-level MS OS 2.0 descriptor layout:

```text
MS OS 2.0 Set Header
  ├── Compatible ID: WINUSB
  └── Registry Property: DeviceInterfaceGUID (REG_SZ)
Total descriptor set length: 0x9E
```

Configuration / Function subset headers are **not** used by the classic profile.

### Why (hardware A/B on Windows 11)

Same yellow-dot stick, same VID/PID, same USBasp protocol, BOS present, Linux vendor GET returned a full WINUSB blob in both cases. Only the MS OS 2.0 nesting changed:

| Layout | Set size | Windows 11 result |
|--------|----------|-------------------|
| Device-level (Set → WINUSB → GUID) | `0x9E` | **WinUSB** automatic bind ✓ |
| Nested (Set → Config → Function IF0 → …) | `0xAE` | Unknown device, yellow bang, **no driver publisher** ✗ |

So: for *this* non-composite classic topology on *this* target Windows, device-level binding worked and the nested layout broke automatic WinUSB selection. Nested layouts remain appropriate for **composite** HIDUART.

Do **not** change classic MS OS nesting without a new Windows hardware reason and a recorded A/B.

### `bcdDevice` 2.03

- `2.02` — briefly used during the nested experiment  
- `2.03` — rollback to device-level + cache-bust after the failed bind  

**2.03 is the release identity** for classic WinUSB. Do not bump `bcdDevice` for every descriptor tweak; use bumps for real firmware/compatibility revisions (or rare, documented Windows cache escapes).

## HIDUART (composite)

```text
MS OS 2.0 Set Header (0xB2)
  └── Configuration subset
        └── Function subset IF0
              ├── Compatible ID WINUSB
              └── DeviceInterfaceGUIDs (REG_MULTI_SZ)
```

HID interfaces bind by class. Prefer classic for MSVC avrdude / Arduino on Windows.

## Related docs

- [WINDOWS.md](WINDOWS.md) — acceptance and troubleshooting  
- [USB_DESCRIPTORS.md](USB_DESCRIPTORS.md) — topology table  
- [COMPATIBILITY.md](COMPATIBILITY.md) — L0 contract  
- Source: `firmware/src/usb/ms_os_20.c` (classic), `firmware/src_hid/usb_descriptors.h` (HIDUART)  
- Tests: `firmware/tests/core/test_classic_msos20.py`, `test_hiduart_desc.py`
