#!/usr/bin/env python3
"""requested_sck is written only from SETISPSCK (host), never from jumper/autoslow."""
from pathlib import Path
import re

SRC = Path(__file__).resolve().parents[2] / "src"


def main() -> int:
    assigns = []
    for path in sorted(SRC.rglob("*.c")):
        text = path.read_text()
        for i, line in enumerate(text.splitlines(), 1):
            if re.search(r"\brequested_sck\s*=", line):
                assigns.append((path.name, i, line.strip()))
    # Definition + SETISPSCK store only
    names = [a[0] for a in assigns]
    assert names.count("vendor_isp.c") == 2, assigns  # init + SETISPSCK
    assert all(n == "vendor_isp.c" for n in names), assigns
    assert any("USBASP_ISP_SCK_AUTO" in a[2] for a in assigns)
    assert any("data[2]" in a[2] for a in assigns)
    print("ok  requested_sck_writes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
