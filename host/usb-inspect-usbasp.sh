#!/bin/sh
# USB mode: plug the DUT into the host (unplug the other stick — same VID/PID).
# J2 open on the DUT. Prefer: python3 host/usbaspctl.py info
set -e
ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
VIDPID="16c0:05dc"
if ! lsusb -d "$VIDPID" >/dev/null 2>&1; then
    echo "No USBasp $VIDPID on the bus." >&2
    exit 1
fi
echo "=== lsusb ==="
lsusb -d "$VIDPID"
if command -v python3 >/dev/null 2>&1 && [ -f "$ROOT/host/usbaspctl.py" ]; then
    echo "=== usbaspctl info ==="
    python3 "$ROOT/host/usbaspctl.py" info || true
fi
echo "=== descriptors ==="
lsusb -d "$VIDPID" -v
