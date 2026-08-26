# Yellow-dot / **USBasp2**: ATmega328P-AU reflowed onto the clone TQFP-32 footprint.
# Same 12 MHz crystal / USB / ISP wiring as mega8. Not a release default for Windows.
# Product name: USBasp2 — see docs/USBASP2.md
# Factory chip often has CKDIV8 (1 MHz RC) until fuses are set — use slow -B first.
set(USBASP_MCU atmega328p)
set(USBASP_F_CPU 12000000)
set(USBASP_PROFILE classic)
set(USBASP_HAS_TPI 0)
set(USBASP_HAS_SCK_JUMPER 1)
set(USBASP_HAS_HID_UART 0)
set(USBASP_HAS_DIAG 0)
set(USBASP_HAS_3MHZ 1)
set(USBASP_LED_STYLE USBASP_LED_PORT)
# Same crystal recipe as mega88 USBasp (full-swing ext crystal, SPIEN, no BOOTRST).
set(USBASP_HFUSE 0xde)
set(USBASP_LFUSE 0xff)
# efuse BOD disabled (0xff) — set manually: avrdude -U efuse:w:0xff:m
