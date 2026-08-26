#!/usr/bin/env python3
"""HIDUART report descriptor, mega8 UCSRC URSEL, and nested MS OS layout."""
from pathlib import Path
import re
import sys

FW = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
from ms_os20_parse import load_hiduart_msos, validate_hiduart_winusb

DESC = (FW / "src_hid" / "usb_descriptors.h").read_text()
UART_C = (FW / "src_hid" / "uart.c").read_text()
UART_H = (FW / "src_hid" / "uart.h").read_text()


def test_logical_minimum_zero():
    assert re.search(r"0x15,\s*0x00,.*LOGICAL_MINIMUM \(0\)", DESC)
    assert re.search(r"0x15,\s*0x01", DESC) is None


def test_mega8_ursel_on_ucsrc():
    assert "USBASPUART_URSEL" in UART_H
    assert "(1 << USBASPUART_URSEL) | byte" in UART_C


def test_msos20_nested_composite_if0():
    """Composite profile: Configuration + Function subsets; WINUSB only on IF0."""
    blob = load_hiduart_msos(FW)
    assert len(blob) == 0xB2
    info = validate_hiduart_winusb(blob)
    assert info["layout"] == "nested-composite"
    assert info["interface"] == 0
    assert info["compatible_id"] == "WINUSB"
    assert info["total"] == 0xB2


def test_hiduart_bcddevice_not_classic():
    cmake = (FW / "CMakeLists.txt").read_text()
    assert 'set(USB_CFG_DEVICE_VERSION "0x01, 0x02")' in cmake
    assert 'set(USB_CFG_DEVICE_VERSION "0x03, 0x02")' in cmake
    proto = (FW / "include" / "usbasp" / "protocol.h").read_text()
    assert "0x16c0" in proto and "0x05dc" in proto


def test_hid_idle_protocol_setup():
    setup = (FW / "src_hid" / "usb_setup.c").read_text()
    assert "USBRQ_HID_GET_IDLE" in setup
    assert "USBRQ_HID_SET_IDLE" in setup
    assert "USBRQ_HID_GET_PROTOCOL" in setup
    assert "USBRQ_HID_SET_PROTOCOL" in setup


def test_hiduart_flash_restores_eeprom():
    mk = (FW / "Makefile").read_text()
    assert "flash:w:$(BUILDDIR)/$(HEX_NAME).hex" in mk
    assert "eeprom:w:$(BUILDDIR)/$(HEX_NAME).eep" in mk
    cmake = (FW / "CMakeLists.txt").read_text()
    assert "eeprom:w:${CMAKE_CURRENT_BINARY_DIR}/${APP_NAME}.eep" in cmake
    assert "restore EEPROM serial" in cmake


def main() -> int:
    test_logical_minimum_zero()
    test_mega8_ursel_on_ucsrc()
    test_msos20_nested_composite_if0()
    test_hiduart_bcddevice_not_classic()
    test_hid_idle_protocol_setup()
    test_hiduart_flash_restores_eeprom()
    print("ok  hiduart_desc")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
