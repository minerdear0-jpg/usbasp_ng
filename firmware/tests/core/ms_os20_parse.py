#!/usr/bin/env python3
"""Parse MS OS 2.0 descriptor sets from C PROGMEM initializers."""
from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

# Descriptor types (little-endian wDescriptorType values as used in MS OS 2.0)
MSOS_SET_HEADER = 0x00
MSOS_CONFIG_SUBSET = 0x01
MSOS_FUNCTION_SUBSET = 0x02
MSOS_COMPATIBLE_ID = 0x03
MSOS_REG_PROPERTY = 0x04


def c_array_bytes(source: str, array_name: str, extra_macros: dict | None = None) -> bytes:
    """Extract initializer bytes for `name[...] = { ... };` (hex/char/macros)."""
    macros = dict(extra_macros or {})
    for m in re.finditer(
        r"#define\s+([A-Za-z_][A-Za-z0-9_]*)\s+(.+)",
        source,
    ):
        name, val = m.group(1), m.group(2).split("/*")[0].strip()
        compact = val.replace(" ", "")
        if re.fullmatch(r"(?:0x[0-9A-Fa-f]+,)*0x[0-9A-Fa-f]+", compact):
            macros[name] = val
        elif re.fullmatch(r"\d+", compact):
            macros[name] = compact
    # enum { NAME = N, ... }
    for m in re.finditer(
        r"\b([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(\d+)\s*,?",
        source,
    ):
        macros.setdefault(m.group(1), m.group(2))

    m = re.search(
        rf"{re.escape(array_name)}\s*(?:\[[^\]]*\])?\s*=\s*\{{(.*?)\}};",
        source,
        re.S,
    )
    if not m:
        raise ValueError(f"array {array_name} not found")
    body = m.group(1)
    body = re.sub(r"/\*.*?\*/", "", body, flags=re.S)
    body = re.sub(r"//.*?$", "", body, flags=re.M)
    # Expand known macros (longest names first)
    for name in sorted(macros, key=len, reverse=True):
        body = re.sub(rf"\b{name}\b", macros[name], body)

    out: list[int] = []
    for tok in re.finditer(
        r"0x([0-9A-Fa-f]+)|\b(\d+)\b|'((?:\\.|[^'\\]))'",
        body,
    ):
        if tok.group(1) is not None:
            out.append(int(tok.group(1), 16) & 0xFF)
        elif tok.group(2) is not None:
            out.append(int(tok.group(2)) & 0xFF)
        else:
            ch = tok.group(3)
            if ch.startswith("\\"):
                raise ValueError(f"unsupported escape in {array_name}: {ch}")
            out.append(ord(ch) & 0xFF)
    return bytes(out)


def _u16(blob: bytes, off: int) -> int:
    return blob[off] | (blob[off + 1] << 8)


@dataclass
class MsOsNode:
    kind: int
    length: int
    offset: int
    children: list
    raw: bytes


def parse_ms_os20(blob: bytes) -> MsOsNode:
    if len(blob) < 10:
        raise ValueError("too short for set header")
    if _u16(blob, 0) != 0x0A or _u16(blob, 2) != MSOS_SET_HEADER:
        raise ValueError("missing set header")
    total = _u16(blob, 8)
    if total != len(blob):
        raise ValueError(f"set total {total:#x} != len {len(blob):#x}")

    root = MsOsNode(MSOS_SET_HEADER, 0x0A, 0, [], blob[0:10])
    off = 10
    while off < len(blob):
        if off + 4 > len(blob):
            raise ValueError("truncated descriptor")
        header_len = _u16(blob, off)
        kind = _u16(blob, off + 2)
        # Config/function subset: bLength is header only; wTotalLength nests body.
        if kind in (MSOS_CONFIG_SUBSET, MSOS_FUNCTION_SUBSET):
            if header_len != 0x08 or off + 8 > len(blob):
                raise ValueError(f"bad subset header at {off:#x}")
            subset_total = _u16(blob, off + 6)
            if subset_total < 8 or off + subset_total > len(blob):
                raise ValueError(f"bad subset total {subset_total:#x} at {off:#x}")
            node = MsOsNode(kind, subset_total, off, [], blob[off : off + subset_total])
            # Parse nested descriptors after the 8-byte subset header
            nested_off = 8
            nested = node.raw
            while nested_off < len(nested):
                nlen = _u16(nested, nested_off)
                nkind = _u16(nested, nested_off + 2)
                if nkind in (MSOS_CONFIG_SUBSET, MSOS_FUNCTION_SUBSET):
                    if nlen != 0x08:
                        raise ValueError("nested subset header size")
                    ntotal = _u16(nested, nested_off + 6)
                    child = MsOsNode(
                        nkind, ntotal, off + nested_off, [], nested[nested_off : nested_off + ntotal]
                    )
                    # features inside function
                    feat_off = 8
                    while feat_off < ntotal:
                        flen = _u16(child.raw, feat_off)
                        fkind = _u16(child.raw, feat_off + 2)
                        child.children.append(
                            MsOsNode(
                                fkind,
                                flen,
                                off + nested_off + feat_off,
                                [],
                                child.raw[feat_off : feat_off + flen],
                            )
                        )
                        feat_off += flen
                    node.children.append(child)
                    nested_off += ntotal
                else:
                    child = MsOsNode(
                        nkind, nlen, off + nested_off, [], nested[nested_off : nested_off + nlen]
                    )
                    node.children.append(child)
                    nested_off += nlen
            root.children.append(node)
            off += subset_total
        else:
            if header_len < 4 or off + header_len > len(blob):
                raise ValueError(f"bad length {header_len:#x} at {off:#x}")
            node = MsOsNode(kind, header_len, off, [], blob[off : off + header_len])
            root.children.append(node)
            off += header_len
    return root


