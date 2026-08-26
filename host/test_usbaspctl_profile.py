#!/usr/bin/env python3
"""Unit tests for usbaspctl profile heuristic (no USB hardware)."""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("usbaspctl", ROOT / "usbaspctl.py")
mod = importlib.util.module_from_spec(spec)
# Avoid hard fail if pyusb missing: load only if import works inside module.
try:
    spec.loader.exec_module(mod)
except SystemExit:
    # usbaspctl exits on ImportError at import time — stub pyusb first
    import sys
    import types

    usb = types.ModuleType("usb")
    usb.core = types.ModuleType("usb.core")
    usb.util = types.ModuleType("usb.util")
    sys.modules["usb"] = usb
    sys.modules["usb.core"] = usb.core
    sys.modules["usb.util"] = usb.util
    spec.loader.exec_module(mod)


def main() -> int:
    p = mod._profile
    assert "WinUSB" in p(0x0202, 1, False)
    assert "pre-WinUSB" in p(0x0200, 1, False)
    assert "hiduart" in p(0x0201, 3, True)
    assert "hiduart" in p(0x0201, 3, False)
    assert "classic-like" in p(0x0203, 1, False)
    print("ok  usbaspctl_profile")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
