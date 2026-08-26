# usbasp-ng-diag

Host tool for the USBasp NG **Diagnostics Plane** (HID EP2, DIAG v1).

```bash
cargo build --release
./target/release/usbasp-ng-diag record YEL0 capture.bin
./target/release/usbasp-ng-diag decode capture.bin
./target/release/usbasp-ng-diag replay capture.bin --speed 10
./target/release/usbasp-ng-diag replay capture.bin --step
./target/release/usbasp-ng-diag demo --list
./target/release/usbasp-ng-diag demo enableprog_fail_sw
./target/release/usbasp-ng-diag demo memop_flash --out /tmp/m.bin
./target/release/usbasp-ng-diag monitor YEL0
./target/release/usbasp-ng-diag monitor YEL0 --json
```

New recordings write a 16-byte `USBDIAGv` header; legacy captures still decode.

Contracts: [`docs/DIAGNOSTICS.md`](../../docs/DIAGNOSTICS.md), client notes: [`docs/DIAGNOSTICS_CLIENT.md`](../../docs/DIAGNOSTICS_CLIENT.md).

## lnav

```bash
python3 ../../host/usbasp-trace.py capture.bin --jsonl > capture.jsonl
lnav -i lnav/usbasp_ng_diag.json
lnav capture.jsonl
```
