# Diagnostic Evidence Record (host v1)

USBasp2 does not promise to know the cause. It collects **evidence sufficient to test a hypothesis**.

This is a **host container**, not a new EP2 telemetry type. Wire stays [DIAGNOSTICS.md](DIAGNOSTICS.md). Probe honesty stays [DIAGNOSTICS_PROBE.md](DIAGNOSTICS_PROBE.md).

```text
OBSERVE (RAM TRACE / EP2)     →  HOST RECORD
FROZEN snapshot               →  HOST VERDICT (this file)
optional EEPROM later         →  PERSISTENT_EVIDENCE (not shipped)
external sniffer              →  PHYSICAL_CAPTURE
```

`diagplane evidence` builds the record from `AppState`. `snapshot` remains the flat instrument dump.

## Schema

`schema = 1`

| Block | Purpose |
|-------|---------|
| identity | `session_id`, `capture_id` (digest), HELLO schema/profile, CAPS masks |
| configuration | SCK **id** + HW/SW (not Hz), diag bits. Hz / jumper live state: not on wire |
| target | signature **only** if a tagged source supplies it. EP2 does not emit `1E 93 07` |
| execution | RESET assert/release, ENABLEPROG attempt count, TRACE summary |
| claims | expected / observed / verdict / `evidence=protocol` / confidence |
| result | observation vs interpretation vs `cannot_prove` |
| integrity | `protocol_observed`, `physical_capture`, `persistent_evidence`, digest |
| provenance | diagplane version, protocol version, session complete |

`firmware_build_id` is **null** until firmware advertises a hash. Do not invent it from `bcdDevice`.

Signature sources (when present): `ISP` (host avrdude), `target_uart`, `USBASP_OBSERVATION`, `PHYSICAL_CAPTURE`. Mixing them without a tag is a bug.

## EEPROM (not this grain)

Black box = last forensic record after FROZEN, 1–4 slots, CRC + COMMIT byte, sequence number. Not a TRACE ring. Caps bit `PERSISTENCE` means atomic save/restore of that record — **not** “the MCU has EEPROM”. Policy: OFF / LAST_FAILURE / LAST_SESSION. Host Evidence v1 first.

## Analysis (host, not firmware)

Decoder is dumb: frames → Evidence. Analyzers (`isp`, `reset`, `sck`, `session`) emit Findings with domain, scope, confidence, and causal relevance. The correlator produces a session Verdict. It does not live in the 328P.

A LINE_FAULT observation is not a session FAIL. Golden: `diagplane analyze --demo pass_with_rst_anomaly` → `PASS_WITH_ANOMALY`. LINE-only → `INCONCLUSIVE`. ENABLEPROG FAIL without physical capture → `FAIL_UNCONFIRMED`.

```bash
diagplane analyze --demo pass_with_rst_anomaly
diagplane analyze --demo enableprog_fail_sw
diagplane analyze --file capture.bin --out session.usbasp2e
```

`.usbasp2e` = JSON `{ format, evidence, analysis }`. Improve the analyzer later without flashing the stick.

SCK Hz / FX2 analyzers wait on `PHYSICAL_CAPTURE` and timestamps-as-period — not this grain.

## CLI

```bash
./diagplane.bin evidence --demo enableprog_fail_sw
./diagplane.bin evidence --demo enableprog_ladder_silent --json
```
