# usbasp-ng-diag

Host tool for the USBasp NG **Diagnostics Plane** (HID EP2, DIAG v1).

```bash
cargo build --release
./target/release/usbasp-ng-diag record YEL0 capture.bin
./target/release/usbasp-ng-diag decode capture.bin
./target/release/usbasp-ng-diag replay capture.bin --speed 10
./target/release/usbasp-ng-diag replay capture.bin --step
./target/release/usbasp-ng-diag demo --list
./target/release/usbasp-ng-diag demo enableprog_fail_sw --faults
./target/release/usbasp-ng-diag demo enableprog_fail_sw --jsonl | lnav
./target/release/usbasp-ng-diag decode capture.bin --faults
./target/release/usbasp-ng-diag watch --demo enableprog_fail_sw
./target/release/usbasp-ng-diag watch --file capture.bin
./target/release/usbasp-ng-diag watch --serial YEL0
./target/release/usbasp-ng-diag monitor YEL0
./target/release/usbasp-ng-diag capabilities --demo capabilities_yel0
./target/release/usbasp-ng-diag capabilities --serial YEL0
```

TUI keys: `q` quit, `f` faults filter, `c` capabilities panel, `j`/`k` scroll, `g`/`G` top/bottom, `Space` follow.

Release asset: **`diagplane.bin`** (Linux x86-64, musl static) — same CLI. Build locally: `../../scripts/build-diagplane.sh`.

- `--jsonl` — lnav-ready JSON Lines on stdout  
- `--faults` — ERROR / OVERFLOW / FAIL + summary  
- `watch` — ratatui console UI (file / demo / live)  

Contracts: [`docs/DIAGNOSTICS.md`](../../docs/DIAGNOSTICS.md), client notes: [`docs/DIAGNOSTICS_CLIENT.md`](../../docs/DIAGNOSTICS_CLIENT.md).

## lnav

Prefer `--jsonl` straight into lnav (see above). Format install once:

```bash
lnav -i lnav/usbasp_ng_diag.json
```
