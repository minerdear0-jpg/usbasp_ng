#!/usr/bin/env python3
"""Host-side LE helpers must match firmware/include/usbasp/endian.h."""


def read_le16(p):
    return p[0] | (p[1] << 8)


def read_le32(p):
    return p[0] | (p[1] << 8) | (p[2] << 16) | (p[3] << 24)


def main():
    assert read_le16(bytes([0x34, 0x12])) == 0x1234
    assert read_le32(bytes([0x78, 0x56, 0x34, 0x12])) == 0x12345678
    # misaligned buffer from SETUP data[2]
    setup = bytes([0xC0, 9, 0xDE, 0xBC, 0x0A, 0x00, 0x00, 0x00])
    assert read_le32(setup[2:]) == 0x000ABCDE
    print("ok  endian")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
