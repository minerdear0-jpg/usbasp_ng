# RC1 protocol/FSM track

Two independent tracks (do not merge):

```text
PROTOCOL/FSM                         PHYSICAL SCK
fix now, no hardware                  WAIT FOR CAPTURE
overrun / reset / guards               RST/SCK/MOSI/MISO
```

## Checklist

| ID | Item | Status |
|----|------|--------|
| RC1-01 | `prog_reset_state()` on CONNECT/DISCONNECT | **done** (`4321302`) |
| RC1-02 | Data-stage write overrun (FLASH nbytes→IDLE must not write EEPROM) | **done** (this change) |
| RC1-03 | Zero-length transfer guards (`prog_begin_transfer`) | **done** (`4321302`) |
| RC1-04 | EEPROM API `uint16_t` | **done** (`4321302`) |
| RC1-05 | TPI capability off until silicon (`USBASP_HAS_TPI=0`) | **done** (`4321302`) |
| RC1-06 | SCK capture `-B 8` vs `-B 22` | **wait** — [ACCEPTANCE-SCK-SWEEP-001](ACCEPTANCE-SCK-SWEEP-001.md) |
| RC1-07 | SW SCK fix | **only after** waveform |
| RC1-08 | Regression acceptance | after RC1-02 flash + HW ISP smoke |

## RC1-02 detail

`usbasp_isp_write()` used `if (WRITEFLASH) … else EEPROM`. When `prog_nbytes` hit 0 mid-packet, state became IDLE and the **same** USB OUT packet continued in the `else` → EEPROM writes. Fix: `else if (WRITEEEPROM)` + `break` when nbytes exhausted.

Tests: `firmware/tests/core/test_prog_session.py`.

## Out of this track

HIDUART **Diagnostics Plane** (binary telemetry / TRACE) is a separate post-RC1 research design: [../DIAGNOSTICS.md](../DIAGNOSTICS.md). Classic stays telemetry-free. Does not replace SCK capture.
