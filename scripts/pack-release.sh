#!/usr/bin/env bash
# Clean source distribution: tracked tree only (no .git, no firmware/build/).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
  VERSION="$(git describe --tags --always --dirty 2>/dev/null || echo unknown)"
fi
# Strip leading v for filename consistency when user passes v0.2.0
SAFE="${VERSION#v}"
OUT="${ROOT}/dist/usbasp-ng-src-v${SAFE}.zip"
mkdir -p "${ROOT}/dist"

# Prefer git archive (excludes untracked build/, .git/, and respects export-ignore).
git archive --format=zip --prefix="usbasp-ng-v${SAFE}/" -o "$OUT" HEAD

echo "Wrote $OUT"
unzip -l "$OUT" | head -n 20
echo "..."
if unzip -l "$OUT" | grep -E '(\.git/|firmware/build/|__pycache__|\.o$|\.elf$)' >/dev/null; then
  echo "ERROR: archive contains build/VCS junk" >&2
  exit 1
fi
echo "OK: no .git / firmware/build / __pycache__ entries"
