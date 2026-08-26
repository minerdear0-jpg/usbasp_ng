#!/usr/bin/env python3
"""Offline decoder for DIAG v1 captures (host_ns + 8B report).

Usage:
  python3 host/usbasp-trace.py capture.bin
  python3 host/usbasp-trace.py capture.bin --jsonl > capture.jsonl
  lnav -f tools/usbasp-ng-diag/lnav/usbasp_ng_diag.json capture.jsonl
"""
from __future__ import annotations
import argparse
import json
import struct
import sys
from datetime import datetime, timezone
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
    12: "MEMOP",
}

RESET_ASSERT = 0x01
RESET_RELEASE = 0x02
EP_START = 0x01
EP_CONT = 0x02
EP_END = 0x04
EP_OK = 0x10
EP_FAIL = 0x20
TRANSPORT = {0: "HW", 1: "SW"}
MEM_KIND = {0: "FLASH", 1: "EEPROM", 2: "READFLASH"}
CAPTURE_MAGIC = b"USBDIAGv"
CAPTURE_HEADER_SIZE = 16


def skip_capture_header(blob: bytes) -> tuple[bytes, dict | None]:
    """Return (records_blob, header_info|None). Legacy files have no header."""
    if len(blob) < 8 or blob[:8] != CAPTURE_MAGIC:
        return blob, None
    if len(blob) < CAPTURE_HEADER_SIZE:
        raise ValueError("truncated USBDIAGv header")
    info = {
        "format_version": blob[8],
        "diag_schema": blob[9],
        "record_size": blob[10],
        "flags": blob[11],
    }
    if info["format_version"] != 1:
        raise ValueError(f"unsupported capture format_version {info['format_version']}")
    if info["record_size"] != 16:
        raise ValueError(f"unsupported record_size {info['record_size']}")
    return blob[CAPTURE_HEADER_SIZE:], info


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


def host_ns_iso(host_ns: int) -> str:
    sec = host_ns // 1_000_000_000
    nsec = host_ns % 1_000_000_000
    dt = datetime.fromtimestamp(sec, tz=timezone.utc)
    return dt.strftime("%Y-%m-%dT%H:%M:%S.") + f"{nsec:09d}"[:6] + "Z"


def level_for(typ: int, flags: int) -> str:
    if typ == 10:  # OVERFLOW
        return "warning"
    if typ == 11:  # ERROR try-note
        return "error"
    if typ == 9 and (flags & EP_FAIL):
        return "error"
    if typ == 6 and (flags & EP_FAIL):
        return "error"
    if typ in (1, 2, 3, 4, 5):
        return "info"
    return "debug"


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
    elif typ == 6:
        extra = f" {_seq_flags(flags)} data={a:02x}{b:02x}"
    elif typ == 9:
        extra = f" {_seq_flags(flags)} data={a:02x}{b:02x}"
        if flags & EP_START:
            extra += (
                f" sck_req={a >> 4} eff={a & 0x0f}"
                f" transport={TRANSPORT.get(b, b)}"
            )
        elif flags & EP_END:
            res = "FAIL" if flags & EP_FAIL else "OK" if flags & EP_OK else "?"
            extra += f" rx0=0x{a:02x} sw_delay={b} {res}"
    elif typ == 10:
        extra = f" dropped={a}"
    elif typ == 11:
        path = "AVR" if flags & 0x01 else "AT89" if flags & 0x02 else "?"
        extra = f" try={path} check=0x{a:02x} sw_delay={b}"
    elif typ == 12:
        mem = MEM_KIND.get(a, f"mem{a}")
        if flags & EP_START:
            extra = f" START {mem} pagesize={b}"
        elif flags & EP_END:
            extra = f" END {mem} pages={b}"
        else:
            extra = f" {_seq_flags(flags)} {mem} b={b}"
    return f"t={ts:5d} {name:16s} flags=0x{flags:02x} a={a:02x} b={b:02x}{extra}"


