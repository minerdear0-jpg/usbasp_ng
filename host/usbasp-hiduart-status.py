#!/usr/bin/env python3
"""HIDUART status via hidraw — does not detach usbhid, avrdude keeps IF0.

Usage: usbasp-hiduart-status.py [YEL0]
"""
from __future__ import annotations
import os
import select
import sys
from pathlib import Path

import usb.core
import usb.util

VID, PID = 0x16C0, 0x05DC
UART_STATE_ENABLED = 16


def usb_dev(want: str):
    dev = usb.core.find(idVendor=VID, idProduct=PID)
    if dev is None:
        print("No USBasp 16c0:05dc", file=sys.stderr)
        return None
    ser = usb.util.get_string(dev, dev.iSerialNumber) or ""
    if want and ser != want:
        print(f"serial={ser!r} expected {want}", file=sys.stderr)
        return None
    return dev, ser


def hidraw_nodes(dev, serial: str) -> list[Path]:
    found: list[Path] = []
    hidraw = Path("/sys/class/hidraw")
    if not hidraw.is_dir():
        return found
    want_id = f"0003:{dev.idVendor:08X}:{dev.idProduct:08X}"
    for node in sorted(hidraw.iterdir(), key=lambda p: p.name):
        uevent = node / "device" / "uevent"
        if not uevent.is_file():
            continue
        fields = {}
        try:
            for line in uevent.read_text().splitlines():
                if "=" in line:
                    k, v = line.split("=", 1)
                    fields[k] = v
        except OSError:
            continue
        if fields.get("HID_ID", "").upper() != want_id:
            continue
        if serial and fields.get("HID_UNIQ", "") not in ("", serial):
            continue
        found.append(Path("/dev") / node.name)
    return found


def read_hidraw(path: Path) -> bytes | None:
    fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
    try:
        r, _, _ = select.select([fd], [], [], 0.3)
        if not r:
            return None
        return os.read(fd, 8)
    except OSError as e:
        print(f"{path}: {e}", file=sys.stderr)
        return None
    finally:
        os.close(fd)


def main() -> int:
    want = sys.argv[1] if len(sys.argv) > 1 else "YEL0"
    got = usb_dev(want)
    if got is None:
        return 1
    dev, ser = got
    print(f"serial={ser!r} bcdUSB={dev.bcdUSB:04x} bcdDevice={dev.bcdDevice:04x}")
    cfg = dev.get_active_configuration()
    print(f"interfaces={cfg.bNumInterfaces}")
    for intf in cfg:
        k = dev.is_kernel_driver_active(intf.bInterfaceNumber)
        print(
            f"  IF{intf.bInterfaceNumber} class=0x{intf.bInterfaceClass:02x} "
            f"eps={intf.bNumEndpoints} kernel={k}"
        )
    nodes = hidraw_nodes(dev, ser)
    if not nodes:
        print("no hidraw for this VID/PID", file=sys.stderr)
        return 1
    for path in nodes:
        raw = read_hidraw(path)
        if raw is None:
            print(f"{path}: no report yet")
            continue
        st = raw[7] if len(raw) == 8 else 0
        uart = "on" if (st & UART_STATE_ENABLED) else "off"
        print(f"{path}: {raw.hex()}  byte7={st} uart={uart}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
