#!/usr/bin/env python3
"""USBasp NG host diagnostic. Does not change ISP protocol.

Linux: needs pyusb for `info`. `windows-hints` needs no device.
"""
from __future__ import annotations

import argparse
import json
import sys

VID, PID = 0x16C0, 0x05DC
FUNC_GETCAPABILITIES = 127

WINDOWS_HINTS = """
USBasp NG on Windows (classic)
==============================
Goal: Microsoft WinUSB, no Zadig / libusbK / INF.

1. Flash classic usbasp.hex (bcdDevice 2.02), not HIDUART.
2. Device Manager → WinUSB (Microsoft). Label may say "WinUSB Device".
3. Use avrdude 7+/8.x MSVC or AVRDUDESS. Arduino 1.8.19 ships avrdude 6.3 —
   it cannot open WinUSB (cannot query manufacturer).
4. Fix Arduino: arduino/replace-avrdude.ps1  OR use AVRDUDESS for ISP.
5. Do NOT reinstall libusbK to please old Arduino.

Docs: docs/WINDOWS.md  docs/ARDUINO.md  docs/KNOWN_ISSUES.md
""".strip()


def _profile(bcd_device: int, n_intf: int, has_hid: bool) -> str:
    if n_intf == 1 and not has_hid:
        if bcd_device == 0x0202:
            return "classic (WinUSB metadata)"
        if bcd_device == 0x0200:
            return "classic (pre-WinUSB / Fischl-like)"
        return f"classic-like (bcdDevice={bcd_device:04x})"
    if has_hid or n_intf > 1:
        return "hiduart (composite)"
    return "unknown"


def _load_usb():
    try:
        import usb.core
        import usb.util
    except ImportError as exc:
        raise SystemExit("need pyusb: pip install pyusb") from exc
    return usb.core, usb.util


def _string(usb_util, dev, index: int) -> str:
    if not index:
        return ""
    try:
        return usb_util.get_string(dev, index) or ""
    except Exception:
        return "?"


def _collect_one(usb_core, usb_util, dev) -> dict:
    cfg = None
    try:
        cfg = dev.get_active_configuration()
    except Exception:
        try:
            dev.set_configuration()
            cfg = dev.get_active_configuration()
        except Exception:
            pass

    n_intf = 0
    has_hid = False
    n_ep = 0
    if cfg is not None:
        n_intf = cfg.bNumInterfaces
        for intf in cfg:
            if intf.bInterfaceClass == 3:
                has_hid = True
            n_ep += intf.bNumEndpoints

    info = {
        "vid": dev.idVendor,
        "pid": dev.idProduct,
        "bcdUSB": dev.bcdUSB,
        "bcdDevice": dev.bcdDevice,
        "manufacturer": _string(usb_util, dev, dev.iManufacturer),
        "product": _string(usb_util, dev, dev.iProduct),
        "serial": _string(usb_util, dev, dev.iSerialNumber),
        "interfaces": n_intf,
        "endpoints_excl_ep0": n_ep,
        "hid": has_hid,
        "profile": _profile(dev.bcdDevice, n_intf, has_hid),
        "capabilities": None,
        "tpi": None,
        "mhz3": None,
        "topology_ok": None,
        "error": None,
    }

    claimed = False
    try:
        if cfg is not None:
            intf0 = cfg[(0, 0)]
            try:
                if dev.is_kernel_driver_active(intf0.bInterfaceNumber):
                    try:
                        dev.detach_kernel_driver(intf0.bInterfaceNumber)
                    except Exception:
                        pass
            except Exception:
                pass
            try:
                usb_util.claim_interface(dev, intf0.bInterfaceNumber)
                claimed = True
            except Exception:
                pass
        raw = bytes(dev.ctrl_transfer(0xC0, FUNC_GETCAPABILITIES, 0, 0, 4, timeout=2000))
        packed = raw[0] | (raw[1] << 8) | (raw[2] << 16) | (raw[3] << 24)
        info["capabilities"] = {
            "bytes": [raw[0], raw[1], raw[2], raw[3]],
            "packed": packed,
        }
        info["tpi"] = bool(packed & 0x01)
        info["mhz3"] = bool(packed & (1 << 24))
    except Exception as exc:
        info["error"] = str(exc)
    finally:
        if claimed:
            try:
                usb_util.release_interface(dev, 0)
            except Exception:
                pass
        try:
            usb_util.dispose_resources(dev)
        except Exception:
            pass

    if info["profile"].startswith("classic") and n_intf == 1 and n_ep == 0:
        info["topology_ok"] = "1 IF, EP0 only"
    elif "hiduart" in info["profile"] and has_hid:
        info["topology_ok"] = "composite HID present"

    if info["bcdDevice"] == 0x0202 and info["profile"].startswith("classic"):
        info["windows_expect"] = "WinUSB (Microsoft) without Zadig"
    elif "hiduart" in info["profile"]:
        info["windows_expect"] = "prefer classic for MSVC avrdude; HID uses hidclass"

    return info


def cmd_info(args: argparse.Namespace) -> int:
    usb_core, usb_util = _load_usb()
    devices = list(usb_core.find(find_all=True, idVendor=VID, idProduct=PID))
    if not devices:
        print("No USBasp 16c0:05dc on the bus", file=sys.stderr)
        return 1

    rows = [_collect_one(usb_core, usb_util, d) for d in devices]
    if args.json:
        json.dump(rows if len(rows) > 1 else rows[0], sys.stdout, indent=2)
        sys.stdout.write("\n")
        return 0

    for i, info in enumerate(rows):
        if i:
            print("---")
        print("USBasp NG")
        print(f"  VID:PID       {info['vid']:04x}:{info['pid']:04x}")
        print(f"  bcdUSB        {info['bcdUSB']:04x}")
        print(f"  bcdDevice     {info['bcdDevice']:04x}")
        print(f"  manufacturer  {info['manufacturer']!r}")
        print(f"  product       {info['product']!r}")
        print(f"  serial        {info['serial']!r}")
        print(f"  interfaces    {info['interfaces']}")
        print(f"  endpoints     {info['endpoints_excl_ep0']} (excl. EP0)")
        print(f"  HID present   {info['hid']}")
        print(f"  USB profile   {info['profile']}")
        if info.get("windows_expect"):
            print(f"  Windows       {info['windows_expect']}")
        if info["error"]:
            print(f"  GETCAPABILITIES failed: {info['error']}")
        elif info["capabilities"]:
            b = info["capabilities"]["bytes"]
            print(
                f"  capabilities  {b[0]:02x} {b[1]:02x} {b[2]:02x} {b[3]:02x}"
                f"  (0x{info['capabilities']['packed']:08x})"
            )
            print(f"  TPI           {info['tpi']}")
            print(f"  3MHz          {info['mhz3']}")
        if info["topology_ok"]:
            print(f"  topology      OK ({info['topology_ok']})")
    return 0


def cmd_windows_hints(_: argparse.Namespace) -> int:
    print(WINDOWS_HINTS)
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description="USBasp NG host diagnostics")
    sub = p.add_subparsers(dest="cmd", required=True)

    info = sub.add_parser("info", help="enumerate 16c0:05dc and GETCAPABILITIES")
    info.add_argument("--json", action="store_true", help="machine-readable output")
    info.set_defaults(func=cmd_info)

    hints = sub.add_parser(
        "windows-hints",
        help="print WinUSB / Arduino / avrdude troubleshooting (no USB needed)",
    )
    hints.set_defaults(func=cmd_windows_hints)

    args = p.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
