#!/usr/bin/env python3
"""Regression: classic Device Descriptor string indices ≠ V-USB PROP flags."""
from pathlib import Path
import sys

FW = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
from ms_os20_parse import classic_string_contract

STRINGS = (FW / "include" / "usbasp" / "usb_strings.h").read_text()
MS = (FW / "src" / "usb" / "ms_os_20.c").read_text()


def test_classic_device_string_indices():
    """Catch iManufacturer/iProduct == 0 from mixing PROP flags with indices."""
    info = classic_string_contract(FW)
    assert info["iManufacturer"] == 1
    assert info["iProduct"] == 2
    assert info["iSerialNumber"] == 0
    assert info["manufacturer"] == "www.fischl.de"
    assert info["product"] == "USBasp"
    assert info["idVendor"] == 0x16C0
    assert info["idProduct"] == 0x05DC
    assert info["bcdDevice"] == 0x0202
    assert info["bcdUSB"] == 0x0201


def test_string_contract_not_prop_flags():
    assert "USB_STR_MANUFACTURER" in STRINGS
    assert "USB_STR_PRODUCT" in STRINGS
    assert "USB_CFG_DESCR_PROPS_STRING_VENDOR" not in MS.split("usbDescriptorDevice[]")[1].split(
        "usbasp_bos_descriptor"
    )[0]


def main() -> int:
    test_classic_device_string_indices()
    test_string_contract_not_prop_flags()
    print("ok  classic_device_strings")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
