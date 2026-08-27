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

Semantics **evidence-v1** is **frozen**. Do not change the meaning of findings, causal relevance, or the five verdicts without a schema bump. New work is new sources and tests, not a quieter FAIL.

LINE GPIO anomaly + ENABLEPROG PASS + MEMOP PASS is the regression of the whole model (`PASS_WITH_ANOMALY`), not a special case to suppress.

**PASS is a session outcome, not “ENABLEPROG happened”.** If MEMOP START/CONT is seen without MEMOP END, verdict is `FAIL_UNCONFIRMED` (`ISP.MEMOP_INCOMPLETE`). CONT OK pages do not mean the write finished. If any FLASH CONT|FAIL was seen, that failure is **sticky** across a following READFLASH MEMOP — `MEMOP END … OK` does not wash it into PASS (ribbon tear mid-write). If the host timeline shows a **multi-second gap mid FLASH/EEPROM write** and firmware still emits `MEMOP END|OK`, that stall is **sticky** (`ISP.MEMOP_STALL`) — END after a USB drop is not success. avrdude verify-mismatch is still not on EP2 — but poll FAIL and stall *are*.

```text
raw evidence → observations → findings → claims → correlation → verdict
```

Analyzers must not compute the session verdict. The correlator must not rewrite raw.

**Next (not this freeze):** replay corpus + determinism (host tests), then a **recorded** independent physical capture for `EVIDENCE.CONFLICT`, then firmware `build_id` on the wire, then EEPROM using this same schema. Not a new EEPROM format. Not another ISP analyzer.

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

## Evidence hierarchy ≠ source hierarchy

Do not rank FX2 above firmware or firmware above FX2. A source has a **scope of observation**, not a privilege to declare truth.

| Source | Scope |
|--------|--------|
| USBASP_INTERNAL | what firmware attempted / sampled at the programmer MCU |
| PHYSICAL_CAPTURE | what a probe saw on a line |
| HOST_PROTOCOL | what the host requested |
| TARGET_UART | what the target reported |

`firmware: RESET HIGH` vs `FX2: wire LOW` is a **conflict of scopes**. Possible causes include GPIO setup, pin map, electrical fault, capture error, or clock alignment. Diagplane records `EVIDENCE.CONFLICT` and does **not** pick a culprit.

Next experiment (firmware frozen): [PHYSICAL-CAPTURE-001](acceptance/PHYSICAL-CAPTURE-001.md) — first a normal `-B 8` signature with simultaneous Diagplane `.bin` and FX2, then a real conflict. Not an FX2 analyzer. Not EEPROM.

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

FX2 analyzer, target UART as finding source, electrical SCK analyzer, plugin API, new firmware events. First dual recording: [PHYSICAL-CAPTURE-001](acceptance/PHYSICAL-CAPTURE-001.md).

## CLI

```bash
./diagplane.bin evidence --demo enableprog_fail_sw
./diagplane.bin analyze --demo pass_with_rst_anomaly --out /tmp/x.usbasp2e --json
```
