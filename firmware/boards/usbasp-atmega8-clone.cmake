# Fischl-style clone: PORTC LEDs, J3 SCK jumper on PC2.
# Documented 2011 fuses: hfuse=0xc9 lfuse=0xef.
# Measured on yellow-dot clone: hfuse=0xd9 lfuse=0xef. Do not blast hfuse
# with make fuses unless you intend to change CKOPT/boot bits.
set(USBASP_MCU atmega8)
set(USBASP_F_CPU 12000000)
set(USBASP_PROFILE classic)
# TPI FUNC 11–16 are always compiled. GETCAPABILITIES advertises TPI iff USBASP_HAS_TPI=1.
# Not silicon-validated yet; keep 1 for avrdude parity. Set 0 only after a compatibility review.
set(USBASP_HAS_TPI 1)
set(USBASP_HAS_SCK_JUMPER 1)
set(USBASP_HAS_HID_UART 0)
set(USBASP_HAS_3MHZ 1)
set(USBASP_LED_STYLE USBASP_LED_PORT)
set(USBASP_HFUSE 0xc9)
set(USBASP_LFUSE 0xef)