def validate_classic_winusb(blob: bytes) -> dict:
    """Classic non-composite: Set → WINUSB → DeviceInterfaceGUID (no subsets)."""
    root = parse_ms_os20(blob)
    kinds = [c.kind for c in root.children]
    assert MSOS_CONFIG_SUBSET not in kinds, "classic must not use Configuration subset"
    assert MSOS_FUNCTION_SUBSET not in kinds, "classic must not use Function subset"
    assert len(root.children) == 2, kinds
    assert root.children[0].kind == MSOS_COMPATIBLE_ID
    assert root.children[0].raw[4:10] == b"WINUSB"
    assert root.children[1].kind == MSOS_REG_PROPERTY
    prop = root.children[1].raw
    name = prop[8 : 8 + _u16(prop, 6)]
    assert b"D\x00e\x00v\x00i\x00c\x00e\x00I\x00n\x00t\x00e\x00r\x00f\x00a\x00c\x00e\x00G\x00U\x00I\x00D\x00" in name
    # Singular GUID (REG_SZ), not GUIDs (REG_MULTI_SZ)
    assert b"G\x00U\x00I\x00D\x00s\x00" not in name
    return {
        "total": len(blob),
        "layout": "device-level",
        "compatible_id": "WINUSB",
    }


def validate_hiduart_winusb(blob: bytes) -> dict:
    """HIDUART composite: Set → Config → Function IF0 → WINUSB → DeviceInterfaceGUIDs."""
    root = parse_ms_os20(blob)
    assert len(root.children) == 1, [c.kind for c in root.children]
    cfg = root.children[0]
    assert cfg.kind == MSOS_CONFIG_SUBSET
    assert len(cfg.children) == 1
    fn = cfg.children[0]
    assert fn.kind == MSOS_FUNCTION_SUBSET
    assert fn.raw[4] == 0  # vendor IF0
    assert len(fn.children) == 2
    assert fn.children[0].kind == MSOS_COMPATIBLE_ID
    assert fn.children[0].raw[4:10] == b"WINUSB"
    assert fn.children[1].kind == MSOS_REG_PROPERTY
    prop = fn.children[1].raw
    name = prop[8 : 8 + _u16(prop, 6)]
    assert b"G\x00U\x00I\x00D\x00s\x00" in name  # plural MULTI_SZ
    return {
        "total": len(blob),
        "layout": "nested-composite",
        "config_subset": cfg.length,
        "function_subset": fn.length,
        "interface": 0,
        "compatible_id": "WINUSB",
    }


def load_classic_msos(fw_root: Path) -> bytes:
    src = (fw_root / "src" / "usb" / "ms_os_20.c").read_text()
    return c_array_bytes(src, "usbasp_ms_os_20_set", {"USBASP_MS_OS_VENDOR_CODE": "0x5D"})


