#!/usr/bin/env python3
"""Offline decoder for usbasp-hidraw-log.py captures (DIAG v1)."""
from __future__ import annotations
import struct
import sys
from pathlib import Path

TYPES = {
    1: "HELLO",
    2: "SESSION_BEGIN",
    3: "SESSION_END",
    4: "RESET",
    5: "SCK_CONFIG",
    6: "ENABLEPROG",
    7: "SPI_BYTE",
    8: "SCK_STATS",
    9: "FAULT_SNAPSHOT",
    10: "TRACE_OVERFLOW",
    11: "ERROR",
}

RESET_ASSERT = 0x01
RESET_RELEASE = 0x02
EP_START = 0x01
EP_CONT = 0x02
EP_END = 0x04
EP_OK = 0x10
EP_FAIL = 0x20
TRANSPORT = {0: "HW", 1: "SW"}


def _seq_flags(flags: int) -> str:
    parts = []
    if flags & EP_START:
        parts.append("START")
    if flags & EP_CONT:
        parts.append("CONT")
    if flags & EP_END:
        parts.append("END")
    if flags & EP_OK:
        parts.append("OK")
    if flags & EP_FAIL:
        parts.append("FAIL")
    return "|".join(parts) if parts else f"0x{flags:02x}"


def decode_frame(data: bytes) -> str:
    typ, flags, tlo, thi, a, b = data[0], data[1], data[2], data[3], data[4], data[5]
    ts = tlo | (thi << 8)
    name = TYPES.get(typ, f"TYPE_{typ}")
    extra = ""
    if typ == 1:
        extra = f" schema={a} profile={b} caps=0x{flags:02x}"
    elif typ == 4:
        if flags & RESET_ASSERT:
            extra = " ASSERT"
        if flags & RESET_RELEASE:
            extra = " RELEASE"
    elif typ == 5:
        extra = f" sck_id={a} transport={TRANSPORT.get(b, b)}"
    elif typ in (6, 9):
        extra = f" {_seq_flags(flags)} data={a:02x}{b:02x}"
    elif typ == 10:
        extra = f" dropped={a}"
    return f"t={ts:5d} {name:16s} flags=0x{flags:02x} a={a:02x} b={b:02x}{extra}"


def reassemble_enableprog(frames: list[bytes]) -> str | None:
    """frames: four ENABLEPROG USB reports (8 bytes each) or 6-byte payloads."""
    if len(frames) != 4:
        return None
    parts = []
    for fr in frames:
        d = fr[:6] if len(fr) >= 6 else fr
        parts.append((d[1], d[4], d[5]))
    if not (parts[0][0] & EP_START and parts[3][0] & EP_END):
        return None
    tx = bytes([parts[0][1], parts[0][2], parts[1][1], parts[1][2]])
    rx = bytes([parts[2][1], parts[2][2], parts[3][1], parts[3][2]])
    result = "PASS" if parts[3][0] & EP_OK else "FAIL" if parts[3][0] & EP_FAIL else "?"
    return f"ENABLEPROG  TX {tx.hex(' ').upper()}  RX {rx.hex(' ').upper()}  {result}"


def main() -> int:
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} capture.bin", file=sys.stderr)
        return 1
    path = Path(sys.argv[1])
    blob = path.read_bytes()
    rec = 8 + 8  # host ns + frame
    if len(blob) % rec != 0:
        print(f"warning: trailing {len(blob) % rec} bytes", file=sys.stderr)

    ep_buf: list[bytes] = []
    for i in range(0, len(blob) - rec + 1, rec):
        host_ns, = struct.unpack_from("<Q", blob, i)
        data = blob[i + 8 : i + 16]
        print(f"{host_ns}  {decode_frame(data)}")
        if data[0] == 6:
            ep_buf.append(data)
            if len(ep_buf) == 4:
                line = reassemble_enableprog(ep_buf)
                if line:
                    print(f"{'':20}>> {line}")
                ep_buf.clear()
        else:
            ep_buf.clear()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
