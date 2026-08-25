#!/usr/bin/env python3
"""Intel HEX for HIDUART EEPROM serial matches V-USB 4-char string at address 0."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "host" / "usbasp-serial-eep.py"


def test_yel0():
    out = subprocess.check_output(["python3", str(SCRIPT), "YEL0"], text=True)
    # 10 03 'Y' 00 'E' 00 'L' 00 '0' 00
    assert "0A03590045004C003000" in out.replace("\n", "").upper()


def main() -> int:
    test_yel0()
    print("ok  serial eep YEL0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
