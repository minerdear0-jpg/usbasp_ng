#!/usr/bin/env python3
"""USBasp NG host diagnostic (Linux). Does not change ISP protocol."""
from __future__ import annotations

import argparse
import sys

try:
    import usb.core
    import usb.util
except ImportError:
    print("need pyusb: pip install pyusb", file=sys.stderr)
    sys.exit(1)

VID, PID = 0x16C0, 0x05DC
FUNC_GETCAPABILITIES = 127


def _string(dev, index: int) -> str:
    if not index:
        return ""
    try:
        return usb.util.get_string(dev, index) or ""
    except Exception:
        return "?"


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


def cmd_info(_: argparse.Namespace) -> int:
    devices = list(usb.core.find(find_all=True, idVendor=VID, idProduct=PID))
    if not devices:
        print("No USBasp 16c0:05dc on the bus", file=sys.stderr)
        return 1

    for i, dev in enumerate(devices):
        if i:
            print("---")
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

        mfg = _string(dev, dev.iManufacturer)
        prod = _string(dev, dev.iProduct)
        ser = _string(dev, dev.iSerialNumber)
        profile = _profile(dev.bcdDevice, n_intf, has_hid)

        print("USBasp NG")
        print(f"  VID:PID       {dev.idVendor:04x}:{dev.idProduct:04x}")
        print(f"  bcdUSB        {dev.bcdUSB:04x}")
        print(f"  bcdDevice     {dev.bcdDevice:04x}")
        print(f"  manufacturer  {mfg!r}")
        print(f"  product       {prod!r}")
        print(f"  serial        {ser!r}")
        print(f"  interfaces    {n_intf}")
        print(f"  endpoints     {n_ep} (excl. EP0)")
        print(f"  HID present   {has_hid}")
        print(f"  USB profile   {profile}")

        caps = None
        claimed = False
        try:
            if cfg is not None:
                intf0 = cfg[(0, 0)]
                if dev.is_kernel_driver_active(intf0.bInterfaceNumber):
                    # WinUSB/classic: often no kernel driver. HIDUART IF0 may be unbound.
                    try:
                        dev.detach_kernel_driver(intf0.bInterfaceNumber)
                    except Exception:
                        pass
                try:
                    usb.util.claim_interface(dev, intf0.bInterfaceNumber)
                    claimed = True
                except Exception:
                    pass
            raw = dev.ctrl_transfer(0xC0, FUNC_GETCAPABILITIES, 0, 0, 4, timeout=2000)
            caps = bytes(raw)
        except Exception as exc:
            print(f"  GETCAPABILITIES failed: {exc}")
        finally:
            if claimed:
                try:
                    usb.util.release_interface(dev, 0)
                except Exception:
                    pass
            try:
                usb.util.dispose_resources(dev)
            except Exception:
                pass

        if caps is not None and len(caps) >= 4:
            packed = caps[0] | (caps[1] << 8) | (caps[2] << 16) | (caps[3] << 24)
            print(f"  capabilities  {caps[0]:02x} {caps[1]:02x} {caps[2]:02x} {caps[3]:02x}  (0x{packed:08x})")
            print(f"  TPI           {bool(packed & 0x01)}")
            print(f"  3MHz          {bool(packed & (1 << 24))}")

        if profile.startswith("classic") and n_intf == 1 and n_ep == 0:
            print("  topology      OK (1 IF, EP0 only)")
        if "hiduart" in profile and has_hid:
            print("  topology      composite OK (HID present)")

    return 0


def main() -> int:
    p = argparse.ArgumentParser(description="USBasp NG host diagnostics")
    sub = p.add_subparsers(dest="cmd", required=True)
    info = sub.add_parser("info", help="enumerate 16c0:05dc and GETCAPABILITIES")
    info.set_defaults(func=cmd_info)
    args = p.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
