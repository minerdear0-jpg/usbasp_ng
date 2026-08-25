# Historical reference snapshots

These directories are historical references.

- They are **not** build inputs.
- They are **not** modified as part of USBasp NG development.
- They exist so a behaviour can be traced to a known upstream tree.

| Directory | Upstream |
|-----------|----------|
| `usbasp-2011-05-28/` | Thomas Fischl USBasp 2011-05-28 (protocol / device identity) |
| `dioannidis-v1.11/` | Dimitrios Ioannidis fork (HID UART, WCID, ISP fixes, AT89 probe) |

Working firmware lives in [`../firmware/`](../firmware/).

Do not nest a second copy of these trees inside `reference/` (no `dioannidis-v1.11/dioannidis/` or `usbasp-2011-05-28/usbasp.2011-05-28/`).
