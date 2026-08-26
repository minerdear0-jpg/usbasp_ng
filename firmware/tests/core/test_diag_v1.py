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
    assert d["DIAG_MEMOP"] == 12
    assert d["DIAG_CAPS"] == 13
    assert d["DIAG_TRACE_BEGIN"] == 14
    assert d["DIAG_TRACE_END"] == 15
    assert d["DIAG_ISP_PINS"] == 16
    assert d["DIAG_PINS_AFTER_DISC"] == 0x01
    assert d["DIAG_MEM_FLASH"] == 0
    assert d["DIAG_EP_START"] == 0x01
    assert d["DIAG_EP_CONT"] == 0x02
    assert d["DIAG_EP_END"] == 0x04
    assert d["DIAG_EP_RESULT_OK"] == 0x10
    assert d["DIAG_EP_RESULT_FAIL"] == 0x20
    assert d["DIAG_CAP_TRANSACTION"] == 0x02
    assert d["DIAG_CAP_SNAPSHOT"] == 0x04
    assert d["DIAG_CAP_TIMESTAMP"] == 0x20
    assert d["DIAG_CAP_TRACE"] == 0x08

    events = EVENTS.read_text()
    assert "DIAG_FCAP_TIMESTAMP" in events
    assert "DIAG_FCAP_TRACE" in events
    assert "DIAG_FCAP_TRIGGER" in events
    assert "DIAG_FCAP_PRETRIGGER" in events
    assert "BOARD_CAP_PHYSICAL_CAPTURE" in events
    assert "BOARD_CAP_SCK_JUMPER" in events

    ring_h = (FW / "include" / "diag" / "diag_ring.h").read_text()
    assert "USBASP_DIAG_TRACE_SLOTS" in ring_h
    assert "USBASP_DIAG_POST_CAPTURE_EVENTS" in ring_h
    cfg_in = (FW / "cmake" / "usbasp_config.h.in").read_text()
    assert "USBASP_DIAG_TRACE_SLOTS" in cfg_in
    board328 = (FW / "boards" / "usbasp-hiduart-atmega328p.cmake").read_text()
    assert "USBASP_DIAG_TRACE_SLOTS 128" in board328
    assert "diag_trace_push" in ring_h
    assert "DIAG_CAP_STATE_POST" in ring_h
    assert "DIAG_CAP_STATE_FROZEN" in ring_h
    assert "trigger_kind" in ring_h

    trig_h = (FW / "include" / "diag" / "diag_trigger.h").read_text()
    assert "DIAG_TRIG_ENABLEPROG_FAIL" in trig_h
    assert "diag_trigger_match" in trig_h

    diag_c = DIAG_C.read_text()
    assert "diag_emit_enableprog" in diag_c
    assert "DIAG_FCAP_TRIGGER" in diag_c
    assert "DIAG_FCAP_PRETRIGGER" in diag_c
    assert "DIAG_TRACE_BEGIN" in diag_c
    assert "DIAG_TRACE_END" in diag_c
    assert "diag_emit_isp_pins" in diag_c
    assert "DIAG_ISP_PINS" in diag_c
    assert "DIAG_MEMOP_PAGE_STRIDE" in diag_c
    assert "0x1E00" in diag_c
    assert "diag_trace_arm" in diag_c
    assert "meta.trigger_kind" in diag_c or "trigger_kind" in diag_c

    trace_c = (FW / "src" / "diag" / "diag_trace.c").read_text()
    assert "diag_trigger_on_event" in trace_c
    assert "DIAG_CAP_STATE_POST" in trace_c
    assert "DIAG_CAP_STATE_FROZEN" in trace_c
    assert "cli()" not in trace_c

    trig_c = (FW / "src" / "diag" / "diag_trigger.c").read_text()
    assert "DIAG_TRIG_ENABLEPROG_FAIL" in trig_c
    assert "DIAG_EP_RESULT_FAIL" in trig_c

    clock_h = (FW / "include" / "diag" / "diag_clock.h").read_text()
    assert "diag_tick_t" in clock_h
    assert "diag_now_wire16" in clock_h
    clock_c = (FW / "src" / "diag" / "diag_clock.c").read_text()
    assert "TOIE1" in clock_c
    assert "CS11" in clock_c
    assert "SIGNAL(" not in clock_c
    assert "ISR(" not in clock_c  # no overflow ISR vector

    isp = ISP.read_text()
    assert "diag_report_enableprog" in isp
    assert "diag_note_enableprog_try" in isp
    assert "diag_emit_sck_config" in isp
    assert "if (tries == 1)" in isp
    sw = isp[isp.index("ispTransmit_sw") : isp.index("ispTransmit_hw")]
    assert "diag_" not in sw

    vendor = VENDOR.read_text()
    assert "diag_on_connect();" in vendor
    assert "diag_memop_begin" in vendor
    assert "diag_memop_end" in vendor
    assert "diag_memop_page(prog_address" in vendor
    assert "flush_fail" in vendor
    assert "DIAG_MEM_READFLASH" in vendor

    print("ok  diag_v1_golden")
    return 0


if __name__ == "__main__":
    sys.exit(main())
