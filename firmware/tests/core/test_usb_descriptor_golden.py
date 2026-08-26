#!/usr/bin/env python3
"""Golden USB identity invariants for classic vs HIDUART (source-level)."""
from pathlib import Path

FW = Path(__file__).resolve().parents[2]
MS = (FW / "src" / "usb" / "ms_os_20.c").read_text()
CMAKE = (FW / "CMakeLists.txt").read_text()
HID_DESC = (FW / "src_hid" / "usb_descriptors.h").read_text()
PROTO = (FW / "include" / "usbasp" / "protocol.h").read_text()


def test_vid_pid():
    assert "0x16c0" in PROTO and "0x05dc" in PROTO


def test_classic_device_descriptor_shape():
    start = MS.index("usbDescriptorDevice[]")
    body = MS[start : MS.index("usbasp_bos_descriptor")]
    # bcdUSB 2.01 little-endian in source
    assert "0x01, 0x02" in body or "0x01,0x02" in body.replace(" ", "")
    assert "iSerial none" in body or "0, /* iSerial" in body
    assert "USB_CFG_DEVICE_VERSION" in body


def test_classic_msos_length_and_winusb():
    assert "USBASP_MS_OS_20_SET_LEN 0x9E" in (FW / "include" / "usbasp" / "ms_os_20.h").read_text()
    assert "USBASP_BOS_LEN 0x21" in (FW / "include" / "usbasp" / "ms_os_20.h").read_text()
    packed = MS.replace(" ", "").replace("\n", "")
    assert "'W','I','N','U','S','B'" in packed
    assert "MS_OS_20_SUBSET_HEADER_FUNCTION" not in MS


def test_classic_cmake_props():
    # classic branch: dynamic UNKNOWN (BOS), device length 18, no serial EEPROM
    assert 'set(USB_CFG_DEVICE_VERSION "0x02, 0x02")' in CMAKE
    assert "USB_PROP_LENGTH(18)" in CMAKE
    assert 'set(USB_CFG_DESCR_PROPS_UNKNOWN "USB_PROP_IS_DYNAMIC")' in CMAKE


def test_hiduart_not_classic_topology():
    assert "usbDescriptorHidReport" in HID_DESC
    assert "0x4B" in HID_DESC  # composite config wTotalLength
    assert HID_DESC.count("USBDESCR_HID") >= 2
    assert 'set(USB_CFG_DEVICE_VERSION "0x01, 0x02")' in CMAKE
    assert "HAVE_INTRIN" not in MS


def test_profiles_share_vid_pid_differ_bcd():
    assert 'set(USB_CFG_DEVICE_VERSION "0x02, 0x02")' in CMAKE  # classic
    assert 'set(USB_CFG_DEVICE_VERSION "0x01, 0x02")' in CMAKE  # hiduart


def main() -> int:
    test_vid_pid()
    test_classic_device_descriptor_shape()
    test_classic_msos_length_and_winusb()
    test_classic_cmake_props()
    test_hiduart_not_classic_topology()
    test_profiles_share_vid_pid_differ_bcd()
    print("ok  usb_descriptor_golden")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
