#!/usr/bin/env python3
"""Build a 4-char USB string descriptor as Intel HEX for HIDUART EEPROM.

V-USB stores iSerial at EEPROM 0: bLength=10, bType=3, then 4 UTF-16LE ASCII chars.
Classic USBasp has no serial (L0). Use only on hiduart images.

  python3 host/usbasp-serial-eep.py YEL0 > /tmp/serial.eep
  avrdude -c usbasp -p atmega8 -U eeprom:w:/tmp/serial.eep:i
"""
from __future__ import annotations
import re
import sys


def eep_bytes(serial: str) -> bytes:
    if not re.fullmatch(r"[A-Za-z0-9]{4}", serial):
        raise SystemExit("serial must be exactly 4 chars [A-Za-z0-9]")
    payload = bytes([10, 3])
    for ch in serial:
        payload += bytes([ord(ch), 0])
    return payload


def intel_hex(data: bytes, base: int = 0) -> str:
    rec = f":{len(data):02X}{base:04X}00{data.hex().upper()}"
    checksum = (-sum(bytes.fromhex(rec[1:]))) & 0xFF
    return f"{rec}{checksum:02X}\n:00000001FF\n"


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: usbasp-serial-eep.py XXXX", file=sys.stderr)
        return 2
    sys.stdout.write(intel_hex(eep_bytes(sys.argv[1])))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
