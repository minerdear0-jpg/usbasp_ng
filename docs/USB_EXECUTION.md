# USB execution model (V-USB)

Architectural invariant for classic and HIDUART. Not an avrdude wire change.

```text
INT0 ISR
    ↓
USB bit clock / V-USB event handling only
    ↓
return
    ↓
main → usbPoll()
    ↓
usbProcessRx() / usbFunctionSetup()   ← runs with I=1
    ↓
USBasp FUNC (CONNECT, TRANSMIT, SETISPSCK, …)
    ↓
ISP / TPI session code
```

## Rules

1. **INT0 must not run the USBasp protocol.** Setup and data stages run from `usbPoll()` in main context.
2. **ISP may be preempted by INT0.** PORTB RMW (MOSI / SCK / RST) uses `cli` / restore `SREG` against V-USB `in`/`ori`/`out` on the same port.
3. **Software SCK delays tolerate ISR stretch.** Half-period is a **minimum**; INT0 must not shorten a phase. See L2.5 in [COMPATIBILITY.md](COMPATIBILITY.md).
4. **LED and USB bookkeeping stay out of `ispTransmit_sw` / `ispTransmit_hw`.** Kick activity at the FUNC / operation level (`vendor_isp`, ENABLEPROG), not per SPI byte on the timing path.
5. **Diagnostics (HIDUART only, when present)** may only `diag_try_emit()` into a RAM ring from ISP; HID drain runs from poll/main. Never block ISP on telemetry. Design: [DIAGNOSTICS.md](DIAGNOSTICS.md).

## Where it lives in code

- Classic setup: `firmware/src/usb_setup.c`
- HIDUART setup: `firmware/src_hid/usb_setup.c`
- Comment on delays: `firmware/src/sck.c` → `isp_sck_delay()`
- PORTB helpers: `isp_out_set_bit` / `isp_out_clr_bit` in `firmware/src/isp.c`
