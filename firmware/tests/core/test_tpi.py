#!/usr/bin/env python3
"""TPI opcodes and host packing vs firmware (no tiny10 required)."""
from pathlib import Path
import re

FW = Path(__file__).resolve().parents[2]
DEFS = (FW / "include" / "usbasp" / "tpi_defs.h").read_text()
ASM = (FW / "src" / "tpi.S").read_text(encoding="utf-8", errors="replace")
VENDOR = (FW / "src" / "vendor_isp.c").read_text()
CMAKE = (FW / "CMakeLists.txt").read_text()


def cpp_define(name: str) -> str:
    m = re.search(rf"#define\s+{re.escape(name)}\s+(.+)", DEFS)
    assert m, name
    return m.group(1).strip()


def tpi_op_sin(a: int) -> int:
    return 0x10 | (((a) << 1) & 0x60) | ((a) & 0x0F)


def tpi_op_sout(a: int) -> int:
    return 0x90 | (((a) << 1) & 0x60) | ((a) & 0x0F)


def test_nvm_opcodes_match_atmel():
    assert int(cpp_define("TPI_OP_SLD_INC"), 0) == 0x24
    assert int(cpp_define("TPI_OP_SST_INC"), 0) == 0x64
    assert int(cpp_define("TPI_OP_SKEY"), 0) == 0xE0
    assert int(cpp_define("NVMCSR"), 0) == 0x32
    assert int(cpp_define("NVMCMD"), 0) == 0x33
    assert int(cpp_define("NVMCMD_WORD_WRITE"), 0) == 0x1D
    assert int(cpp_define("NVMCSR_BSY"), 0) == 0x80
    assert tpi_op_sout(0x33) == 0xF3
    assert tpi_op_sin(0x32) == 0x72
    assert (0x68 | 0) == 0x68  # SSTPR(0)
    assert (0x68 | 1) == 0x69  # SSTPR(1)


def test_asm_pins_are_isp_sck_mosi():
    assert re.search(r"#define\s+TPI_CLK_BIT\s+5", ASM)
    assert re.search(r"#define\s+TPI_DATAOUT_BIT\s+3", ASM)
    assert "TPI_WITH_OPTO" not in CMAKE
    assert "-DTPI_WITH_OPTO" not in CMAKE


def test_asm_uses_block_opcodes():
    for token in (
        "TPI_OP_SLD_INC",
        "TPI_OP_SSTPR(0)",
        "TPI_OP_SSTPR(1)",
        "TPI_OP_SST_INC",
        "TPI_OP_SOUT(NVMCMD)",
        "TPI_OP_SIN(NVMCSR)",
        "NVMCMD_WORD_WRITE",
        "NVMCSR_BSY",
    ):
        assert token in ASM, token
    assert re.search(r"ldi\s+r21,\s*32", ASM)


def test_vendor_connect_delay_and_init():
    assert "tpi_dly_cnt = usbasp_read_le16(&data[2])" in VENDOR
    assert "tpi_init()" in VENDOR
    assert "USBASP_FUNC_TPI_CONNECT" in VENDOR
    conn = VENDOR.split("USBASP_FUNC_TPI_CONNECT", 1)[1].split("USBASP_FUNC_TPI_DISCONNECT", 1)[0]
    assert "ISP_RST" in conn
    assert "clockWait(3)" in conn
    assert "clockWait(16)" in conn


def test_vendor_disconnect_clears_tpisr():
    disc = VENDOR.split("USBASP_FUNC_TPI_DISCONNECT", 1)[1].split("USBASP_FUNC_TPI_RAWREAD", 1)[0]
    assert "TPI_OP_SSTCS(TPISR)" in disc
    assert "tpi_send_byte(0)" in disc


def test_vendor_raw_and_block_packing():
    assert "tpi_send_byte(data[2])" in VENDOR
    assert "replyBuffer[0] = tpi_recv_byte()" in VENDOR
    blk = VENDOR.split("USBASP_FUNC_TPI_READBLOCK", 1)[1]
    assert "prog_address = usbasp_read_le16(&data[2])" in blk
    assert "prog_nbytes = usbasp_read_le16(&data[6])" in blk
    assert "PROG_STATE_TPI_READ" in blk
    assert "PROG_STATE_TPI_WRITE" in blk


def main() -> int:
    tests = [
        test_nvm_opcodes_match_atmel,
        test_asm_pins_are_isp_sck_mosi,
        test_asm_uses_block_opcodes,
        test_vendor_connect_delay_and_init,
        test_vendor_disconnect_clears_tpisr,
        test_vendor_raw_and_block_packing,
    ]
    for t in tests:
        t()
        print(f"ok  {t.__name__}")
    print("ok  tpi")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
