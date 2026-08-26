#!/usr/bin/env python3
"""prog_session: CONNECT/DISCONNECT reset + nbytes==0 must not arm data stage."""
from pathlib import Path

FW = Path(__file__).resolve().parents[2]
VENDOR = (FW / "src" / "vendor_isp.c").read_text()
PROG_H = (FW / "include" / "usbasp" / "prog_state.h").read_text()
ISP_H = (FW / "include" / "usbasp" / "isp.h").read_text()


def test_prog_reset_on_connect_disconnect():
    assert "void prog_reset_state(void)" in PROG_H
    assert "prog_reset_state();" in VENDOR
    # CONNECT resets before ispConnect; DISCONNECT after ispDisconnect
    conn = VENDOR[VENDOR.index("case USBASP_FUNC_CONNECT") : VENDOR.index("case USBASP_FUNC_DISCONNECT")]
    disc = VENDOR[VENDOR.index("case USBASP_FUNC_DISCONNECT") : VENDOR.index("case USBASP_FUNC_TRANSMIT")]
    assert "prog_reset_state();" in conn
    assert conn.index("prog_reset_state();") < conn.index("ispConnect();")
    assert "ispDisconnect();" in disc
    assert disc.index("ispDisconnect();") < disc.index("prog_reset_state();")


def test_nbytes_zero_guard():
    assert "prog_begin_transfer" in VENDOR
    assert "if (nbytes == 0)" in VENDOR
    for name in (
        "USBASP_FUNC_READFLASH",
        "USBASP_FUNC_READEEPROM",
        "USBASP_FUNC_WRITEFLASH",
        "USBASP_FUNC_WRITEEEPROM",
        "USBASP_FUNC_TPI_READBLOCK",
        "USBASP_FUNC_TPI_WRITEBLOCK",
    ):
        assert name in VENDOR


def test_wire_address_types():
    assert "uint32_t prog_address" in PROG_H
    assert "uint16_t prog_nbytes" in PROG_H
    assert "uint16_t prog_pagesize" in PROG_H
    assert "ispReadEEPROM(uint16_t" in ISP_H
    assert "ispWriteEEPROM(uint16_t" in ISP_H
    assert "ispReadFlash(uint32_t" in ISP_H


def test_tpi_read_decrements_nbytes():
    """TPI READ must mirror FLASH/EEPROM: nbytes + IDLE completion."""
    start = VENDOR.index("PROG_STATE_TPI_READ")
    # First occurrence in usbasp_isp_read
    block = VENDOR[VENDOR.index("if (prog_state == PROG_STATE_TPI_READ)") :]
    block = block[: block.index("board_led_isp_activity")]
    assert "prog_nbytes" in block
    assert "PROG_STATE_IDLE" in block


def main() -> int:
    test_prog_reset_on_connect_disconnect()
    test_nbytes_zero_guard()
    test_wire_address_types()
    test_tpi_read_decrements_nbytes()
    print("ok  prog_session")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
