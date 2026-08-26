#!/usr/bin/env python3
"""Classic MS OS 2.0: device-level Set → WINUSB → GUID (non-composite)."""
from pathlib import Path
import sys

FW = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
from ms_os20_parse import load_classic_bos, load_classic_msos, validate_classic_winusb

SETUP = (FW / "src" / "usb_setup.c").read_text()
CMAKE = (FW / "CMakeLists.txt").read_text()
HDR = (FW / "include" / "usbasp" / "ms_os_20.h").read_text()
MS = (FW / "src" / "usb" / "ms_os_20.c").read_text()


def test_msos_structure():
    blob = load_classic_msos(FW)
    assert "USBASP_MS_OS_20_SET_LEN 0x9E" in HDR
    assert len(blob) == 0x9E
    info = validate_classic_winusb(blob)
    assert info["total"] == 0x9E
    assert info["layout"] == "device-level"
    assert info["compatible_id"] == "WINUSB"
    # Nested subsets broke Win11 auto-bind on classic; must stay absent.
    assert "MS_OS_20_SUBSET_HEADER_CONFIGURATION" not in MS
    assert "MS_OS_20_SUBSET_HEADER_FUNCTION" not in MS


def test_bos_points_at_set_length():
    bos = load_classic_bos(FW)
    assert len(bos) == 0x21
    assert bos[0] == 0x05 and bos[1] == 0x0F
    assert (bos[2] | (bos[3] << 8)) == 0x21
    ms_len = bos[29] | (bos[30] << 8)
    assert ms_len == 0x9E
    assert bos[31] == 0x5D  # USBASP_MS_OS_VENDOR_CODE


def test_setup_and_bcd():
    assert "USBDESCR_BOS" in SETUP
    assert "USBASP_MS_OS_VENDOR_CODE" in SETUP
    assert 'set(USB_CFG_DEVICE_VERSION "0x03, 0x02")' in CMAKE  # classic 2.03
    assert 'set(USB_CFG_DEVICE_VERSION "0x01, 0x02")' in CMAKE  # hiduart 2.01


def test_classic_not_composite_topology():
    assert "usbDescriptorHidReport" not in MS
    assert "0xB2, 0x00" not in MS  # HIDUART composite set size


def main() -> int:
    test_msos_structure()
    test_bos_points_at_set_length()
    test_setup_and_bcd()
    test_classic_not_composite_topology()
    print("ok  classic_msos20")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
