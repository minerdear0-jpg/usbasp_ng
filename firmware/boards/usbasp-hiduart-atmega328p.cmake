# **USBasp2** — HIDUART + Diagnostics Plane on ATmega328P (reflow on mega8 clone PCB).
# Headroom: 32 KiB flash / 2 KiB SRAM vs mega8's 8 KiB wall. See docs/USBASP2.md
set(USBASP_MCU atmega328p)
set(USBASP_F_CPU 12000000)
set(USBASP_PROFILE hiduart)
set(USBASP_HAS_TPI 0)
set(USBASP_HAS_SCK_JUMPER 1)
set(USBASP_HAS_HID_UART 1)
set(USBASP_HAS_DIAG 1)
set(USBASP_HAS_3MHZ 1)
set(USBASP_LED_STYLE USBASP_LED_PORT)
set(USBASP_HFUSE 0xde)
set(USBASP_LFUSE 0xff)
