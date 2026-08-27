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
| `05_flash_abort_line_anomaly.json` | LINE + ENABLEPROG PASS + CONT pages, **no MEMOP END** → `FAIL_UNCONFIRMED` (not PASS) |
| `06_flash_poll_fail_end_ok.json` | CONT|FAIL then END|OK + READFLASH → still `FAIL_UNCONFIRMED` (sticky poll) |

`LINE_FAULT` is never a physical proof. `PHYSICAL_CAPTURE` **capability** is
never evidence. `EVIDENCE.CONFLICT` requires two recorded sources.

CONT OK pages are not a finished write. `PASS` / `PASS_WITH_ANOMALY` require a
completed session outcome (MEMOP END if a MEMOP started).

Replay corpus (nine ISP demos): `cargo test corpus` in `tools/usbasp-ng-diag`.
Same capture must yield the same analysis JSON; raw sha256 must not move when
analysis is derived again.

```bash
cd tools/usbasp-ng-diag && cargo test goldens && cargo test corpus
```
