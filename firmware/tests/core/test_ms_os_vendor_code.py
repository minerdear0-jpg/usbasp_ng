#!/usr/bin/env python3
"""Classic and HIDUART must share USBASP_MS_OS_VENDOR_CODE (0x5D)."""
from pathlib import Path
import re

FW = Path(__file__).resolve().parents[2]


def main() -> int:
    shared = (FW / "include" / "usbasp" / "ms_os_vendor.h").read_text()
    classic = (FW / "include" / "usbasp" / "ms_os_20.h").read_text()
    hid = (FW / "src_hid" / "usb_descriptors.h").read_text()
    m = re.search(r"#define\s+USBASP_MS_OS_VENDOR_CODE\s+(0x[0-9A-Fa-f]+)", shared)
    assert m and int(m.group(1), 16) == 0x5D
    assert '#include "usbasp/ms_os_vendor.h"' in classic
    assert '#include "usbasp/ms_os_vendor.h"' in hid
    assert "VENDOR_CODE USBASP_MS_OS_VENDOR_CODE" in hid
    assert re.search(r"#define\s+VENDOR_CODE\s+0x", hid) is None
    print("ok  ms_os_vendor_code")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
