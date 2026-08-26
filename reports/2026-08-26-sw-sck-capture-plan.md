# SW SCK — RST race ruled out; capture checklist

**Accepted:** 2026-08-26  
**Bench:** yellow-dot HIDUART `YEL0` in USB, no-dot target `1E 93 07`, JP3 open. After atomic RST wrap: `-B 8` / `-B 0.5` PASS, `-B 22` ENABLEPROG `0x01`. GETCAPABILITIES `01 00 00 01`.

RST-race is **not** the sole cause of FAIL. The wrap stays (same PORTB RMW rule as MOSI/SCK). USB vendor path is healthy; only the ISP handshake on software SCK fails.

No remaining cheap software hypothesis without a waveform. Capture `-B 8` (PASS) vs `-B 22` (FAIL) and score these five questions. Dump both logs / `plot_capture.py --no-plot` CSV.

## 1. RST — same shape on PASS and FAIL?

One clean falling edge, no bounce / extra pulse, same amplitude. Difference here → not SCK; RST timing vs `clockWait(62)` on the slow path.

## 2. MOSI — is `AC 53 00 00` correct on `-B 22` even if the reply is wrong?

Same bit sequence as PASS, only period different → not command TX; look at RX or the target. Extra/missing bits or ragged levels only on `-B 22` → sw-bitbang path, not the target.

## 3. MISO during the third byte

| What | Next |
|------|------|
| Stable 0 for the whole third byte | Target not in ISP/reset (RST in §1) |
| Toggles but not `0x53` | Target answers, sample phase vs SCK edges |
| Smooth but wrong byte | Bit/byte slip at slow SCK |

## 4. SCK period / duty at `-B 22`

Match ~16 µs half-period? Glitches at `cli`/`SREG` boundaries? V-USB ISR should be honest RMW; still look.

## 5. HW SPI not stuck in software mode

SCK/MOSI must move as bitbang, not frozen (SPCR leftover). Unlikely across separate avrdude runs (`ispSetSCKOption` each time); free check on the same traces.

## Commands

Same as [`host/isp-sniffer/README.md`](../host/isp-sniffer/README.md):

```text
avrdude -c usbasp -p atmega8 -B 8  -U signature:r:-:h    # PASS
avrdude -c usbasp -p atmega8 -B 22 -U signature:r:-:h    # FAIL
```

Need a third mega8 sniffer or FX2/`fx2lafw`. Do not flash the sniffer onto no-dot or yellow unless replacing NG.
