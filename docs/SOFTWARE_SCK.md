# Software SCK ENABLEPROG

**Status:** **CLOSED** for the lab product path — [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md).

Closing proof (2026-08-26): **USBasp2** (ATmega328P HIDUART, `YEL0`) programming an **ATmega8 soldered on a Nano PCB** — both `-B 8` (HW) and `-B 22` (SW) signature **PASS** (`1E 93 07`).

Earlier the same day, classic **mega8 programmer → mega8 clone target** still showed SW ENABLEPROG `0x01`. That isolation stands as history; it is not treated as an open release blocker. Prefer **USBasp2** for Diagnostics Plane / slow-SCK work.

Optional FX2/scope capture is no longer required to close this **USBasp2** gate. Diagnostics Plane is **firmware truth** (semantic frames the MCU already sampled) — not pin edges. TRACE/FAULT_SNAPSHOT cannot substitute a sniffer when the bug is electrical or in the sample path itself.

**Classic mega8** (`usbasp-atmega8-clone`, `USBASP_HAS_DIAG=0`) remains the Windows daily image. On that programmer, SW SCK vs another mega8 clone is still the historical FAIL above. That is a **known limitation of the shipping classic pair**, not an open USBasp2 acceptance item. Flashing compact mega8 HIDUART to get EP2 TRACE changes USB and the image — it is not the same stick as classic and is **not** the cheap way to reopen that archaeology (HIDUART mega8 is frozen).

## Observer A/B (open — needs a mega8 programmer stick)

`diag_emit_sck_config()` / `diag_note_enableprog_try()` run *inside* `ispEnterProgrammingMode` (RST loop, same PORTB as V-USB INT0). That is back-action, not only timestamp overhead.

Cheap test, **same board recipe**, only the flag:

```bash
# HIDUART mega8 image twice — do not compare classic vs HIDUART and call it this A/B
cmake -DUSBASP_HAS_DIAG=0   # vs default =1 on usbasp-hiduart-atmega8
avrdude -c usbasp -p m8 -B 22 -U signature:r:-:h
```

If PASS/FAIL flips, the instrument perturbs the race (observer effect) and must not be trusted as a witness of that race. Cycle counts for `diag_try_emit` are **unmeasured**; GPIO-toggle + sniffer or `avr-objdump -d` is the calibration. Current lab cage is **USBasp2 328P** (both `-B 22` PASS) — this A/B waits on a mega8 programmer + mega8 target pair.

## Symptom (historical)

`USBASP_FUNC_ENABLEPROG` returned `0x01` on software SCK ids (`-B 22` and slower) with mega8↔mega8 clone pairs. USB stayed enumerated.

## Closing commands

```bash
# USBasp2 in USB, mega8-on-Nano-PCB on ISP ribbon, J2 open on USBasp2
avrdude -c usbasp -p m8 -B 8  -U signature:r:-:h   # PASS
avrdude -c usbasp -p m8 -B 22 -U signature:r:-:h   # PASS
```
