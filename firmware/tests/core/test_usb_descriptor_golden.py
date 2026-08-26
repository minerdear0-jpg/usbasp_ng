#!/usr/bin/env python3
"""Golden USB identity: classic vs HIDUART topology + MS OS structure."""
from pathlib import Path
import sys

FW = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
from ms_os20_parse import classic_string_contract, load_classic_msos, validate_classic_winusb

MS = (FW / "src" / "usb" / "ms_os_20.c").read_text()
CMAKE = (FW / "CMakeLists.txt").read_text()
HID_DESC = (FW / "src_hid" / "usb_descriptors.h").read_text()
PROTO = (FW / "include" / "usbasp" / "protocol.h").read_text()
ISP = (FW / "src" / "isp.c").read_text()


def test_vid_pid():
    assert "0x16c0" in PROTO and "0x05dc" in PROTO


def test_classic_device_descriptor_shape():
    info = classic_string_contract(FW)
    assert info["bcdUSB"] == 0x0201
    assert info["iSerialNumber"] == 0
    assert info["iManufacturer"] == 1
    assert info["iProduct"] == 2


def test_classic_device_string_indices():
    """Legacy avrdude -c usbasp needs Fischl strings via non-zero indices."""
    info = classic_string_contract(FW)
    assert info["iManufacturer"] == 1
    assert info["iProduct"] == 2
    assert info["iSerialNumber"] == 0
    assert info["manufacturer"] == "www.fischl.de"
    assert info["product"] == "USBasp"


def test_classic_msos_structural():
    info = validate_classic_winusb(load_classic_msos(FW))
    assert info["interface"] == 0
    assert info["compatible_id"] == "WINUSB"


def test_classic_cmake_props():
    assert 'set(USB_CFG_DEVICE_VERSION "0x02, 0x02")' in CMAKE
    assert "USB_PROP_LENGTH(18)" in CMAKE
    assert 'set(USB_CFG_DESCR_PROPS_UNKNOWN "USB_PROP_IS_DYNAMIC")' in CMAKE


def test_hiduart_not_classic_topology():
    assert "usbDescriptorHidReport" in HID_DESC
    assert "0x4B" in HID_DESC
    assert HID_DESC.count("USBDESCR_HID") >= 2
    assert 'set(USB_CFG_DEVICE_VERSION "0x01, 0x02")' in CMAKE


def test_bcddevice_is_deliberate_profile_key():
    """Same VID/PID; bcdDevice distinguishes classic WinUSB vs HIDUART for Windows IDs."""
    assert 'set(USB_CFG_DEVICE_VERSION "0x02, 0x02")' in CMAKE  # classic 2.02
    assert 'set(USB_CFG_DEVICE_VERSION "0x01, 0x02")' in CMAKE  # hiduart 2.01


def test_isp_bus_select_helpers():
    assert "void isp_bus_select_hw(void)" in ISP
    assert "void isp_bus_select_sw(void)" in ISP
    assert "isp_bus_select_hw()" in (FW / "src" / "sck.c").read_text()
    assert "isp_bus_select_sw()" in (FW / "src" / "sck.c").read_text()


def main() -> int:
    test_vid_pid()
    test_classic_device_descriptor_shape()
    test_classic_device_string_indices()
    test_classic_msos_structural()
    test_classic_cmake_props()
    test_hiduart_not_classic_topology()
    test_bcddevice_is_deliberate_profile_key()
    test_isp_bus_select_helpers()
    print("ok  usb_descriptor_golden")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
