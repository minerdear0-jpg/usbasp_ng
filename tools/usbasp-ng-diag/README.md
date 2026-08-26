# usbasp-ng-diag

Host tool for the USBasp NG **Diagnostics Plane** (HID EP2, DIAG v1).

```bash
cargo build --release
./target/release/usbasp-ng-diag record YEL0 capture.bin
./target/release/usbasp-ng-diag decode capture.bin
./target/release/usbasp-ng-diag monitor YEL0
./target/release/usbasp-ng-diag monitor YEL0 --json
```

Contracts: [`docs/DIAGNOSTICS.md`](../../docs/DIAGNOSTICS.md), client notes: [`docs/DIAGNOSTICS_CLIENT.md`](../../docs/DIAGNOSTICS_CLIENT.md).