def frame_fields(data: bytes) -> dict:
    typ, flags, tlo, thi, a, b = data[0], data[1], data[2], data[3], data[4], data[5]
    tick = tlo | (thi << 8)
    name = TYPES.get(typ, f"TYPE_{typ}")
    ev: dict = {
        "type": name,
        "type_id": typ,
        "flags": flags,
        "tick": tick,
        "a": a,
        "b": b,
        "state": data[7] if len(data) > 7 else 0,
    }
    if typ == 1:
        ev.update(schema=a, profile=b, caps=flags)
    elif typ == 4:
        ev["reset"] = (
            "assert"
            if flags & RESET_ASSERT
            else "release" if flags & RESET_RELEASE else "unknown"
        )
    elif typ == 5:
        ev.update(sck_id=a, transport=TRANSPORT.get(b, str(b)))
    elif typ == 6:
        ev["seq"] = _seq_flags(flags)
    elif typ == 9:
        ev["seq"] = _seq_flags(flags)
        if flags & EP_START:
            ev.update(
                sck_req=a >> 4,
                effective_sck=a & 0x0F,
                transport=TRANSPORT.get(b, str(b)),
            )
        elif flags & EP_END:
            ev.update(
                rx0=a,
                sw_delay=b,
                result="FAIL" if flags & EP_FAIL else "OK" if flags & EP_OK else "?",
            )
    elif typ == 10:
        ev["dropped"] = a
    elif typ == 11:
        ev.update(
            try_path="AVR" if flags & 0x01 else "AT89" if flags & 0x02 else "?",
            check=a,
            sw_delay=b,
        )
    elif typ == 12:
        ev.update(
            mem=MEM_KIND.get(a, f"mem{a}"),
            seq=_seq_flags(flags),
        )
        if flags & EP_START:
            ev["pagesize"] = b
        elif flags & EP_END:
            ev["pages"] = b
    return ev


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


def reassemble_enableprog_dict(frames: list[bytes]) -> dict | None:
    if len(frames) != 4:
        return None
    parts = []
    for fr in frames:
        d = fr[:6] if len(fr) >= 6 else fr
        parts.append((d[1], d[4], d[5]))
    if not (parts[0][0] & EP_START and parts[3][0] & EP_END):
        return None
    tx = [parts[0][1], parts[0][2], parts[1][1], parts[1][2]]
    rx = [parts[2][1], parts[2][2], parts[3][1], parts[3][2]]
    result = "PASS" if parts[3][0] & EP_OK else "FAIL" if parts[3][0] & EP_FAIL else "?"
    return {
        "kind": "enableprog",
        "tx": [f"{x:02x}" for x in tx],
        "rx": [f"{x:02x}" for x in rx],
        "result": result,
        "level": "error" if result == "FAIL" else "info",
        "msg": f"ENABLEPROG TX {' '.join(f'{x:02X}' for x in tx)} "
        f"RX {' '.join(f'{x:02X}' for x in rx)} {result}",
    }


def reassemble_fault_snapshot(frames: list[bytes]) -> str | None:
    """Four compact FAULT_SNAPSHOT frames → one semantic line."""
    if len(frames) != 4:
        return None
    parts = []
    for fr in frames:
        d = fr[:6] if len(fr) >= 6 else fr
        parts.append((d[1], d[4], d[5]))
    if not (parts[0][0] & EP_START and parts[3][0] & EP_END):
        return None
    packed, transport = parts[0][1], parts[0][2]
    reset_driven, state = parts[1][1], parts[1][2]
    tx0, tx1 = parts[2][1], parts[2][2]
    rx0, sw_delay = parts[3][1], parts[3][2]
    end_flags = parts[3][0]
    result = (
        "FAIL" if end_flags & EP_FAIL else "OK" if end_flags & EP_OK else "?"
    )
    tr = TRANSPORT.get(transport, transport)
    return (
        f"FAULT_SNAPSHOT  sck_req={packed >> 4} eff={packed & 0x0f}"
        f" transport={tr} reset=0x{reset_driven:02x} state=0x{state:02x}"
        f" tx={tx0:02x}{tx1:02x}.. rx0=0x{rx0:02x} sw_delay={sw_delay} {result}"
    )


