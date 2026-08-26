#!/usr/bin/env python3
"""Classic is still one vendor interface; BOS/MS OS 2.0 only bind WinUSB."""
from pathlib import Path

FW = Path(__file__).resolve().parents[2]
MS = (FW / "src" / "usb" / "ms_os_20.c").read_text()
SETUP = (FW / "src" / "usb_setup.c").read_text()
CMAKE = (FW / "CMakeLists.txt").read_text()
ISO = FW / "src"


def test_winusb_device_level_set():
    assert "'W', 'I', 'N', 'U', 'S', 'B'" in MS or "'W','I','N','U','S','B'" in MS.replace(" ", "")
    assert "0x9E, 0x00" in MS
    assert "MS_OS_20_SUBSET_HEADER_FUNCTION" not in MS
    assert "0xB2, 0x00" not in MS
    assert "interrupt" not in MS.lower()


def test_bos_and_setup():
    assert "usbasp_bos_descriptor" in MS
    assert "USBDESCR_BOS" in SETUP
    assert "USBASP_MS_OS_VENDOR_CODE" in SETUP
    assert "MS_OS_2_0_DESCRIPTOR_INDEX" in SETUP
    assert "usbasp_vendor_setup" in SETUP


def test_bcddevice_split():
    assert 'set(USB_CFG_DEVICE_VERSION "0x02, 0x02")' in CMAKE
    assert 'set(USB_CFG_DEVICE_VERSION "0x01, 0x02")' in CMAKE
    assert 'set(USB_CFG_DEVICE_VERSION "0x00, 0x02")' not in CMAKE


def test_classic_src_not_composite():
    text = (ISO / "usb" / "ms_os_20.c").read_text()
    assert "usbDescriptorHidReport" not in text
    assert "bNumEndpoints" not in text


def main() -> int:
    test_winusb_device_level_set()
    test_bos_and_setup()
    test_bcddevice_split()
    test_classic_src_not_composite()
    print("ok  classic_msos20")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
