#!/usr/bin/env python3
"""Unit tests for usbaspctl helpers (no USB hardware)."""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("usbaspctl", ROOT / "usbaspctl.py")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


def main() -> int:
    p = mod._profile
    assert "WinUSB" in p(0x0202, 1, False)
    assert "WinUSB" in p(0x0203, 1, False)
    assert "pre-WinUSB" in p(0x0200, 1, False)
    assert "hiduart" in p(0x0201, 3, True)
    assert "classic-like" in p(0x0204, 1, False)
    assert "Zadig" in mod.WINDOWS_HINTS
    assert "6.3" in mod.WINDOWS_HINTS
    import io
    from contextlib import redirect_stdout

    buf = io.StringIO()
    with redirect_stdout(buf):
        assert mod.cmd_windows_hints(type("A", (), {})()) == 0
    assert "WinUSB" in buf.getvalue()
    print("ok  usbaspctl_profile")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
