#!/usr/bin/env python3
"""Dumb HID EP2 recorder for USBASP-NG diagnostics (schema-agnostic).

Writes raw 8-byte interrupt reports with a host wall-clock stamp.
Does not decode frames — use usbasp-trace.py later.

Usage:
  python3 host/usbasp-hidraw-log.py [serial] [out.bin]
  Default serial match: any 16c0:05dc with bcdDevice 2.01 (composite).
"""
from __future__ import annotations
import struct
import sys
import time

import usb.core
import usb.util

VID, PID = 0x16C0, 0x05DC
# Composite IF2 = EP 0x82 interrupt IN (monitor / diagnostics)
EP2_IN = 0x82
IF_MONITOR = 2
CAPTURE_MAGIC = b"USBDIAGv"


def write_capture_header(f) -> None:
    """16-byte USBDIAGv header (format=1, schema=1, record=16)."""
    hdr = bytearray(16)
    hdr[0:8] = CAPTURE_MAGIC
    hdr[8] = 1  # format_version
    hdr[9] = 1  # diag_schema
    hdr[10] = 16  # record_size
    f.write(hdr)


def find_dev(want_serial: str | None):
    for dev in usb.core.find(idVendor=VID, idProduct=PID, find_all=True):
        try:
            ser = usb.util.get_string(dev, dev.iSerialNumber) or ""
        except Exception:
            ser = ""
        if want_serial and ser != want_serial:
            continue
        # Prefer composite (bcdDevice 2.01); skip classic 2.03
        if dev.bcdDevice == 0x0203:
            continue
        return dev, ser
    return None, None


def main() -> int:
    want = sys.argv[1] if len(sys.argv) > 1 else ""
    out_path = sys.argv[2] if len(sys.argv) > 2 else "usbasp-diag.bin"
    if want in ("-h", "--help"):
        print(__doc__)
        return 0

    dev, ser = find_dev(want or None)
    if dev is None:
        print("No composite USBasp 16c0:05dc found", file=sys.stderr)
        return 1

    print(f"recording serial={ser!r} bcdDevice={dev.bcdDevice:04x} → {out_path}", flush=True)
    try:
        if dev.is_kernel_driver_active(IF_MONITOR):
            dev.detach_kernel_driver(IF_MONITOR)
    except (usb.core.USBError, NotImplementedError):
        pass
    usb.util.claim_interface(dev, IF_MONITOR)

    n = 0
    try:
        with open(out_path, "ab") as f:
            if f.tell() == 0:
                write_capture_header(f)
            while True:
                try:
                    data = bytes(dev.read(EP2_IN, 8, timeout=1000))
                except usb.core.USBTimeoutError:
                    continue
                except usb.core.USBError as e:
                    print(f"USB error: {e}", file=sys.stderr)
                    break
                if len(data) < 8:
                    continue
                # host stamp: uint64_t ns + 8 raw bytes
                ts = time.time_ns()
                f.write(struct.pack("<Q", ts) + data)
                f.flush()
                n += 1
                typ, flags, tlo, thi, a, b = data[0], data[1], data[2], data[3], data[4], data[5]
                print(f"{n:5d} type={typ:02x} flags={flags:02x} ts={thi:02x}{tlo:02x} a={a:02x} b={b:02x}")
    except KeyboardInterrupt:
        print(f"\nstopped, {n} frames → {out_path}")
    finally:
        usb.util.release_interface(dev, IF_MONITOR)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
