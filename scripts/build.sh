#!/bin/sh
# One-shot firmware build from the repo root.
# Canonical build: CMake (firmware/CMakeLists.txt). This script and
# firmware/Makefile are convenience wrappers only — do not diverge flags.
# Full workflow (tests, flash, all boards): firmware/Makefile
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
FW="${ROOT}/firmware"

usage() {
    cat <<EOF
Lightweight USBasp NG firmware build (CMake + avr-gcc).

Usage: $0 [product|BOARD] [cmake-args...]

Products:
  classic   board usbasp-atmega8-clone   (default)
  hiduart   board usbasp-hiduart-atmega8 (USBHID composite)
  usbhid    alias for hiduart

Board names match firmware/boards/<name>.cmake.

Environment:
  SERIAL      HIDUART iSerial, exactly 4 [A-Za-z0-9] (default 0000)
  GENERATOR   CMake generator (reuse existing cache, else Ninja, else Unix Makefiles)

Examples:
  $0
  $0 hiduart
  SERIAL=YEL0 $0 usbhid
  $0 usbasp-atmega88
  SERIAL=YEL0 $0 usbasp-hiduart-atmega328p
EOF
}

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "$0: missing $1" >&2
        exit 1
    }
}

product=classic
case "${1-}" in
    -h|--help)
        usage
        exit 0
        ;;
    classic|hiduart|usbhid|usbasp-*)
        product=$1
        shift
        ;;
    "")
        ;;
    -*)
        echo "$0: unknown option $1 (try --help)" >&2
        exit 1
        ;;
    *)
        echo "$0: unknown product or board '$1' (try --help)" >&2
        exit 1
        ;;
esac

case "$product" in
    classic) BOARD=usbasp-atmega8-clone ;;
    hiduart|usbhid) BOARD=usbasp-hiduart-atmega8 ;;
    *) BOARD=$product ;;
esac

board_file="${FW}/boards/${BOARD}.cmake"
if [ ! -f "$board_file" ]; then
    echo "$0: no board profile ${board_file}" >&2
    exit 1
fi

need cmake
need avr-gcc
need avr-objcopy

SERIAL=${SERIAL:-0000}
BUILDDIR="${FW}/build/${BOARD}"

if [ -z "${GENERATOR-}" ]; then
    if [ -f "${BUILDDIR}/CMakeCache.txt" ]; then
        GENERATOR=$(sed -n 's/^CMAKE_GENERATOR:INTERNAL=//p' "${BUILDDIR}/CMakeCache.txt")
    fi
    if [ -z "${GENERATOR-}" ]; then
        if command -v ninja >/dev/null 2>&1; then
            GENERATOR=Ninja
        else
            GENERATOR="Unix Makefiles"
        fi
    fi
fi

if [ "$GENERATOR" = "Unix Makefiles" ]; then
    need make
elif [ "$GENERATOR" = Ninja ]; then
    need ninja
fi

cmake -S "$FW" -B "$BUILDDIR" -G "$GENERATOR" \
    -DBOARD="$BOARD" \
    -DUSBASP_SERIAL="$SERIAL" \
    "$@"
cmake --build "$BUILDDIR"

if [ -f "${BUILDDIR}/usbasp-hiduart.hex" ]; then
    hex="${BUILDDIR}/usbasp-hiduart.hex"
else
    hex="${BUILDDIR}/usbasp.hex"
fi
echo "hex: ${hex}"
if [ -f "${BUILDDIR}/usbasp-hiduart.eep" ]; then
    echo "eep: ${BUILDDIR}/usbasp-hiduart.eep  (SERIAL=${SERIAL})"
fi
