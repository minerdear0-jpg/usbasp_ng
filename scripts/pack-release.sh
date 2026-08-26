#!/usr/bin/env bash
# Production release artifacts for USBasp NG.
#
# Source ZIP (git archive only — never a working-tree dump):
#   source, tests, reference, docs, scripts, arduino, .github, …
# Never: .git/, firmware/build/, __pycache__/, *.obj, *.elf, *.hex
#
# Firmware HEX assets are separate files in dist/, not inside the source zip.
# Host client: dist/diagplane.bin via --diag (see scripts/build-diagplane.sh).
#
# Usage:
#   ./scripts/pack-release.sh [VERSION] [--hex] [--diag]
#   VERSION defaults to `git describe --tags --always`
#   --hex  classic/HIDUART hex for atmega8 + atmega88 + atmega328p (USBasp2)
#   --diag portable Linux x86-64 host client → dist/diagplane.bin
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BUILD_HEX=0
BUILD_DIAG=0
VERSION=""
for arg in "$@"; do
  case "$arg" in
    --hex) BUILD_HEX=1 ;;
    --diag) BUILD_DIAG=1 ;;
    -*)
      echo "unknown option: $arg" >&2
      exit 2
      ;;
    *)
      if [[ -n "$VERSION" ]]; then
        echo "unexpected argument: $arg" >&2
        exit 2
      fi
      VERSION="$arg"
      ;;
  esac
done

if [[ -z "$VERSION" ]]; then
  VERSION="$(git describe --tags --always --dirty 2>/dev/null || echo unknown)"
fi
SAFE="${VERSION#v}"
DIST="${ROOT}/dist"
mkdir -p "$DIST"

SRC_ZIP="${DIST}/usbasp-ng-src-v${SAFE}.zip"
PREFIX="usbasp-ng-v${SAFE}"

echo "==> source zip (git archive HEAD)"
git archive --format=zip --prefix="${PREFIX}/" -o "$SRC_ZIP" HEAD
echo "Wrote $SRC_ZIP"

# Fail hard if the archive looks like a developer dump.
BAD=$(unzip -Z1 "$SRC_ZIP" | grep -E '(^|/)\.git/|/firmware/build/|/__pycache__/|\.pyc$|\.o$|\.obj$|\.elf$|\.hex$|\.eep$|\.map$' || true)
if [[ -n "$BAD" ]]; then
  echo "ERROR: source zip contains forbidden paths:" >&2
  echo "$BAD" >&2
  exit 1
fi
COUNT=$(unzip -Z1 "$SRC_ZIP" | wc -l)
echo "OK: $COUNT entries, no .git / build / object / hex junk"

if [[ "$BUILD_HEX" -eq 1 ]]; then
  echo "==> firmware hex assets"
  # Release names (stable public filenames) ← board profiles
  declare -a JOBS=(
    "usbasp-atmega8-clone|classic|usbasp|usbasp-ng-classic-atmega8.hex|"
    "usbasp-atmega88|classic|usbasp|usbasp-ng-classic-atmega88.hex|"
    "usbasp-hiduart-atmega8|hiduart|usbasp-hiduart|usbasp-ng-hiduart-atmega8.hex|usbasp-ng-hiduart-atmega8.eep"
    "usbasp-hiduart-atmega88|hiduart|usbasp-hiduart|usbasp-ng-hiduart-atmega88.hex|usbasp-ng-hiduart-atmega88.eep"
    "usbasp-hiduart-atmega328p|hiduart|usbasp-hiduart|usbasp-ng-hiduart-atmega328p.hex|usbasp-ng-hiduart-atmega328p.eep"
  )
  for job in "${JOBS[@]}"; do
    IFS='|' read -r BOARD PROFILE HEX_STEM OUT_HEX OUT_EEP <<<"$job"
    SERIAL=0000
    make -C "$ROOT/firmware" BOARD="$BOARD" SERIAL="$SERIAL" hex
    SRC_HEX="$ROOT/firmware/build/${BOARD}/${HEX_STEM}.hex"
    cp -f "$SRC_HEX" "${DIST}/${OUT_HEX}"
    echo "  ${OUT_HEX}  ($(wc -c <"${DIST}/${OUT_HEX}") bytes)"
    if [[ -n "$OUT_EEP" ]]; then
      SRC_EEP="$ROOT/firmware/build/${BOARD}/${HEX_STEM}.eep"
      cp -f "$SRC_EEP" "${DIST}/${OUT_EEP}"
      echo "  ${OUT_EEP}  ($(wc -c <"${DIST}/${OUT_EEP}") bytes)"
    fi
  done
fi

if [[ "$BUILD_DIAG" -eq 1 ]]; then
  echo "==> diagplane.bin (Linux x86-64)"
  "${ROOT}/scripts/build-diagplane.sh" "${DIST}/diagplane.bin"
fi

echo "==> dist/"
ls -la "$DIST"/usbasp-ng-*"${SAFE}"* "$DIST"/usbasp-ng-*.hex "$DIST"/usbasp-ng-*.eep "$DIST"/diagplane.bin 2>/dev/null || ls -la "$DIST"
echo "Done. Attach source zip + hex + diagplane.bin as separate GitHub release assets."
echo "Do not upload a working-tree dump. Prefer: ./scripts/pack-release.sh ${SAFE} --hex --diag"