def load_hiduart_msos(fw_root: Path) -> bytes:
    """Active HIDUART set is MS_2_0_OS_DESCRIPTOR_SET before the #else (non-HID) branch."""
    src = (fw_root / "src_hid" / "usb_descriptors.h").read_text()
    # Truncate at the alternate #else block so we don't pick the duplicate name.
    cut = src.find("\n#else")
    if cut < 0:
        raise ValueError("expected #else in usb_descriptors.h")
    head = src[:cut]
    extras = {
        "MS_OS_20_SET_HEADER_DESCRIPTOR": "0x00, 0x00",
        "MS_OS_20_SUBSET_HEADER_CONFIGURATION": "0x01, 0x00",
        "MS_OS_20_SUBSET_HEADER_FUNCTION": "0x02, 0x00",
        "MS_OS_20_FEATURE_COMPATIBLE_ID": "0x03, 0x00",
        "MS_OS_20_FEATURE_REG_PROPERTY": "0x04, 0x00",
        "MS_OS_20_REG_PROPERTY_REG_MULTI_SZ": "0x07, 0x00",
        "VENDOR_CODE": "0x5D",
    }
    return c_array_bytes(head, "MS_2_0_OS_DESCRIPTOR_SET", extras)


def load_classic_bos(fw_root: Path) -> bytes:
    src = (fw_root / "src" / "usb" / "ms_os_20.c").read_text()
    hdr = (fw_root / "include" / "usbasp" / "ms_os_20.h").read_text()
    vendor = (fw_root / "include" / "usbasp" / "ms_os_vendor.h").read_text()
    # USBDESCR_BOS lives in the header; vendor code may be numeric or macro.
    extras = {"USBDESCR_BOS": "0x0F", "USBASP_MS_OS_VENDOR_CODE": "0x5D"}
    for m in re.finditer(r"#define\s+(USBDESCR_BOS|USBASP_MS_OS_VENDOR_CODE)\s+(0x[0-9A-Fa-f]+)", hdr + "\n" + vendor):
        extras[m.group(1)] = m.group(2)
    return c_array_bytes(src, "usbasp_bos_descriptor", extras)


def parse_cfg_char_string(source: str, macro_name: str) -> str:
    """Decode `#define NAME 'a', 'b', ...` into a Python str."""
    m = re.search(rf"#define\s+{re.escape(macro_name)}\s+(.+)", source)
    if not m:
        raise ValueError(f"macro {macro_name} not found")
    chars: list[str] = []
    for tok in re.finditer(r"'((?:\\.|[^'\\]))'", m.group(1)):
        ch = tok.group(1)
        if ch.startswith("\\"):
            raise ValueError(f"unsupported escape in {macro_name}: {ch}")
        chars.append(ch)
    return "".join(chars)


def load_classic_device_descriptor(fw_root: Path) -> bytes:
    """18-byte classic device descriptor with string indices resolved."""
    src = (fw_root / "src" / "usb" / "ms_os_20.c").read_text()
    strings = (fw_root / "include" / "usbasp" / "usb_strings.h").read_text()
    cfg_in = (fw_root / "cmake" / "usbconfig.h.in").read_text()
    # Classic cmake: class 0xff, bcdDevice 2.03 (little-endian bytes in macro).
    extras = {
        "USBDESCR_DEVICE": "0x01",
        "USB_CFG_DEVICE_CLASS": "0xff",
        "USB_CFG_DEVICE_SUBCLASS": "0",
        "USB_CFG_VENDOR_ID": "0xc0, 0x16",
        "USB_CFG_DEVICE_ID": "0xdc, 0x05",
        "USB_CFG_DEVICE_VERSION": "0x03, 0x02",
    }
    blob = c_array_bytes(src + "\n" + strings + "\n" + cfg_in, "usbDescriptorDevice", extras)
    if len(blob) != 18:
        raise ValueError(f"device descriptor length {len(blob)}, expected 18")
    return blob


def classic_string_contract(fw_root: Path) -> dict:
    """Parsed classic USB string identity for avrdude -c usbasp."""
    dev = load_classic_device_descriptor(fw_root)
    cfg_in = (fw_root / "cmake" / "usbconfig.h.in").read_text()
    return {
        "iManufacturer": dev[14],
        "iProduct": dev[15],
        "iSerialNumber": dev[16],
        "manufacturer": parse_cfg_char_string(cfg_in, "USB_CFG_VENDOR_NAME"),
        "product": parse_cfg_char_string(cfg_in, "USB_CFG_DEVICE_NAME"),
        "bcdUSB": dev[2] | (dev[3] << 8),
        "bcdDevice": dev[12] | (dev[13] << 8),
        "idVendor": dev[8] | (dev[9] << 8),
        "idProduct": dev[10] | (dev[11] << 8),
    }
