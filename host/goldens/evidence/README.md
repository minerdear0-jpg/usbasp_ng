# Evidence Engine goldens (constitution)

Four JSON specs. Each names a `diagplane` demo and the **required** analysis
assertions. They are not raw captures; the demo is the generator, this file is
the contract.

| File | Meaning |
|------|---------|
| `01_enableprog_pass.json` | ENABLEPROG + MEMOP → `PASS`, `ISP.PROGRAMMING_PATH` |
| `02_enableprog_pass_line_anomaly.json` | LINE echo + protocol success → `PASS_WITH_ANOMALY` (regression) |
| `03_enableprog_fail.json` | ENABLEPROG FAIL, no physical source → `FAIL_UNCONFIRMED` |
| `04_enableprog_fail_line_anomaly.json` | LINE + ENABLEPROG FAIL, still not `FAIL_CONFIRMED` |

`LINE_FAULT` is never a physical proof. `PHYSICAL_CAPTURE` **capability** is
never evidence. `EVIDENCE.CONFLICT` requires two recorded sources.

```bash
cd tools/usbasp-ng-diag && cargo test goldens
```
