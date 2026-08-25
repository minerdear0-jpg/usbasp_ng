#!/usr/bin/env python3
"""HIDUART loopback: SET_REPORT baud, EP1 OUT, expect same bytes on EP1 IN.

Needs hiduart on the bus (yellow-dot). Loopback is ATmega8 TQFP pins 30–31 (PD0–PD1),
not MOSI/MISO. Detaches kernel HID on interfaces 1 and 2 for the duration.
"""
from __future__ import annotations
import sys
import time
import usb.core
import usb.util

VID, PID = 0x16C0, 0x05DC
# ~9600 @ 12 MHz U2X: UBRR = F_CPU/(8*baud)-1
UBRR_9600 = 155
UART_8N1 = 0x18
UART_STATE_ENABLED = 16


def main() -> int:
    want = sys.argv[1] if len(sys.argv) > 1 else "YEL0"
    payload = b"HELLO"
    dev = usb.core.find(idVendor=VID, idProduct=PID)
    if dev is None:
        print("No USBasp 16c0:05dc", file=sys.stderr)
        return 1
    ser = usb.util.get_string(dev, dev.iSerialNumber) or ""
    print(f"serial={ser!r}")
    if want and ser != want:
        print(f"expected iSerial {want}", file=sys.stderr)
        return 1
    for n in (1, 2):
        try:
            if dev.is_kernel_driver_active(n):
                dev.detach_kernel_driver(n)
        except usb.core.USBError:
            pass
    usb.util.claim_interface(dev, 1)
    usb.util.claim_interface(dev, 2)
    uart_on = False
    ok = False
    saw_in = False
    try:
        feat = bytes([UBRR_9600 & 0xFF, UBRR_9600 >> 8, UART_8N1, 0, 0, 0, 0, 0])
        dev.ctrl_transfer(0x21, 0x09, 0x0300, 1, feat, timeout=2000)
        try:
            mon = bytes(dev.read(0x82, 8, timeout=800))
            uart_on = (mon[7] & UART_STATE_ENABLED) != 0
            print(f"monitor state={mon[7]} uart={'on' if uart_on else 'off'}")
        except usb.core.USBError as e:
            print("EP2 IN", e, file=sys.stderr)
        pkt = payload + bytes(7 - len(payload)) + bytes([len(payload)])
        dev.write(0x01, pkt, timeout=2000)
        time.sleep(0.05)
        for _ in range(8):
            try:
                data = bytes(dev.read(0x81, 8, timeout=400))
            except usb.core.USBError as e:
                print("EP1 IN", e, file=sys.stderr)
                break
            saw_in = True
            cnt = data[7]
            chunk = data[: min(cnt, 7)]
            print(f"IN cnt={cnt} {chunk!r}")
            if chunk == payload:
                ok = True
                break
    finally:
        for n in (1, 2):
            try:
                usb.util.release_interface(dev, n)
            except usb.core.USBError:
                pass
    if ok:
        print("LOOPBACK OK")
        return 0
    if uart_on and saw_in:
        print(
            "LOOPBACK FAIL: UART enabled, HID IN empty. "
            "Short ATmega8 PD0–PD1 (TQFP 30–31), not MOSI/MISO.",
            file=sys.stderr,
        )
    else:
        print("LOOPBACK FAIL", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
