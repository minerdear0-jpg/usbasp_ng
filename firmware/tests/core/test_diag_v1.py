#!/usr/bin/env python3
"""Golden constants for USBASP-NG DIAG v1 (lifecycle + ENABLEPROG/snapshot)."""
from pathlib import Path
import re
import sys

FW = Path(__file__).resolve().parents[2]
EVENTS = FW / "include" / "diag" / "diag_events.h"
DIAG_C = FW / "src" / "diag" / "diag.c"
VENDOR = FW / "src" / "vendor_isp.c"
ISP = FW / "src" / "isp.c"


def defines(path: Path) -> dict[str, int]:
    out: dict[str, int] = {}
    for line in path.read_text().splitlines():
        m = re.match(r"#define\s+(DIAG_\w+)\s+(0x[0-9A-Fa-f]+|\d+)", line)
        if m:
            out[m.group(1)] = int(m.group(2), 0)
    return out


def main() -> int:
    d = defines(EVENTS)
    assert d["DIAG_SCHEMA_V1"] == 1
    assert d["DIAG_HELLO"] == 1
    assert d["DIAG_ENABLEPROG"] == 6
    assert d["DIAG_FAULT_SNAPSHOT"] == 9
    assert d["DIAG_EP_START"] == 0x01
    assert d["DIAG_EP_CONT"] == 0x02
    assert d["DIAG_EP_END"] == 0x04
    assert d["DIAG_EP_RESULT_OK"] == 0x10
    assert d["DIAG_EP_RESULT_FAIL"] == 0x20
    assert d["DIAG_CAP_TRANSACTION"] == 0x02
    assert d["DIAG_CAP_SNAPSHOT"] == 0x04

    diag_c = DIAG_C.read_text()
    assert "diag_emit_enableprog" in diag_c
    assert "diag_publish_snapshot" in diag_c
    assert "diag_report_enableprog" in diag_c
    assert "memcpy(&diag_fault_snapshot" in diag_c
    assert "DIAG_CAP_SESSION | DIAG_CAP_TRANSACTION | DIAG_CAP_SNAPSHOT" in diag_c
    # Compact 4-frame FAULT_SNAPSHOT packing
    assert "sck_req << 4" in diag_c
    assert diag_c.count("DIAG_FAULT_SNAPSHOT") == 4

    assert "diag_note_enableprog_try" in diag_c
    assert "DIAG_ERR_EP_AVR" in (FW / "include" / "diag" / "diag_events.h").read_text()
    assert "DIAG_RING_SIZE 32" in (FW / "include" / "diag" / "diag_ring.h").read_text()

    isp = ISP.read_text()
    assert "diag_report_enableprog" in isp
    assert "diag_note_enableprog_try" in isp
    assert "diag_emit_sck_config" in isp
    assert "if (tries == 1)" in isp
    sw = isp[isp.index("ispTransmit_sw") : isp.index("ispTransmit_hw")]
    assert "diag_" not in sw

    vendor = VENDOR.read_text()
    assert "diag_on_connect();" in vendor

    print("ok  diag_v1_golden")
    return 0


if __name__ == "__main__":
    sys.exit(main())
