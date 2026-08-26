# USBasp2 beta.1

First public beta of **USBasp2**: a USBasp fork with an integrated laboratory instrument (L1 avrdude + Diagnostics Plane). Tag: `usbasp2-beta.1`.

```text
USBasp (Fischl)  →  USBasp NG classic (Windows daily)
                 →  USBasp2 beta = 328P + HIDUART + Diagnostics Plane (lab)
```

## Product claim (freeze)

| Claim | Meaning |
|-------|---------|
| **L1 unchanged** | `avrdude -c usbasp` / `usbasp-clone` on Linux programs targets as ordinary USBasp |
| **Lab instrument** | EP2 semantic timeline + host `diagplane` (watch / record / decode / correlate) |
| **Platform** | ATmega328P HIDUART board `usbasp-hiduart-atmega328p` (`bcdDevice` 2.01) |

See [USBASP2.md](USBASP2.md) and [DIAGNOSTICS_PROBE.md](DIAGNOSTICS_PROBE.md).

## Beta.1 IN

| Area | Evidence / note |
|------|-----------------|
| SW SCK | [ACCEPTANCE-SCK-SWEEP-001](acceptance/ACCEPTANCE-SCK-SWEEP-001.md) |
| CAPS / TRACE / TRIGGER | CAPS `0x3f`, trigger A+B; [ACCEPTANCE-DIAG-TRIGGER-001](acceptance/ACCEPTANCE-DIAG-TRIGGER-001.md) |
| MEMOP grains | PAGE@addr, READFLASH coalesce, subsample, END pages=N |
| ISP_PINS Hi-Z | after disconnect |
| Ring | `TRACE_SLOTS=128`, live overflow=no @ pages=89 |
| Dual-truth | oracle + mangle/fault; `diagplane correlate` |
| Host | `diagplane.bin` — watch with diagnosis/phases/instruments + dual-column when `--uart` |

## Beta.1 OUT

- FX2 / PHYSICAL_CAPTURE
- Windows HIDUART ISP programming
- TPI advertise
- Optiboot + oracle canary on the same hex / same flash ownership of `0x1E00`
- mega8/88 HIDUART full lab grains (compact plane only; use **USBasp2 328P** for trigger / MEMOP PAGE / ISP_PINS)

## Cage memory (USBasp2 → Канарейка)

L1 on the ribbon is ordinary avrdude. beta.1 smoke (YEL0, `-p m8 -B 8`, **reads**):

| Memory | Read | Write |
|--------|------|-------|
| flash (8 KiB) | yes | yes (`make flash` / hex); do not mix Optiboot + oracle canary |
| eeprom (512 B) | yes | yes (lab: write + verify + restore PASS) |
| lfuse / hfuse / lock | yes | possible; **do not blast** — oracle costume is `hfuse=0xC5` |

Asciinema (TUI + those reads): [`docs/media/demo-diagplane-beta1.cast`](../docs/media/demo-diagplane-beta1.cast) — `asciinema play docs/media/demo-diagplane-beta1.cast`.

## Smoke checklist

Lab stick serial often `YEL0` (local override); release EEPROM uses `0000`.

```bash
python3 host/usbaspctl.py info          # hiduart (YEL0 or pack serial)
avrdude -c usbasp -P usb:YEL0 -p m8 -B 8 -U signature:r:-:h
./dist/diagplane.bin capabilities --serial YEL0
# record + harness → TRACE_BEGIN slots=128, overflow=no, PASS
./dist/diagplane.bin correlate --diag … --uart …
```

## Known limits

- Dual-truth is a **lab method** (oracle UART + EP2), not a Windows product claim.
- Dual-column watch is in beta.1 (`watch --uart` / `--diag`+`--uart`); further polish may follow.
- No bcdDevice bump beyond HIDUART 2.01; no DIAG v2 wire in this tag.

## ATmega8 HIDUART — frozen after beta.1

Fighting the mega8 flash/RAM wall for full Diagnostics Plane grains is **not** a USBasp2 priority.

| Line | After beta.1 |
|------|----------------|
| **Classic** (`usbasp-atmega8-clone` / 88) | Kept — Windows / Arduino daily ISP |
| **USBasp2 instrument** | **ATmega328P** + HIDUART + Diagplane only |
| mega8/88 HIDUART | Shipped once in **beta.1** (compact plane); left in tree for anyone who wants to optimize; **no further product attention** |

Next USBasp2-oriented packs ship **classic** + **328P Diagplane** (+ `diagplane.bin`), not mega8 HIDUART as a lab claim.

## Release assets

Built with:

```bash
./scripts/pack-release.sh usbasp2-beta.1 --hex --diag
```

| Asset | Role |
|-------|------|
| `usbasp-ng-src-vusbasp2-beta.1.zip` | Source archive of tagged tree |
| `usbasp-ng-classic-atmega8.hex` | Classic mega8 |
| `usbasp-ng-classic-atmega88.hex` | Classic mega88 |
| `usbasp-ng-hiduart-atmega8.hex` + `.eep` | mega8 HIDUART (**frozen** compact; last intentional ship) |
| `usbasp-ng-hiduart-atmega88.hex` + `.eep` | mega88 HIDUART (**frozen** compact; last intentional ship) |
| `usbasp-ng-hiduart-atmega328p.hex` + `.eep` | **USBasp2** lab image (`SERIAL=0000` in pack) |
| `diagplane.bin` | Linux x86-64 host client |

Flash USBasp2 from a classic stick (J2 closed on the 328P board), then open J2 and put the target on the ribbon.
