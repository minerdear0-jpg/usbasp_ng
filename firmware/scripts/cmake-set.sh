#!/bin/sh
# Print set(KEY value) from a board cmake file. Avoids GNU make $(shell) vs ')'.
key=$1
file=$2
exec awk -v key="$key" '
$1 == "set(" key { v = $2; sub(/[)]$/, "", v); print v; exit }
' "$file"
