#!/usr/bin/env python3
"""Vendor GETCAPABILITIES (FUNC 127) from a USBasp on the bus."""
import sys
import usb.core
import usb.util

VID, PID = 0x16C0, 0x05DC
FUNC_GETCAPABILITIES = 127


def main() -> int:
    dev = usb.core.find(idVendor=VID, idProduct=PID)
    if dev is None:
        print("No USBasp 16c0:05dc", file=sys.stderr)
        return 1
    try:
        if dev.is_kernel_driver_active(0):
            dev.detach_kernel_driver(0)
    except Exception:
        pass
    data = dev.ctrl_transfer(
        bmRequestType=0xC0,
        bRequest=FUNC_GETCAPABILITIES,
        wValue=0,
        wIndex=0,
        data_or_wLength=4,
        timeout=2000,
    )
    b = bytes(data)
    caps = b[0] | (b[1] << 8) | (b[2] << 16) | (b[3] << 24)
    print(f"GETCAPABILITIES bytes: {b[0]:02x} {b[1]:02x} {b[2]:02x} {b[3]:02x}")
    print(f"packed: 0x{caps:08x}")
    print(f"TPI:    {bool(caps & 0x01)}")
    print(f"3MHz:   {bool(caps & (1 << 24))}")
    if b[1] or b[2]:
        print("WARN: bytes 1-2 should be 0 for avrdude-compatible caps")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
