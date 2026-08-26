#!/usr/bin/env python3
"""boards/*/fuses.txt must match USBASP_HFUSE/LFUSE in the sibling .cmake."""
from pathlib import Path
import re

BOARDS = Path(__file__).resolve().parents[2] / "boards"


def main() -> int:
    failed = 0
    for cmake in sorted(BOARDS.glob("*.cmake")):
        name = cmake.stem
        fuses = BOARDS / name / "fuses.txt"
        text = cmake.read_text()
        hm = re.search(r"set\(USBASP_HFUSE\s+(0x[0-9a-fA-F]+)\)", text)
        lm = re.search(r"set\(USBASP_LFUSE\s+(0x[0-9a-fA-F]+)\)", text)
        if not hm or not lm:
            print(f"FAIL {name}: missing HFUSE/LFUSE in cmake")
            failed += 1
            continue
        if not fuses.is_file():
            print(f"FAIL {name}: missing {fuses.relative_to(BOARDS.parent)}")
            failed += 1
            continue
        body = fuses.read_text()
        if f"hfuse={hm.group(1).lower()}" not in body.lower():
            print(f"FAIL {name}: hfuse mismatch")
            failed += 1
        if f"lfuse={lm.group(1).lower()}" not in body.lower():
            print(f"FAIL {name}: lfuse mismatch")
            failed += 1
    if failed:
        return 1
    print("ok  board_fuses")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
