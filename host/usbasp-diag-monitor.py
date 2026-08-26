#!/usr/bin/env python3
"""Live diagnostics monitor (lab): HID EP2 → decode → stdout.

Usage:
  python3 host/usbasp-diag-monitor.py [serial]
  python3 host/usbasp-diag-monitor.py YEL0 --json

Detach kernel driver on IF2 for the duration. Ctrl+C to stop.
Production path will be Rust usbasp-ng-diag; this is the lab twin.
"""
from __future__ import annotations
import argparse
import json
import sys
import time

import usb.core
import usb.util

# Reuse decoder helpers from usbasp-trace
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import importlib.util

_spec = importlib.util.spec_from_file_location(
    "usbasp_trace", Path(__file__).resolve().parent / "usbasp-trace.py"
)
_trace = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(_trace)

VID, PID = 0x16C0, 0x05DC
EP2_IN = 0x82
IF_MONITOR = 2

TYPES = _trace.TYPES
TRANSPORT = _trace.TRANSPORT
RESET_ASSERT = _trace.RESET_ASSERT
RESET_RELEASE = _trace.RESET_RELEASE


def find_dev(want: str | None):
    for dev in usb.core.find(idVendor=VID, idProduct=PID, find_all=True):
        try:
            ser = usb.util.get_string(dev, dev.iSerialNumber) or ""
        except Exception:
            ser = ""
        if want and ser != want:
            continue
        if dev.bcdDevice == 0x0203:
            continue
        return dev, ser
    return None, None


def event_dict(data: bytes) -> dict:
    typ, flags, tlo, thi, a, b = data[0], data[1], data[2], data[3], data[4], data[5]
    ts = tlo | (thi << 8)
    name = TYPES.get(typ, f"TYPE_{typ}").lower()
    ev: dict = {"t_tick": ts, "type": name, "flags": flags, "a": a, "b": b}
    if typ == 1:
        ev.update(schema=a, profile=b, caps=flags)
    elif typ == 4:
        ev["reset"] = (
            "assert"
            if flags & RESET_ASSERT
            else "release" if flags & RESET_RELEASE else "unknown"
        )
    elif typ == 5:
        ev.update(sck_id=a, transport=TRANSPORT.get(b, str(b)))
    elif typ == 10:
        ev["dropped"] = a
    return ev


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("serial", nargs="?", default="", help="iSerial (e.g. YEL0)")
    ap.add_argument("--json", action="store_true", help="one JSON object per line")
    args = ap.parse_args()

    dev, ser = find_dev(args.serial or None)
    if dev is None:
        print("No composite USBasp (diag EP2) found", file=sys.stderr)
        return 1

    if not args.json:
        print(
            f"USBasp NG Diagnostics  serial={ser!r} bcdDevice={dev.bcdDevice:04x}",
            flush=True,
        )
        print("schema: USBASP-NG DIAG v1 (lab monitor)", flush=True)

    try:
        if dev.is_kernel_driver_active(IF_MONITOR):
            dev.detach_kernel_driver(IF_MONITOR)
    except (usb.core.USBError, NotImplementedError):
        pass
    usb.util.claim_interface(dev, IF_MONITOR)

    try:
        while True:
            try:
                data = bytes(dev.read(EP2_IN, 8, timeout=1000))
            except usb.core.USBTimeoutError:
                continue
            except usb.core.USBError as e:
                print(f"USB error: {e}", file=sys.stderr)
                break
            if len(data) < 8 or data[0] == 0:
                continue
            if args.json:
                print(json.dumps(event_dict(data), separators=(",", ":")), flush=True)
            else:
                wall = time.strftime("%H:%M:%S")
                print(f"[{wall}] {_trace.decode_frame(data)}", flush=True)
    except KeyboardInterrupt:
        if not args.json:
            print("\nstopped", flush=True)
    finally:
        usb.util.release_interface(dev, IF_MONITOR)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
