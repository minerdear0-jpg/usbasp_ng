#!/usr/bin/env python3
"""Classic firmware/src must not mention the HIDUART product."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src"
FORBIDDEN = ("HID", "hiduart", "WCID", "src_hid", "uart.h")


def main() -> int:
    failed = 0
    for path in sorted(SRC.rglob("*")):
        if path.suffix not in {".c", ".h", ".S"}:
            continue
        text = path.read_text(errors="replace")
        for token in FORBIDDEN:
            if token in text:
                print(f"FAIL {path.relative_to(ROOT)} contains {token!r}")
                failed += 1
    proto = ROOT / "include" / "usbasp" / "protocol.h"
    if "SET_REPORT" in proto.read_text():
        print("FAIL protocol.h must not define HID SET_REPORT")
        failed += 1
    if failed:
        return 1
    print("ok  classic src isolated from HIDUART")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
