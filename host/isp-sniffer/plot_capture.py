#!/usr/bin/env python3
"""Plot / decode CSV from capture_sniffer.c (RST falling edge = t0)."""

from __future__ import annotations

import argparse
import re
import sys

LINE_RE = re.compile(r"^([0-9A-Fa-f]{4});([01]);([01]);([01]);([01])\s*$")
META_RE = re.compile(r"#\s*F_CPU=(\d+)\s+prescale=(\d+)")


def tick_us_from_meta(lines: list[str], override: float | None) -> float:
    if override is not None:
        return override
    for line in lines:
        m = META_RE.search(line)
        if m:
            f_cpu, prescale = int(m.group(1)), int(m.group(2))
            return (prescale * 1e6) / f_cpu
    return 8e6 / 12e6  # clone default: 12 MHz, Timer1 /8


def parse_lines(lines: list[str], tick_us: float):
    events = []
    for line in lines:
        m = LINE_RE.match(line.strip())
        if not m:
            continue
        t_hex, rst, mosi, miso, sck = m.groups()
        t_us = int(t_hex, 16) * tick_us
        events.append((t_us, int(rst), int(mosi), int(miso), int(sck)))
    return events


def read_from_file(path: str) -> tuple[list[str], list]:
    with open(path, encoding="utf-8", errors="ignore") as f:
        lines = f.readlines()
    tick_us = tick_us_from_meta(lines, None)
    return lines, parse_lines(lines, tick_us)


def read_from_serial(port: str, baud: int, tick_us: float | None):
    import serial

    ser = serial.Serial(port, baud, timeout=5)
    lines: list[str] = []
    print("Waiting for capture... trigger avrdude now.", file=sys.stderr)
    while True:
        raw = ser.readline()
        if not raw:
            continue
        text = raw.decode("ascii", errors="ignore")
        sys.stderr.write(text)
        lines.append(text)
        if "--- END" in text:
            break
    ser.close()
    tu = tick_us_from_meta(lines, tick_us)
    return parse_lines(lines, tu)


def to_steps(events, idx):
    xs, ys = [], []
    for t, *bits in events:
        level = bits[idx]
        if xs:
            xs.append(t)
            ys.append(ys[-1])
        xs.append(t)
        ys.append(level)
    return xs, ys


def decode_spi(events):
    if not events:
        return []
    prev_sck = events[0][4]
    bits_mosi, bits_miso, edge_times = [], [], []
    bytes_out = []

    def flush(force=False):
        n = len(bits_mosi)
        if n == 0:
            return
        if n == 8 or force:
            mosi_byte = miso_byte = 0
            for b in bits_mosi:
                mosi_byte = (mosi_byte << 1) | b
            for b in bits_miso:
                miso_byte = (miso_byte << 1) | b
            if n < 8:
                mosi_byte <<= 8 - n
                miso_byte <<= 8 - n
            bytes_out.append(
                {
                    "t_start": edge_times[0],
                    "t_end": edge_times[-1],
                    "mosi": mosi_byte,
                    "miso": miso_byte,
                    "n_bits": n,
                }
            )

    for t, _rst, mosi, miso, sck in events:
        if prev_sck == 0 and sck == 1:
            bits_mosi.append(mosi)
            bits_miso.append(miso)
            edge_times.append(t)
            if len(bits_mosi) == 8:
                flush()
                bits_mosi, bits_miso, edge_times = [], [], []
        prev_sck = sck
    flush(force=True)
    return bytes_out


