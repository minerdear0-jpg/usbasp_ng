# usbasp-ng-diag (diagplane)

Host tool for the USBasp NG **Diagnostics Plane** (HID EP2).

Versions (independent):

- **diagplane** — this client (`Cargo.toml` / `diagplane --version`)
- **protocol** — EP2 wire schema (`DIAG_SCHEMA_V1` = 1)

TUI header shows both, e.g. `diagplane 0.1.4  protocol 1`.

**Rust:** 1.78 or newer (`rust-version` in `Cargo.toml`). `Cargo.lock` is **v4** — cargo older than 1.78 will refuse it. That is expected; use a current toolchain rather than editing the lockfile. `diagplane.bin` needs no rustc.

`watch` needs a real pty. Headless (SSH without `-t`, detached tmux, CI): use `demo --jsonl`, `decode FILE --jsonl`, or `snapshot`.

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
./target/release/usbasp-ng-diag watch --diag ep2.jsonl --uart oracle.txt
./target/release/usbasp-ng-diag monitor YEL0
./target/release/usbasp-ng-diag capabilities --demo capabilities_yel0
# live CAPS = ISP CONNECT (start this, then avrdude)
./target/release/usbasp-ng-diag capabilities --serial YEL0 --timeout 30
./target/release/usbasp-ng-diag snapshot --demo memop_flash
./target/release/usbasp-ng-diag snapshot --demo enableprog_fail_sw --json
# live: wait for SESSION_END then dump (start this, then avrdude)
./target/release/usbasp-ng-diag snapshot --serial YEL0
./target/release/usbasp-ng-diag evidence --demo enableprog_fail_sw
./target/release/usbasp-ng-diag evidence --demo enableprog_ladder_silent --json
./target/release/usbasp-ng-diag analyze --demo enableprog_fail_sw
./target/release/usbasp-ng-diag analyze --demo memop_flash --out /tmp/cage.usbasp2e --json
./target/release/usbasp-ng-diag correlate --diag ep2.jsonl --uart oracle.txt
# UART host stamps (python3 harness.py monitor):  HOST_NS @ms EVENT
# → dt_ready_host_ns ± doubt_ns  (Cristian |dt|/2)
```

TUI keys: `q` quit, `x` / `Ctrl+L` clear (confirm `y`), `w` wire frames, `f` faults, `c` caps, `j`/`k` scroll, `g`/`G` top/bottom, `Space` HOLD/RUN (RUN blinks = alive). Dual-column when `--uart` is set (yellow row = RELEASE↔READY **order**, not absolute µs). Full-width **VERDICT** rail under the log is a host evidence viewer (`evidence` / future analyzers), not firmware.

Diagplane answers **what firmware observed**. **Why** is a host analysis layer. Pin edges need a sniffer; TRACE aims it, it does not replace it.

Live `capabilities`: device on USB alone is not a diag session — see [ACCEPTANCE-DIAG-TRIGGER-001](../../docs/acceptance/ACCEPTANCE-DIAG-TRIGGER-001.md).


Release asset: **`diagplane.bin`** (Linux x86-64, musl static) — same CLI. Build locally: `../../scripts/build-diagplane.sh`.

- `--jsonl` — lnav-ready JSON Lines on stdout  
- `--faults` — ERROR / OVERFLOW / FAIL + summary  
- `watch` — ratatui console UI (file / demo / live); needs an interactive terminal  
- `snapshot` — one coherent dump of USB/ISP/TRACE/MEMOP (file / demo / jsonl / live)
- `evidence` — Evidence Record v1: expected/observed/verdict; never claims pin capture from EP2 ([EVIDENCE.md](../../docs/EVIDENCE.md))
- `analyze` — ISP session analyzer → Findings → Verdict; `--out file.usbasp2e` for offline replay. TUI is not this engine.

Contracts: [`docs/DIAGNOSTICS.md`](../../docs/DIAGNOSTICS.md), client notes: [`docs/DIAGNOSTICS_CLIENT.md`](../../docs/DIAGNOSTICS_CLIENT.md).

## lnav

Prefer `--jsonl` straight into lnav (see above). Format install once:

```bash
lnav -i lnav/usbasp_ng_diag.json
```
