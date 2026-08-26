#!/usr/bin/env bash
# Build portable Linux x86-64 host client → dist/diagplane.bin
#
# Prefer musl + crt-static (runs on any glibc/musl x86-64 Linux).
# Fallback: host glibc + vendored libusb (no libusb-1.0.so; still tied to host glibc).
#
# Usage:
#   ./scripts/build-diagplane.sh [OUT_PATH]
# Default OUT: dist/diagplane.bin
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-${ROOT}/dist/diagplane.bin}"
CRATE="${ROOT}/tools/usbasp-ng-diag"
mkdir -p "$(dirname "$OUT")"

# Avoid probing system libudev/libusb so the vendored build stays netlink-only
# (no runtime libudev / libusb shared deps).
export PKG_CONFIG_PATH="/nonexistent"
export PKG_CONFIG_LIBDIR="/nonexistent"

TARGET_DIR="${CARGO_TARGET_DIR:-${CRATE}/target}"
BUILT=""
MODE=""

have_musl_linker() {
  command -v musl-gcc >/dev/null 2>&1 || command -v x86_64-linux-musl-gcc >/dev/null 2>&1
}

if have_musl_linker; then
  if ! rustup target list --installed 2>/dev/null | grep -qx 'x86_64-unknown-linux-musl'; then
    echo "==> rustup target add x86_64-unknown-linux-musl"
    rustup target add x86_64-unknown-linux-musl
  fi
  echo "==> diagplane: musl static (x86_64-unknown-linux-musl)"
  RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static" \
    cargo build --release --manifest-path "${CRATE}/Cargo.toml" \
      --target x86_64-unknown-linux-musl
  BUILT="${TARGET_DIR}/x86_64-unknown-linux-musl/release/usbasp-ng-diag"
  MODE="musl-static"
else
  echo "==> diagplane: host glibc + vendored libusb (install musl-gcc for fully portable)"
  cargo build --release --manifest-path "${CRATE}/Cargo.toml"
  BUILT="${TARGET_DIR}/release/usbasp-ng-diag"
  MODE="glibc-vendored"
fi

cp -f "$BUILT" "$OUT"
if command -v strip >/dev/null 2>&1; then
  strip "$OUT" || true
fi
chmod +x "$OUT"

echo "Wrote $OUT ($MODE, $(wc -c <"$OUT") bytes)"
if command -v file >/dev/null 2>&1; then
  file "$OUT"
fi
if command -v ldd >/dev/null 2>&1; then
  if ldd "$OUT" >/dev/null 2>&1; then
    echo "Dynamic deps:"
    ldd "$OUT" || true
  else
    echo "Static (no dynamic linker deps)"
  fi
fi
