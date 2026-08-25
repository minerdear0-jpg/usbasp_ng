#!/bin/sh
# USB mode: plug yellow-dot into the host, unplug no-dot (same VID/PID).
# J2 open. Expect classic: one vendor interface, class 0xff, no HID.
set -e
VIDPID="16c0:05dc"
if ! lsusb -d "$VIDPID" >/dev/null 2>&1; then
    echo "No USBasp $VIDPID on the bus." >&2
    exit 1
fi
echo "=== lsusb ==="
lsusb -d "$VIDPID"
echo "=== descriptors ==="
lsusb -d "$VIDPID" -v
