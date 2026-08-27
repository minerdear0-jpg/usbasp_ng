#!/usr/bin/env python3
"""Host side of the ATmega8 diag oracle (Channel 2).

No extra Python deps. TTY via termios. Optional diagplane JSONL as Channel 1.

  python3 harness.py test
  python3 harness.py crc mega8-diag-oracle.hex
  python3 harness.py monitor [--port /dev/ttyUSB0]
  python3 harness.py run [--port /dev/ttyUSB0] [--diag-jsonl FILE] [--b 8]
"""
from __future__ import annotations

import argparse
import json
import os
import select
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from oracle_lib import (  # noqa: E402
    FLASH_SIZE,
    OracleLine,
    crc16_ccitt,
    drop_last_flash_page,
    find_canary_offset,
    flash_image_from_hex,
    image_to_ihex,
    parse_uart_line,
    CANARY_ADDR,
)

HERE = Path(__file__).resolve().parent
DEFAULT_PORT = os.environ.get("USBASP_TARGET_UART", "/dev/ttyUSB0")
BAUD = 115200


def expected_crc(hex_path: Path) -> tuple[int, int | None, bytes]:
    image = flash_image_from_hex(hex_path.read_text())
    return crc16_ccitt(image), find_canary_offset(image), image


def cmd_crc(hex_path: Path) -> int:
    crc, off, image = expected_crc(hex_path)
    used = max((i for i, b in enumerate(image) if b != 0xFF), default=0) + 1
    print(f"hex={hex_path}")
    print(f"flash_crc={crc:04X} (8KiB, erased=FF)")
    print(f"canary_off={off if off is not None else 'MISSING'}")
    print(f"used_span={used}")
    if off != CANARY_ADDR:
        print(f"error: canary must sit at {CANARY_ADDR:#x} (tail), got {off}", file=sys.stderr)
        return 1
    return 0


def cmd_mangle(kind: str, hex_path: Path) -> int:
    image = flash_image_from_hex(hex_path.read_text())
    if kind == "last-page":
        image = drop_last_flash_page(image)
    else:
        print(f"unknown mangle {kind}", file=sys.stderr)
        return 2
    sys.stdout.write(image_to_ihex(image))
    return 0


def _tty_setup(fd: int) -> None:
    import array
    import fcntl
    import termios

    t = termios.tcgetattr(fd)
    iflag, oflag, cflag, lflag, ispeed, ospeed, cc = t
    iflag = 0
    oflag = 0
    lflag = 0
    cflag = termios.CS8 | termios.CREAD | termios.CLOCAL
    try:
        baud = termios.B115200
    except AttributeError:
        baud = ospeed
    ispeed = ospeed = baud
    cc[termios.VMIN] = 0
    cc[termios.VTIME] = 0
    termios.tcsetattr(fd, termios.TCSANOW, [iflag, oflag, cflag, lflag, ispeed, ospeed, cc])
    termios.tcflush(fd, termios.TCIOFLUSH)
    # Nano CH340: DTR → RESET. Drop DTR so ioctl/open does not fight ISP.
    try:
        buf = array.array("I", [0])
        fcntl.ioctl(fd, termios.TIOCMGET, buf, True)
        buf[0] &= ~termios.TIOCM_DTR
        fcntl.ioctl(fd, termios.TIOCMSET, buf, True)
    except (OSError, AttributeError):
        pass


