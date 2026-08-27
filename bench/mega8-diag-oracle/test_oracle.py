#!/usr/bin/env python3
"""Host-side checks for the mega8 diag oracle (no hardware)."""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from oracle_lib import (  # noqa: E402
    CANARY_ADDR,
    CANARY_LEN,
    FLASH_SIZE,
    canary_blob,
    canary_expect,
    crc16_ccitt,
    drop_last_flash_page,
    find_canary_offset,
    flash_image_from_hex,
    parse_uart_line,
    PAGE,
)


def test_canary_pages() -> None:
    assert canary_expect(0) == 0x00
    assert canary_expect(63) == 63
    assert canary_expect(64) == 0xFF
    assert canary_expect(65) == 0xFE
    assert canary_expect(128) == 0xAA
    assert canary_expect(129) == 0x55
    assert canary_expect(192) == 0x00
    assert canary_expect(256) == 0xFF
    assert canary_expect(320) == 0x2A
    assert canary_expect(384) == 0xA5
    assert canary_expect(448) == 0x5A
    blob = canary_blob()
    assert len(blob) == CANARY_LEN
    assert blob[0] == 0 and blob[-1] == 0x5A


def test_crc_erased() -> None:
    assert crc16_ccitt(b"\xFF" * FLASH_SIZE) == crc16_ccitt(bytes([0xFF] * FLASH_SIZE))
    assert crc16_ccitt(b"\x00\x01\x02") != crc16_ccitt(b"\x00\x01\x03")


def test_parse_lines() -> None:
    r = parse_uart_line("@00000012 READY,who=canary,mcu=m8,f_cpu=16000000,sig_expect=1E9307")
    assert r is not None
    assert r.t_ms == 12
    stamped = parse_uart_line("1700000000000000000 @00000012 READY,who=canary,tcnt1=00AB")
    assert stamped is not None and stamped.event == "READY" and stamped.kv.get("tcnt1") == "00AB"
    assert r.event == "READY"
    assert r.kv["who"] == "canary"
    assert r.kv["mcu"] == "m8"
    assert r.kv["sig_expect"] == "1E9307"
    c = parse_uart_line("@00000186 RESET_CAUSE,csr=02,porf=0,extrf=1,borf=0,wdrf=0,eeprom=chip_erased,boot=1")
    assert c is not None and c.kv["extrf"] == "1" and c.kv["eeprom"] == "chip_erased"
    f = parse_uart_line("@00000200 FLASH_CRC,off=0000,len=2000,crc=ABCD,inject=1")
    assert f is not None and f.kv["crc"] == "ABCD" and f.kv["inject"] == "1"
    inj = parse_uart_line("@00000020 FAULT,kind=canary,arg=7")
    assert inj is not None and inj.kv["kind"] == "canary" and inj.kv["arg"] == "7"
    assert parse_uart_line("ping LEDs") is None
    assert parse_uart_line("") is None


def test_ihex_canary_roundtrip() -> None:
    blob = canary_blob()
    # Minimal ihex: canary at 0x1C00 (typical tail) plus a vector byte at 0.
    mem = bytearray([0xFF] * FLASH_SIZE)
    mem[0] = 0x0C
    mem[CANARY_ADDR : CANARY_ADDR + CANARY_LEN] = blob
    recs = []
    for addr in range(0, FLASH_SIZE, 16):
        chunk = bytes(mem[addr : addr + 16])
        if all(b == 0xFF for b in chunk) and addr != 0:
            continue
        if all(b == 0xFF for b in chunk):
            continue
        payload = bytes([len(chunk), (addr >> 8) & 0xFF, addr & 0xFF, 0]) + chunk
        csum = (-sum(payload)) & 0xFF
        recs.append(":" + payload.hex().upper() + f"{csum:02X}")
    recs.append(":00000001FF")
    text = "\n".join(recs) + "\n"
    image = flash_image_from_hex(text)
    assert find_canary_offset(image) == CANARY_ADDR
    assert crc16_ccitt(image) == crc16_ccitt(bytes(mem))


def test_truncated_canary_detected() -> None:
    mem = bytearray([0xFF] * FLASH_SIZE)
    mem[CANARY_ADDR : CANARY_ADDR + CANARY_LEN] = canary_blob()
    good = crc16_ccitt(bytes(mem))
    mem[FLASH_SIZE - 16] = 0x00  # last page corrupted
    assert crc16_ccitt(bytes(mem)) != good
    assert find_canary_offset(bytes(mem)) is None


def test_drop_last_page() -> None:
    mem = bytearray([0xFF] * FLASH_SIZE)
    mem[CANARY_ADDR : CANARY_ADDR + CANARY_LEN] = canary_blob()
    bad = drop_last_flash_page(bytes(mem), PAGE)
    assert bad[-PAGE:] == b"\xFF" * PAGE
    assert bad[:-PAGE] == bytes(mem)[:-PAGE]
    assert crc16_ccitt(bad) != crc16_ccitt(bytes(mem))


def main() -> int:
    test_canary_pages()
    test_crc_erased()
    test_parse_lines()
    test_ihex_canary_roundtrip()
    test_truncated_canary_detected()
    test_drop_last_page()
    print("ok  mega8-diag-oracle")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
