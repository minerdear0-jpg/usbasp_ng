#!/usr/bin/env python3
"""Python ↔ Rust decode parity for host/golden/diag/*.bin."""
from __future__ import annotations
import importlib.util
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
GOLDEN = Path(__file__).resolve().parent
TRACE = ROOT / "host" / "usbasp-trace.py"
RUST_DIR = ROOT / "tools" / "usbasp-ng-diag"
BIN = GOLDEN / "enableprog_fail_sw.bin"


def py_decode(path: Path) -> str:
    spec = importlib.util.spec_from_file_location("usbasp_trace", TRACE)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    # Reuse main by capturing stdout via decode helpers on blob
    import io
    from contextlib import redirect_stdout

    buf = io.StringIO()
    argv = sys.argv
    try:
        sys.argv = ["usbasp-trace.py", str(path)]
        with redirect_stdout(buf):
            rc = mod.main()
        assert rc == 0
    finally:
        sys.argv = argv
    return buf.getvalue()


def rust_decode(path: Path) -> str:
    if not (RUST_DIR / "Cargo.toml").is_file():
        raise SystemExit("skip: no Rust client tree")
    try:
        subprocess.run(["cargo", "--version"], capture_output=True, check=True)
    except (FileNotFoundError, subprocess.CalledProcessError):
        print("skip  decode_parity (no cargo)")
        raise SystemExit(0)
    r = subprocess.run(
        ["cargo", "run", "--quiet", "--", "decode", str(path)],
        cwd=RUST_DIR,
        capture_output=True,
        text=True,
        check=False,
    )
    if r.returncode != 0:
        print(r.stderr, file=sys.stderr)
        raise SystemExit(f"rust decode failed: {r.returncode}")
    return r.stdout


def semantic_lines(text: str) -> list[str]:
    """Drop host_ns prefix; keep frame decode + >> reassembled lines."""
    out = []
    for line in text.splitlines():
        if ">>" in line:
            out.append(line[line.index(">>") :].strip())
            continue
        # "12345  t=... NAME ..." or "t=..."
        if " t=" in line:
            out.append(line.split(" t=", 1)[1].strip())
        elif line.lstrip().startswith("t="):
            out.append(line.strip())
    return out


def main() -> int:
    assert BIN.is_file(), BIN
    py = semantic_lines(py_decode(BIN))
    rs = semantic_lines(rust_decode(BIN))
    if py != rs:
        print("MISMATCH python vs rust", file=sys.stderr)
        for i, (a, b) in enumerate(zip(py, rs)):
            if a != b:
                print(f"  [{i}] py: {a}", file=sys.stderr)
                print(f"  [{i}] rs: {b}", file=sys.stderr)
        if len(py) != len(rs):
            print(f"  len py={len(py)} rs={len(rs)}", file=sys.stderr)
        return 1
    assert any("ENABLEPROG" in x and "FAIL" in x for x in py)
    assert any("FAULT_SNAPSHOT" in x and "sw_delay=6" in x for x in py)
    print(f"ok  decode_parity  ({len(py)} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
