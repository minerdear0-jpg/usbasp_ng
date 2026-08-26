#!/usr/bin/env python3
"""USBASP_HAS_TPI must be wired through board cmake → usbasp_config.h."""
from pathlib import Path
import re

FW = Path(__file__).resolve().parents[2]


def main() -> int:
    cfg = (FW / "cmake" / "usbasp_config.h.in").read_text()
    assert "USBASP_HAS_TPI" in cfg
    vendor = (FW / "src" / "vendor_isp.c").read_text()
    assert "#if USBASP_HAS_TPI" in vendor
    for cmake in (FW / "boards").glob("*.cmake"):
        text = cmake.read_text()
        assert re.search(r"set\(USBASP_HAS_TPI\s+[01]\)", text), cmake.name
    print("ok  has_tpi_board_gate")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
