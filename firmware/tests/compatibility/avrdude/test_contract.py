#!/usr/bin/env python3
"""Contract tests for USBasp FUNC IDs, capabilities, and avrdude SETUP packing."""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
PROTOCOL_H = ROOT / "include" / "usbasp" / "protocol.h"
SPEC = Path(__file__).resolve().parent / "spec.yaml"

EXPECTED_FUNCS = {
    "USBASP_FUNC_CONNECT": 1,
    "USBASP_FUNC_DISCONNECT": 2,
    "USBASP_FUNC_TRANSMIT": 3,
    "USBASP_FUNC_READFLASH": 4,
    "USBASP_FUNC_ENABLEPROG": 5,
    "USBASP_FUNC_WRITEFLASH": 6,
    "USBASP_FUNC_READEEPROM": 7,
    "USBASP_FUNC_WRITEEEPROM": 8,
    "USBASP_FUNC_SETLONGADDRESS": 9,
    "USBASP_FUNC_SETISPSCK": 10,
    "USBASP_FUNC_TPI_CONNECT": 11,
    "USBASP_FUNC_TPI_DISCONNECT": 12,
    "USBASP_FUNC_TPI_RAWREAD": 13,
    "USBASP_FUNC_TPI_RAWWRITE": 14,
    "USBASP_FUNC_TPI_READBLOCK": 15,
    "USBASP_FUNC_TPI_WRITEBLOCK": 16,
    "USBASP_FUNC_GETCAPABILITIES": 127,
}


def parse_defines(text: str) -> dict:
    defs = {}
    for m in re.finditer(r"#define\s+(USBASP_FUNC_\w+)\s+(\d+)", text):
        defs[m.group(1)] = int(m.group(2))
    return defs


def avrdude_pack(send0, send1, send2, send3):
    wvalue = ((send1 << 8) | send0) & 0xFFFF
    windex = ((send3 << 8) | send2) & 0xFFFF
    return wvalue, windex


def setup_data(bm, brequest, send, wlength, receive=True):
    wvalue, windex = avrdude_pack(*send)
    data = [0] * 8
    data[0] = bm
    data[1] = brequest
    data[2] = wvalue & 0xFF
    data[3] = (wvalue >> 8) & 0xFF
    data[4] = windex & 0xFF
    data[5] = (windex >> 8) & 0xFF
    data[6] = wlength & 0xFF
    data[7] = (wlength >> 8) & 0xFF
    assert data[0] == (0xC0 if receive else 0x40)
    return data


def test_func_ids():
    defs = parse_defines(PROTOCOL_H.read_text())
    for name, num in EXPECTED_FUNCS.items():
        assert defs[name] == num, f"{name}: {defs[name]} != {num}"


def test_identity():
    text = PROTOCOL_H.read_text()
    assert "0x16c0" in text
    assert "0x05dc" in text


def test_capabilities_layout():
    text = PROTOCOL_H.read_text()
    assert re.search(r"#define\s+USBASP_CAP_TPI\s+0x01", text)
    classic = [0x01, 0x00, 0x00, 0x01]
    caps = classic[0] | (classic[1] << 8) | (classic[2] << 16) | (classic[3] << 24)
    assert caps == (0x01 | (1 << 24))


def test_setlongaddress_le():
    addr = 0x000ABCDE
    send = [addr & 0xFF, (addr >> 8) & 0xFF, (addr >> 16) & 0xFF, (addr >> 24) & 0xFF]
    data = setup_data(0xC0, 9, send, 4)
    le = data[2] | (data[3] << 8) | (data[4] << 16) | (data[5] << 24)
    assert le == addr


def test_writeflash_meg128_pagesize():
    address = 0x100
    page_size = 0x100
    blockflags = 1
    send = [
        address & 0xFF,
        (address >> 8) & 0xFF,
        page_size & 0xFF,
        (blockflags & 0x0F) + ((page_size & 0xF00) >> 4),
    ]
    data = setup_data(0x40, 6, send, 200, receive=False)
    pagesize = data[4] + ((data[5] & 0xF0) << 4)
    flags = data[5] & 0x0F
    assert pagesize == 0x100
    assert flags == 1
    # Firmware must keep pagecounter as uint: (uchar)0x100 == 0 would skip flushes.


def test_sck_ids_match_avrdude():
    text = PROTOCOL_H.read_text()
    expected = {
        "USBASP_ISP_SCK_AUTO": 0,
        "USBASP_ISP_SCK_0_5": 1,
        "USBASP_ISP_SCK_8": 5,
        "USBASP_ISP_SCK_1500": 12,
        "USBASP_ISP_SCK_3000": 13,
    }
    for name, num in expected.items():
        m = re.search(rf"#define\s+{name}\s+(\d+)", text)
        assert m, name
        assert int(m.group(1)) == num


def test_tpi_connect_delay_le16():
    """avrdude: temp[0]=dly, temp[1]=dly>>8 → wValue LE."""
    dly = 0x123
    send = [dly & 0xFF, (dly >> 8) & 0xFF, 0, 0]
    data = setup_data(0xC0, 11, send, 4)
    assert (data[2] | (data[3] << 8)) == dly


def test_forbidden_func_gap():
    text = PROTOCOL_H.read_text()
    assert "USBASP_FUNC_UART" not in text


def test_spec_func_ids():
    text = SPEC.read_text()
    for name, num in EXPECTED_FUNCS.items():
        short = name.replace("USBASP_FUNC_", "")
        if short == "GETCAPABILITIES":
            m = re.search(r"^getcapabilities:\s*\n(?:[ \t]+.+\n)*?[ \t]+func:\s*(\d+)", text, re.M)
        else:
            m = re.search(rf"^[ \t]+{short}:\s*\n[ \t]+id:\s*(\d+)", text, re.M)
        assert m, f"spec.yaml missing {short}"
        assert int(m.group(1)) == num, f"spec {short}: {m.group(1)} != {num}"


def main():
    tests = [
        test_func_ids,
        test_identity,
        test_capabilities_layout,
        test_setlongaddress_le,
        test_writeflash_meg128_pagesize,
        test_sck_ids_match_avrdude,
        test_tpi_connect_delay_le16,
        test_forbidden_func_gap,
        test_spec_func_ids,
    ]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"ok  {t.__name__}")
        except Exception as e:
            failed += 1
            print(f"FAIL {t.__name__}: {e}")
    if not SPEC.exists():
        failed += 1
        print("FAIL spec.yaml missing")
    else:
        print("ok  spec.yaml present")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
