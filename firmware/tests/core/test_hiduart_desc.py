#!/usr/bin/env python3
"""HIDUART report descriptor and mega8 UCSRC URSEL (source checks)."""
from pathlib import Path
import re

FW = Path(__file__).resolve().parents[2]
DESC = (FW / "src_hid" / "usb_descriptors.h").read_text()
UART_C = (FW / "src_hid" / "uart.c").read_text()
UART_H = (FW / "src_hid" / "uart.h").read_text()


def test_logical_minimum_zero():
    assert re.search(r"0x15,\s*0x00,.*LOGICAL_MINIMUM \(0\)", DESC)
    assert re.search(r"0x15,\s*0x01", DESC) is None


def test_mega8_ursel_on_ucsrc():
    assert "USBASPUART_URSEL" in UART_H
    assert "(1 << USBASPUART_URSEL) | byte" in UART_C


def test_msos20_winusb_only_if0():
    """WINUSB + DeviceInterfaceGUIDs only on vendor IF0; HID IFs use class binding."""
    start = DESC.index("MS_2_0_OS_DESCRIPTOR_SET[]")
    hiduart = DESC[start : DESC.index("usbDescriptorHidReport")]
    packed = hiduart.replace(" ", "").replace("\n", "")
    assert "'G',0x00,'U',0x00,'I',0x00,'D',0x00,'s',0x00" in packed
    assert "MS_OS_20_REG_PROPERTY_REG_MULTI_SZ" in hiduart
    assert hiduart.count("'W','I','N','U','S','B'") == 1
    assert "0xB2, 0x00" in hiduart
    assert "0xBE, 0x01" not in hiduart


def test_hiduart_bcddevice_not_classic():
    cmake = (FW / "CMakeLists.txt").read_text()
    assert 'set(USB_CFG_DEVICE_VERSION "0x01, 0x02")' in cmake
    assert 'set(USB_CFG_DEVICE_VERSION "0x02, 0x02")' in cmake
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
    test_msos20_winusb_only_if0()
    test_hiduart_bcddevice_not_classic()
    test_hid_idle_protocol_setup()
    test_hiduart_flash_restores_eeprom()
    print("ok  hiduart_desc")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
