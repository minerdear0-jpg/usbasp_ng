#!/usr/bin/env python3
"""reference/ is an immutable snapshot. Firmware compile must not use it."""
from pathlib import Path

FW = Path(__file__).resolve().parents[2]
SRC_DIRS = (FW / "src", FW / "src_hid", FW / "include", FW / "cmake", FW / "boards")


def main() -> int:
    failed = 0
    cmake = (FW / "CMakeLists.txt").read_text()
    if "reference" in cmake:
        print("FAIL firmware/CMakeLists.txt mentions reference/")
        failed += 1
    for d in SRC_DIRS:
        if not d.exists():
            continue
        for path in sorted(d.rglob("*")):
            if path.suffix not in {".c", ".h", ".S", ".cmake", ".in"}:
                continue
            for i, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
                s = line.strip()
                if not s.startswith("#include") and "add_subdirectory" not in s:
                    continue
                if "reference" in s:
                    print(f"FAIL {path.relative_to(FW)}:{i}: {s}")
                    failed += 1
    if failed:
        return 1
    print("ok  reference/ is not a firmware compile input")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