class TargetUart:
    def __init__(self, port: str):
        self.port = port
        self.fd = os.open(port, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
        _tty_setup(self.fd)
        self.buf = bytearray()

    def close(self) -> None:
        os.close(self.fd)

    def write(self, s: str) -> None:
        os.write(self.fd, s.encode("ascii"))

    def read_lines(self, timeout: float) -> list[str]:
        deadline = time.monotonic() + timeout
        out: list[str] = []
        while time.monotonic() < deadline:
            remain = deadline - time.monotonic()
            r, _, _ = select.select([self.fd], [], [], max(0.0, remain))
            if not r:
                break
            try:
                chunk = os.read(self.fd, 256)
            except BlockingIOError:
                continue
            if not chunk:
                break
            self.buf.extend(chunk)
            while True:
                n = self.buf.find(b"\n")
                if n < 0:
                    break
                raw = bytes(self.buf[: n + 1])
                del self.buf[: n + 1]
                out.append(raw.decode("ascii", errors="replace").strip())
                deadline = time.monotonic() + 0.15
        return out

    def drain(self, timeout: float = 0.2) -> list[str]:
        return self.read_lines(timeout)


def collect_boot(uart: TargetUart, timeout: float) -> list[OracleLine]:
    lines: list[OracleLine] = []
    deadline = time.monotonic() + timeout
    saw_selftest = False
    while time.monotonic() < deadline:
        remain = max(0.05, deadline - time.monotonic())
        for raw in uart.read_lines(min(0.5, remain)):
            rec = parse_uart_line(raw)
            if rec is None:
                continue
            # DTR glitch or leftover burst can emit two boots; keep the last.
            if rec.event == "READY":
                lines = [rec]
                saw_selftest = False
                continue
            lines.append(rec)
            if rec.event == "SELFTEST":
                saw_selftest = True
                deadline = min(deadline, time.monotonic() + 0.3)
        if saw_selftest and time.monotonic() > deadline:
            break
    return lines


def validate(lines: list[OracleLine], hex_path: Path | None) -> tuple[str, list[str], dict]:
    reasons: list[str] = []
    by: dict[str, OracleLine] = {}
    canaries: list[OracleLine] = []
    for rec in lines:
        if rec.event == "CANARY":
            canaries.append(rec)
        else:
            by[rec.event] = rec

    if "READY" not in by:
        reasons.append("no READY")
    if "APP_START" not in by:
        reasons.append("no APP_START")
    if "RESET_CAUSE" not in by:
        reasons.append("no RESET_CAUSE")
    else:
        rc = by["RESET_CAUSE"]
        if rc.kv.get("extrf") != "1":
            reasons.append(f"RESET_CAUSE extrf={rc.kv.get('extrf')} (want 1 after ISP)")

    st = by.get("SELFTEST")
    if st is None:
        reasons.append("no SELFTEST")
    elif st.kv.get("result") != "PASS":
        reasons.append("SELFTEST FAIL")

    for c in canaries:
        if c.kv.get("result") != "PASS":
            reasons.append(f"CANARY {c.kv.get('page')} FAIL")

    flash = by.get("FLASH_CRC")
    expect_crc = None
    canary_off = None
    if hex_path is not None and hex_path.exists():
        expect_crc, canary_off, _ = expected_crc(hex_path)
        if flash is None:
            reasons.append("no FLASH_CRC")
        else:
            got = int(flash.kv.get("crc", "0"), 16)
            if got != expect_crc:
                reasons.append(f"FLASH_CRC {got:04X} != hex {expect_crc:04X}")
        if canary_off is None:
            reasons.append("canary pattern missing from hex")

    result = "PASS" if not reasons else "FAIL"
    summary = {
        "result": result,
        "failure_reasons": reasons,
        "events": {k: v.kv for k, v in by.items()},
        "canary_pages": [c.kv for c in canaries],
        "expect_crc": f"{expect_crc:04X}" if expect_crc is not None else None,
        "canary_off": canary_off,
    }
    return result, reasons, summary


def correlate(diag_jsonl: Path, target: list[OracleLine], t0_host_ns: int) -> list[dict]:
    rows: list[dict] = []
    if not diag_jsonl.exists():
        return rows
    interesting = {
        "RESET",
        "ENABLEPROG",
        "SESSION_BEGIN",
        "SESSION_END",
        "MEMOP",
        "SCK_CONFIG",
        "FAULT_SNAPSHOT",
        "TRACE_BEGIN",
        "TRACE_END",
    }
    for line in diag_jsonl.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        kind = obj.get("type") or obj.get("kind")
        if kind not in interesting and obj.get("kind") != "semantic":
            continue
        rows.append(
            {
                "src": "programmer",
                "host_ns": obj.get("host_ns"),
                "dt_ms": None
                if obj.get("host_ns") is None
                else round((int(obj["host_ns"]) - t0_host_ns) / 1e6, 3),
                "msg": obj.get("msg") or kind,
            }
        )
    for rec in target:
        if rec.event in {"HEARTBEAT"}:
            continue
        rows.append(
            {
                "src": "target",
                "host_ns": None,
                "dt_ms": rec.t_ms,
                "msg": rec.raw,
            }
        )
    return rows


def cmd_monitor(port: str) -> int:
    uart = TargetUart(port)
    print(f"# {port} 115200  Ctrl-C to stop", file=sys.stderr)
    try:
        while True:
            for raw in uart.read_lines(1.0):
                print(f"{time.time_ns()} {raw}", flush=True)
    except KeyboardInterrupt:
        print(file=sys.stderr)
        return 0
    finally:
        uart.close()


def cmd_run(args: argparse.Namespace) -> int:
    hex_path = Path(args.hex) if args.hex else HERE / "mega8-diag-oracle.hex"
    if not hex_path.exists():
        print(f"missing {hex_path} — run make first", file=sys.stderr)
        return 2

    uart = TargetUart(args.port)
    report: dict = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "port": args.port,
        "hex": str(hex_path),
    }
    try:
        uart.drain(0.2)
        uart.write("arm\n")
        armed = [parse_uart_line(x) for x in uart.read_lines(1.0)]
        armed = [x for x in armed if x]
        report["armed"] = [x.raw for x in armed]

        t0 = time.time_ns()
        cmd = [
            "avrdude",
            "-c",
            args.programmer,
            "-p",
            args.mcu,
            "-B",
            str(args.b),
            "-U",
            f"flash:w:{hex_path}:i",
        ]
        report["avrdude"] = {"command": " ".join(cmd)}
        p = subprocess.run(cmd, capture_output=True, text=True)
        t1 = time.time_ns()
        report["avrdude"]["returncode"] = p.returncode
        report["avrdude"]["duration_sec"] = round((t1 - t0) / 1e9, 3)
        report["avrdude"]["stderr"] = p.stderr[-2000:]
        if p.returncode != 0:
            report["result"] = "FAIL"
            report["failure_reasons"] = [f"avrdude exit {p.returncode}"]
            print(json.dumps(report, indent=2))
            return 1

        lines = collect_boot(uart, timeout=args.timeout)
        report["target_lines"] = [x.raw for x in lines]
        result, reasons, summary = validate(lines, hex_path)
        report.update(summary)

        if args.diag_jsonl:
            report["correlation"] = correlate(Path(args.diag_jsonl), lines, t0)

        out = Path(args.out) if args.out else HERE / "last_report.json"
        out.write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report, indent=2))
        print(f"# result={result} wrote {out}", file=sys.stderr)
        return 0 if result == "PASS" else 1
    finally:
        uart.close()


