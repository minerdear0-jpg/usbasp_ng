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
./target/release/usbasp-ng-diag decode capture.bin --jsonl | lnav
./target/release/usbasp-ng-diag monitor YEL0
./target/release/usbasp-ng-diag monitor YEL0 --json
```

New recordings write a 16-byte `USBDIAGv` header; legacy captures still decode.

- `--jsonl` — lnav-ready JSON Lines on stdout  
- `--faults` — ERROR / OVERFLOW / FAIL + summary  

Contracts: [`docs/DIAGNOSTICS.md`](../../docs/DIAGNOSTICS.md), client notes: [`docs/DIAGNOSTICS_CLIENT.md`](../../docs/DIAGNOSTICS_CLIENT.md).

## lnav

Prefer `--jsonl` straight into lnav (see above). Format install once:

```bash
lnav -i lnav/usbasp_ng_diag.json
```
