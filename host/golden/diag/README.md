# DIAG v1 golden captures

Synthetic `uint64_le host_ns` + 8-byte HID report records for decoder parity.

| File | Scenario |
|------|----------|
| `enableprog_fail_sw.bin` | SW SCK ENABLEPROG FAIL + compact FAULT_SNAPSHOT |

```bash
python3 host/usbasp-trace.py host/golden/diag/enableprog_fail_sw.bin
cd tools/usbasp-ng-diag && cargo run --quiet -- decode ../../host/golden/diag/enableprog_fail_sw.bin
python3 host/golden/diag/test_decode_parity.py
```