def main() -> int:
    ap = argparse.ArgumentParser(description="ATmega8 diag oracle harness")
    sub = ap.add_subparsers(dest="cmd", required=True)

    p_crc = sub.add_parser("crc", help="CRC of an ihex against 8KiB erased flash")
    p_crc.add_argument("hex")

    p_mon = sub.add_parser("monitor", help="print target UART")
    p_mon.add_argument("--port", default=DEFAULT_PORT)

    p_run = sub.add_parser("run", help="flash via avrdude and validate UART report")
    p_run.add_argument("--port", default=DEFAULT_PORT)
    p_run.add_argument("--hex", default="")
    p_run.add_argument("--programmer", default="usbasp")
    p_run.add_argument("--mcu", default="atmega8")
    p_run.add_argument("-B", "--b", default="8")
    p_run.add_argument("--timeout", type=float, default=8.0)
    p_run.add_argument("--diag-jsonl", default="")
    p_run.add_argument("--out", default="")

    p_mg = sub.add_parser("mangle", help="rewrite ihex: last-page erased (real MEMOP truncate)")
    p_mg.add_argument("kind", choices=["last-page"])
    p_mg.add_argument("hex")

    args = ap.parse_args()
    if args.cmd == "crc":
        return cmd_crc(Path(args.hex))
    if args.cmd == "monitor":
        return cmd_monitor(args.port)
    if args.cmd == "run":
        return cmd_run(args)
    if args.cmd == "mangle":
        return cmd_mangle(args.kind, Path(args.hex))
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