def print_spi_table(spi_bytes):
    if not spi_bytes:
        print(
            "No SCK rising edges — no SPI bytes "
            "(SCK never toggled, or capture ended in the RST-low wait).",
            file=sys.stderr,
        )
        return

    print("\n--- SPI (mode 0, sample rising SCK, MSB first) ---")
    print(
        f"{'#':>3}  {'t_start(us)':>11}  {'t_end(us)':>10}  "
        f"{'dur(us)':>8}  {'bit_us':>7}  {'MOSI':>6}  {'MISO':>6}  bits"
    )
    for i, b in enumerate(spi_bytes):
        dur = b["t_end"] - b["t_start"]
        bit_us = dur / (b["n_bits"] - 1) if b["n_bits"] > 1 else float("nan")
        mosi_s = f"0x{b['mosi']:02X}" + ("*" if b["n_bits"] < 8 else "")
        miso_s = f"0x{b['miso']:02X}" + ("*" if b["n_bits"] < 8 else "")
        print(
            f"{i:>3}  {b['t_start']:>11.1f}  {b['t_end']:>10.1f}  "
            f"{dur:>8.1f}  {bit_us:>7.2f}  {mosi_s:>6}  {miso_s:>6}  {b['n_bits']}/8"
        )

    expected = [0xAC, 0x53, 0x00, 0x00]
    got = [b["mosi"] for b in spi_bytes if b["n_bits"] == 8][:4]
    if got == expected[: len(got)] and got:
        print(f"MOSI matches AC 53 00 00 prefix ({len(got)}/4).")
    elif got:
        exp_s = " ".join(f"{x:02X}" for x in expected[: len(got)])
        got_s = " ".join(f"{x:02X}" for x in got)
        print(f"WARNING: MOSI expected {exp_s}, got {got_s}")
    if len(spi_bytes) >= 3:
        ack = spi_bytes[2]["miso"]
        if ack == 0x53:
            print("Byte[2] MISO == 0x53 (ENABLEPROG ACK).")
        else:
            print(f"Byte[2] MISO == 0x{ack:02X} (expected 0x53) — FAIL.")


def plot_events(events, spi_bytes, out: str | None):
    import matplotlib.pyplot as plt

    channels = [("RST", 0), ("MOSI", 1), ("MISO", 2), ("SCK", 3)]
    fig, axes = plt.subplots(len(channels), 1, sharex=True, figsize=(14, 6))
    for ax, (name, idx) in zip(axes, channels):
        xs, ys = to_steps(events, idx)
        ax.step(xs, ys, where="post", linewidth=1.2)
        ax.set_ylim(-0.3, 1.3)
        ax.set_yticks([0, 1])
        ax.set_ylabel(name, rotation=0, labelpad=25, va="center")
        ax.grid(True, alpha=0.3)
    if spi_bytes:
        for b in spi_bytes:
            axes[-1].annotate(
                f"M:{b['mosi']:02X}\nS:{b['miso']:02X}",
                xy=(b["t_start"], 1.05),
                xycoords=("data", "axes fraction"),
                fontsize=7,
                ha="left",
                va="bottom",
            )
    axes[-1].set_xlabel("time since RST falling edge (us)")
    fig.suptitle("USBasp ISP capture (edge-triggered sniffer)")
    fig.tight_layout()
    if out:
        fig.savefig(out, dpi=150)
        print(f"Saved: {out}", file=sys.stderr)
    else:
        plt.show()


def main() -> int:
    ap = argparse.ArgumentParser()
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument("--file")
    src.add_argument("--port")
    ap.add_argument("--baud", type=int, default=38400)
    ap.add_argument("--tick-us", type=float, default=None, help="Override (12 MHz /8 = 0.667)")
    ap.add_argument("--out", default=None)
    ap.add_argument("--no-decode", action="store_true")
    ap.add_argument("--no-plot", action="store_true")
    args = ap.parse_args()

    if args.file:
        with open(args.file, encoding="utf-8", errors="ignore") as f:
            lines = f.readlines()
        tick_us = tick_us_from_meta(lines, args.tick_us)
        events = parse_lines(lines, tick_us)
    else:
        events = read_from_serial(args.port, args.baud, args.tick_us)

    if not events:
        print("No events parsed.", file=sys.stderr)
        return 1

    print(
        f"Parsed {len(events)} edges, span {events[-1][0] - events[0][0]:.1f} us",
        file=sys.stderr,
    )

    spi_bytes = [] if args.no_decode else decode_spi(events)
    if not args.no_decode:
        print_spi_table(spi_bytes)

    if not args.no_plot:
        plot_events(events, spi_bytes, args.out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
