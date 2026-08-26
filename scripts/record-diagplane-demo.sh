#!/usr/bin/env bash
# Live tty tour of USBasp2 beta.1 diagplane (no fuse writes).
# Checked-in recording: python3 scripts/record-diagplane-cast.py
#   → docs/media/demo-diagplane-beta1.cast
# Usage: ./scripts/record-diagplane-demo.sh [DIAGPLANE_BIN]
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${1:-$ROOT/dist/diagplane.bin}"
export TERM="${TERM:-xterm-256color}"

echo "USBasp2 beta.1  —  diagplane + cage (YEL0 → mega8)"
echo "Reads only: signature, fuses, eeprom, flash. No fuse writes."
echo

if [[ -x "$BIN" ]]; then
  echo "== diagplane"
  "$BIN" demo --list
  echo
fi

if command -v avrdude >/dev/null 2>&1; then
  echo "== cage L1  avrdude -c usbasp -P usb:YEL0 -p m8 -B 8"
  avrdude -c usbasp -P usb:YEL0 -p m8 -B 8 -U signature:r:-:h
  echo
  echo "== fuses (read)"
  avrdude -c usbasp -P usb:YEL0 -p m8 -B 8 \
    -U lfuse:r:-:h -U hfuse:r:-:h -U lock:r:-:h
  echo
  echo "== eeprom (read 512 B, first 16 shown)"
  avrdude -c usbasp -P usb:YEL0 -p m8 -B 8 -U eeprom:r:/tmp/diagplane-demo-eeprom.bin:r
  xxd -l 16 /tmp/diagplane-demo-eeprom.bin
  echo
  echo "== flash (read 8 KiB, first 32 shown)"
  avrdude -c usbasp -P usb:YEL0 -p m8 -B 8 -U flash:r:/tmp/diagplane-demo-flash.bin:r
  xxd -l 32 /tmp/diagplane-demo-flash.bin
  echo
fi

echo "== watch  demo memop_flash  (FLASH write + READFLASH verify)"
sleep 0.6
timeout --signal=TERM -k 1 6 "$BIN" watch --demo memop_flash || true
sleep 0.5

echo "== watch  demo enableprog_fail_sw  (TARGET SILENT)"
sleep 0.6
timeout --signal=TERM -k 1 6 "$BIN" watch --demo enableprog_fail_sw || true
sleep 0.5

JSONL="$ROOT/bench/mega8-diag-oracle/captures/yel0-corr.jsonl"
UART="$ROOT/bench/mega8-diag-oracle/captures/oracle-uart.txt"
if [[ -f "$JSONL" && -f "$UART" ]]; then
  echo "== watch  dual-column  RELEASE↔READY  (live sample)"
  sleep 0.6
  timeout --signal=TERM -k 1 8 "$BIN" watch --diag "$JSONL" --uart "$UART" || true
  sleep 0.5
fi

echo
echo "done.  replay: asciinema play docs/media/demo-diagplane-beta1.cast"
