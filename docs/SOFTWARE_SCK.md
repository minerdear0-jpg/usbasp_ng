# Software SCK ENABLEPROG

**Status:** **CLOSED** for the lab product path — [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md).

Closing proof (2026-08-26): **USBasp2** (ATmega328P HIDUART, `YEL0`) programming an **ATmega8 soldered on a Nano PCB** — both `-B 8` (HW) and `-B 22` (SW) signature **PASS** (`1E 93 07`).

Earlier the same day, classic **mega8 programmer → mega8 clone target** still showed SW ENABLEPROG `0x01`. That isolation stands as history; it is not treated as an open release blocker. Prefer **USBasp2** for Diagnostics Plane / slow-SCK work.

Optional FX2/scope capture is no longer required to close this gate. Diagnostics Plane remains useful telemetry, not a substitute investigation: [DIAGNOSTICS.md](DIAGNOSTICS.md), [USBASP2.md](USBASP2.md).

## Symptom (historical)

`USBASP_FUNC_ENABLEPROG` returned `0x01` on software SCK ids (`-B 22` and slower) with mega8↔mega8 clone pairs. USB stayed enumerated.

## Closing commands

```bash
# USBasp2 in USB, mega8-on-Nano-PCB on ISP ribbon, J2 open on USBasp2
avrdude -c usbasp -p m8 -B 8  -U signature:r:-:h   # PASS
avrdude -c usbasp -p m8 -B 22 -U signature:r:-:h   # PASS
```
