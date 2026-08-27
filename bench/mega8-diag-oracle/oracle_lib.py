#!/usr/bin/env python3
"""Shared rules for the mega8 diag oracle: canary, CRC16, ihex, UART lines."""
from __future__ import annotations

import re
from dataclasses import dataclass, field

FLASH_SIZE = 8192
CANARY_LEN = 512
CANARY_ADDR = 0x1E00
PAGE = 64
CANARY_PAGES = CANARY_LEN // PAGE

# CRC-CCITT (0xFFFF, poly 0x1021, xorout 0) — same loop as firmware.
def crc16_ccitt(data: bytes, crc: int = 0xFFFF) -> int:
    for b in data:
        crc ^= b << 8
        for _ in range(8):
            if crc & 0x8000:
                crc = ((crc << 1) ^ 0x1021) & 0xFFFF
            else:
                crc = (crc << 1) & 0xFFFF
    return crc


def canary_expect(off: int) -> int:
    """Byte at offset 0..511. Must match firmware canary_expect()."""
    page = off // PAGE
    i = off % PAGE
    if page == 0:
        return i
    if page == 1:
        return 0xFF - i
    if page == 2:
        return 0x55 if (i & 1) else 0xAA
    if page == 3:
        return 0x00
    if page == 4:
        return 0xFF
    if page == 5:
        return (i * 7 + 0x2A) & 0xFF
    if page == 6:
        return 0xA5
    if page == 7:
        return 0x5A
    raise IndexError(off)


def canary_blob() -> bytes:
    return bytes(canary_expect(i) for i in range(CANARY_LEN))


def parse_ihex(text: str) -> dict[int, int]:
    mem: dict[int, int] = {}
    base = 0
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line[0] != ":":
            continue
        payload = bytes.fromhex(line[1:])
        ln = payload[0]
        addr = (payload[1] << 8) | payload[2]
        typ = payload[3]
        data = payload[4 : 4 + ln]
        csum = payload[4 + ln]
        if ((sum(payload[: 4 + ln]) + csum) & 0xFF) != 0:
            raise ValueError(f"bad ihex checksum: {line}")
        if typ == 0:
            for i, b in enumerate(data):
                mem[base + addr + i] = b
        elif typ == 1:
            break
        elif typ == 2:
            base = ((data[0] << 8) | data[1]) << 4
        elif typ == 4:
            base = ((data[0] << 8) | data[1]) << 16
    return mem


def flash_image_from_hex(text: str, size: int = FLASH_SIZE) -> bytes:
    mem = parse_ihex(text)
    buf = bytearray([0xFF] * size)
    for a, b in mem.items():
        if 0 <= a < size:
            buf[a] = b
    return bytes(buf)


_LINE_RE = re.compile(
    r"^@(?P<t>\d{6,8})\s+(?P<event>[A-Z][A-Z0-9_]*)(?P<rest>.*)$"
)


@dataclass
class OracleLine:
    t_ms: int
    event: str
    kv: dict[str, str] = field(default_factory=dict)
    raw: str = ""


def parse_uart_line(raw: str) -> OracleLine | None:
    s = raw.strip()
    if not s:
        return None
    parts = s.split(None, 1)
    if len(parts) == 2 and parts[0].isdigit() and parts[1].startswith("@"):
        s = parts[1]
    if not s or s[0] != "@":
        return None
    m = _LINE_RE.match(s)
    if not m:
        return None
    kv: dict[str, str] = {}
    rest = m.group("rest")
    if rest.startswith(","):
        rest = rest[1:]
    elif rest.startswith(" "):
        rest = rest[1:]
    if rest:
        for part in rest.split(","):
            part = part.strip()
            if not part:
                continue
            if "=" in part:
                k, v = part.split("=", 1)
                kv[k.strip()] = v.strip()
            else:
                kv[part] = "1"
    return OracleLine(t_ms=int(m.group("t")), event=m.group("event"), kv=kv, raw=s)


def find_canary_offset(image: bytes) -> int | None:
    blob = canary_blob()
    idx = image.find(blob)
    return idx if idx >= 0 else None


def drop_last_flash_page(image: bytes, page: int = PAGE) -> bytes:
    """Erase the last ISP page (real MEMOP truncate, not UART lie)."""
    buf = bytearray(image)
    buf[-page:] = b"\xFF" * page
    return bytes(buf)


def image_to_ihex(image: bytes) -> str:
    lines: list[str] = []
    for addr in range(0, len(image), 16):
        chunk = bytes(image[addr : addr + 16])
        if all(b == 0xFF for b in chunk):
            continue
        payload = bytes([len(chunk), (addr >> 8) & 0xFF, addr & 0xFF, 0]) + chunk
        csum = (-sum(payload)) & 0xFF
        lines.append(":" + payload.hex().upper() + f"{csum:02X}")
    lines.append(":00000001FF")
    return "\n".join(lines) + "\n"