def reassemble_fault_snapshot_dict(frames: list[bytes]) -> dict | None:
    line = reassemble_fault_snapshot(frames)
    if not line:
        return None
    parts = []
    for fr in frames:
        d = fr[:6] if len(fr) >= 6 else fr
        parts.append((d[1], d[4], d[5]))
    packed, transport = parts[0][1], parts[0][2]
    end_flags = parts[3][0]
    result = (
        "FAIL" if end_flags & EP_FAIL else "OK" if end_flags & EP_OK else "?"
    )
    return {
        "kind": "fault_snapshot",
        "sck_req": packed >> 4,
        "effective_sck": packed & 0x0F,
        "transport": TRANSPORT.get(transport, str(transport)),
        "reset_driven": parts[1][1],
        "state": parts[1][2],
        "tx01": f"{parts[2][1]:02x}{parts[2][2]:02x}",
        "rx0": parts[3][1],
        "sw_delay": parts[3][2],
        "result": result,
        "level": "error" if result == "FAIL" else "info",
        "msg": line,
    }


def emit_jsonl(host_ns: int, data: bytes, semantic: dict | None = None) -> None:
    ts = host_ns_iso(host_ns)
    if semantic is not None:
        row = {
            "ts": ts,
            "host_ns": host_ns,
            "level": semantic.get("level", "info"),
            "msg": semantic["msg"],
            **{k: v for k, v in semantic.items() if k not in ("level", "msg")},
        }
    else:
        fields = frame_fields(data)
        lvl = level_for(data[0], data[1])
        row = {
            "ts": ts,
            "host_ns": host_ns,
            "level": lvl,
            "msg": decode_frame(data),
            "kind": "frame",
            **fields,
        }
    print(json.dumps(row, separators=(",", ":"), ensure_ascii=False))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("capture", type=Path, help="capture.bin (host_ns + 8B)")
    ap.add_argument(
        "--jsonl",
        action="store_true",
        help="emit JSON Lines for lnav (ts + level + msg + fields)",
    )
    ap.add_argument(
        "--semantic-only",
        action="store_true",
        help="with --jsonl: only ENABLEPROG / FAULT_SNAPSHOT summaries",
    )
    args = ap.parse_args()

    blob = args.capture.read_bytes()
    try:
        blob, hdr = skip_capture_header(blob)
    except ValueError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    if hdr and not args.jsonl:
        print(
            f"capture header: format={hdr['format_version']} "
            f"schema={hdr['diag_schema']} record={hdr['record_size']}",
            file=sys.stderr,
        )
    elif not hdr and not args.jsonl:
        print("capture header: (legacy, no USBDIAGv)", file=sys.stderr)

    rec = 8 + 8
    if len(blob) % rec != 0:
        print(f"warning: trailing {len(blob) % rec} bytes", file=sys.stderr)

    ep_buf: list[bytes] = []
    snap_buf: list[bytes] = []
    for i in range(0, len(blob) - rec + 1, rec):
        (host_ns,) = struct.unpack_from("<Q", blob, i)
        data = blob[i + 8 : i + 16]
        if args.jsonl:
            if not args.semantic_only:
                emit_jsonl(host_ns, data)
        else:
            print(f"{host_ns}  {decode_frame(data)}")

        if data[0] == 6:
            snap_buf.clear()
            ep_buf.append(data)
            if len(ep_buf) == 4:
                if args.jsonl:
                    d = reassemble_enableprog_dict(ep_buf)
                    if d:
                        emit_jsonl(host_ns, data, d)
                else:
                    line = reassemble_enableprog(ep_buf)
                    if line:
                        print(f"{'':20}>> {line}")
                ep_buf.clear()
        elif data[0] == 9:
            ep_buf.clear()
            snap_buf.append(data)
            if len(snap_buf) == 4:
                if args.jsonl:
                    d = reassemble_fault_snapshot_dict(snap_buf)
                    if d:
                        emit_jsonl(host_ns, data, d)
                else:
                    line = reassemble_fault_snapshot(snap_buf)
                    if line:
                        print(f"{'':20}>> {line}")
                snap_buf.clear()
        else:
            ep_buf.clear()
            snap_buf.clear()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
