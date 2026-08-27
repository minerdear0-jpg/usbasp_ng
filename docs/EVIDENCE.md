# Diagnostic Evidence Record (host v1)

USBasp2 does not promise to know the cause. It collects **evidence sufficient to test a hypothesis**.

This is a **host container**, not a new EP2 telemetry type. Wire stays [DIAGNOSTICS.md](DIAGNOSTICS.md). Probe honesty stays [DIAGNOSTICS_PROBE.md](DIAGNOSTICS_PROBE.md).

```text
OBSERVE (RAM TRACE / EP2)     →  HOST RECORD
FROZEN snapshot               →  HOST VERDICT (this file)
optional EEPROM later         →  PERSISTENT_EVIDENCE (not shipped)
external sniffer              →  PHYSICAL_CAPTURE evidence (not the CAPS bit)
```

`diagplane evidence` builds observations from `AppState`. `snapshot` remains the flat instrument dump. `diagplane analyze` adds derived findings + verdict without replacing raw.

Semantics **evidence-v1** is frozen for this crate line: observation ≠ finding ≠ claim ≠ verdict. New sensors later; meaning of FAIL does not drift.

## Schema

`schema = 1`

| Block | Purpose |
|-------|---------|
| identity | `session_id`, `capture_id`, HELLO schema/profile, CAPS masks. `firmware_build_id` / `firmware_build_hash` / `board_id` are **null** until the wire provides them. Never copy `bcdDevice` into a build hash. |
| configuration | SCK **id** + HW/SW (not Hz), diag bits |
| target | signature **only** if a tagged source supplies it |
| execution | RESET intent, ENABLEPROG attempts, MEMOP/VERIFY |
| claims | expected / observed / protocol verdict |
| sources | recorded independent channels (`physical` only if a capture was stored) |
| result | observation vs interpretation vs `cannot_prove` |
| integrity | `protocol_observed`, `physical_capture_capability` (CAPS), `physical_capture` (source present), digest |
| provenance | diagplane version, protocol version, session complete |

Capability ≠ evidence: `PHYSICAL_CAPTURE` on CAPS means the board *could* attach a sniffer. `sources.physical` means it *did*. `FAIL_CONFIRMED` requires the latter (and a contradicting RST level), never the former.

Signature sources (when present): `ISP` (host avrdude), `target_uart`, `USBASP_OBSERVATION`, `PHYSICAL_CAPTURE`. Mixing them without a tag is a bug.

## Analysis (host, not firmware)

Analyzers emit findings. They do not mutate a finding when later events arrive. The correlator adds aggregates:

- `ISP.PROGRAMMING_PATH` — ENABLEPROG PASS **and** MEMOP PASS (VERIFY may corroborate; confidence stays HIGH)
- `EVIDENCE.CONFLICT` — two **recorded** sources disagree (e.g. internal RESET assert vs physical RST stayed HIGH). Conflict is not “who is lying”.

LINE GPIO echo + ENABLEPROG/MEMOP PASS is **not** a conflict (same source, later protocol refutes causality) → `PASS_WITH_ANOMALY`.

Constitution: [host/goldens/evidence/](../host/goldens/evidence/). Especially `02` and `04`.

```bash
diagplane analyze --demo pass_with_rst_anomaly
diagplane analyze --demo enableprog_fail_line_anomaly
diagplane analyze --file capture.bin --out session.usbasp2e
```

## `.usbasp2e` (format v2)

```text
USBASP2E
├── manifest          schema=evidence-v1, engine
├── provenance        build ids (honest nulls), session, capture, caps
├── raw               USBDIAGv bytes (hex + sha256) when ingest had a capture
├── observations      derived evidence record (not a substitute for raw)
└── analysis          analyzer_version, derived_from, findings, verdict
```

Re-run a later analyzer on the same `raw`. If the verdict changes, the capture did not.

Live/jsonl ingest may omit `raw`; `derived_from` then uses `capture_digest`.

## EEPROM (not this grain)

Black box = last forensic **evidence record** after FROZEN, journal / two-slot commit, CRC + COMMIT. Not a TRACE ring. Caps bit `PERSISTENCE` means atomic save/restore — **not** “the MCU has EEPROM”.

## Not in v1

Independent FX2 ingest, target UART as finding source, electrical SCK analyzer, plugin API, new firmware events.

## CLI

```bash
./diagplane.bin evidence --demo enableprog_fail_sw
./diagplane.bin analyze --demo pass_with_rst_anomaly --out /tmp/x.usbasp2e --json
```
