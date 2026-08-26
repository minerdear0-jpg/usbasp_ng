#!/usr/bin/env python3
"""Classic and HIDUART must share the same MS OS 2.0 vendor bRequest (0x5D)."""
from pathlib import Path
import re

FW = Path(__file__).resolve().parents[2]


def main() -> int:
    classic = (FW / "include" / "usbasp" / "ms_os_20.h").read_text()
    hid = (FW / "src_hid" / "usb_descriptors.h").read_text()
    cm = re.search(r"#define\s+USBASP_MS_OS_VENDOR_CODE\s+(0x[0-9A-Fa-f]+)", classic)
    hm = re.search(r"#define\s+VENDOR_CODE\s+(0x[0-9A-Fa-f]+)", hid)
    assert cm and hm, "missing vendor code defines"
    assert int(cm.group(1), 16) == int(hm.group(1), 16) == 0x5D
    assert "MS_OS_2_0_DESCRIPTOR_INDEX" in classic
    assert "MS_OS_2_0_DESCRIPTOR_INDEX" in hid
    print("ok  ms_os_vendor_code")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
