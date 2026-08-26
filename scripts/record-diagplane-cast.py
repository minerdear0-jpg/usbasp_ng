#!/usr/bin/env python3
"""Record docs/media/demo-diagplane-beta1.cast (asciicast v2).

Needs a real pty with winsize — ratatui draws nothing on 0×0 / redirected stdout.
Reads only on the cage (no fuse writes).
"""
from __future__ import annotations

import fcntl
import json
import os
import pty
import select
import signal
import struct
import subprocess
import sys
import termios
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "dist" / "diagplane.bin"
OUT = ROOT / "docs" / "media" / "demo-diagplane-beta1.cast"
COLS, ROWS = 120, 36
AVRDUDE = [
    "avrdude",
    "-c",
    "usbasp",
    "-P",
    "usb:YEL0",
    "-p",
    "m8",
    "-B",
    "8",
]


class Cast:
    def __init__(self) -> None:
        self.t0 = time.monotonic()
        self.events: list[tuple[float, str, str]] = []

    def now(self) -> float:
        return time.monotonic() - self.t0

    def out(self, data: str | bytes) -> None:
        if isinstance(data, bytes):
            text = data.decode("utf-8", "replace")
        else:
            text = data
        if text:
            self.events.append((self.now(), "o", text))

    def dump(self, path: Path) -> None:
        header = {
            "version": 2,
            "width": COLS,
            "height": ROWS,
            "timestamp": int(time.time()),
            "title": "USBasp2 beta.1 — diagplane watch + cage flash/eeprom/fuses read",
            "env": {"TERM": "xterm-256color", "SHELL": "/bin/sh"},
        }
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("w", encoding="utf-8") as f:
            f.write(json.dumps(header, ensure_ascii=False) + "\n")
            for t, kind, payload in self.events:
                f.write(json.dumps([round(t, 6), kind, payload], ensure_ascii=False) + "\n")


def avrdude(cast: Cast, extra: list[str]) -> None:
    p = subprocess.run(
        AVRDUDE + extra,
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    # avrdude writes progress to stderr
    blob = (p.stdout or "") + (p.stderr or "")
    if not blob.endswith("\n"):
        blob += "\n"
    cast.out(blob.replace("\n", "\r\n"))
    if p.returncode != 0:
        raise SystemExit(f"avrdude failed ({p.returncode}): {blob}")


def watch(cast: Cast, args: list[str], seconds: float) -> None:
    pid, fd = pty.fork()
    if pid == 0:
        os.environ["TERM"] = "xterm-256color"
        os.execv(str(BIN), [str(BIN), "watch", *args])
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
    try:
        os.kill(pid, signal.SIGWINCH)
    except ProcessLookupError:
        pass
    deadline = time.monotonic() + seconds
    try:
        while time.monotonic() < deadline:
            r, _, _ = select.select([fd], [], [], 0.15)
            if not r:
                continue
            try:
                chunk = os.read(fd, 65536)
            except OSError:
                break
            if not chunk:
                break
            cast.out(chunk)
        try:
            os.write(fd, b"q")
        except OSError:
            pass
        time.sleep(0.15)
        os.kill(pid, signal.SIGTERM)
    finally:
        try:
            os.close(fd)
        except OSError:
            pass
        try:
            os.waitpid(pid, 0)
        except ChildProcessError:
            pass


def main() -> int:
    if not BIN.is_file():
        raise SystemExit(f"missing {BIN} (pack --diag first)")
    os.chmod(BIN, os.stat(BIN).st_mode | 0o111)
    c = Cast()
    c.out(
        "USBasp2 beta.1 — diagplane + cage (YEL0 → mega8)\r\n"
        "Reads only: signature, fuses, eeprom, flash. No fuse writes.\r\n\r\n"
    )
    demo_list = subprocess.check_output([str(BIN), "demo", "--list"], text=True)
    c.out("== diagplane demo --list\r\n" + demo_list.replace("\n", "\r\n") + "\r\n")

    c.out("== signature\r\n")
    avrdude(c, ["-U", "signature:r:-:h"])
    c.out("\r\n== fuses (read)\r\n")
    avrdude(c, ["-U", "lfuse:r:-:h", "-U", "hfuse:r:-:h", "-U", "lock:r:-:h"])
    c.out("\r\n== eeprom (read)\r\n")
    eep = "/tmp/diagplane-demo-eeprom.bin"
    avrdude(c, ["-U", f"eeprom:r:{eep}:r"])
    xxd = subprocess.check_output(["xxd", "-l", "16", eep], text=True)
    c.out(xxd.replace("\n", "\r\n") + "\r\n")
    c.out("== flash (read 8 KiB)\r\n")
    fl = "/tmp/diagplane-demo-flash.bin"
    avrdude(c, ["-U", f"flash:r:{fl}:r"])
    xxd = subprocess.check_output(["xxd", "-l", "32", fl], text=True)
    c.out(xxd.replace("\n", "\r\n") + "\r\n")

    c.out("== watch --demo memop_flash  (FLASH + READFLASH)\r\n")
    watch(c, ["--demo", "memop_flash"], 4.0)
    c.out("\r\n== watch --demo enableprog_fail_sw  (TARGET SILENT)\r\n")
    watch(c, ["--demo", "enableprog_fail_sw"], 4.0)

    jsonl = ROOT / "bench/mega8-diag-oracle/captures/yel0-corr.jsonl"
    uart = ROOT / "bench/mega8-diag-oracle/captures/oracle-uart.txt"
    if jsonl.is_file() and uart.is_file():
        c.out("\r\n== watch dual-column  RELEASE↔READY\r\n")
        watch(c, ["--diag", str(jsonl), "--uart", str(uart)], 5.0)

    c.out("\r\nreplay: asciinema play docs/media/demo-diagplane-beta1.cast\r\n")
    c.dump(OUT)
    print(f"Wrote {OUT} ({OUT.stat().st_size} bytes, {len(c.events)} events)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
