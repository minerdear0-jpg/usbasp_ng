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
    test_hiduart_flash_restores_eeprom()
    print("ok  hiduart_desc")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
