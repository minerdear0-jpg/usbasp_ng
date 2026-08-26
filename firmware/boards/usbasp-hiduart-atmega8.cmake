# Composite + compact Diagnostics Plane on mega8.
# FROZEN after usbasp2-beta.1: do not expand lab grains here — use
# usbasp-hiduart-atmega328p (USBasp2). Board kept for optional community size work.
set(USBASP_MCU atmega8)
set(USBASP_F_CPU 12000000)
set(USBASP_PROFILE hiduart)
# TPI: not advertised. See clone board comment.
set(USBASP_HAS_TPI 0)
set(USBASP_HAS_SCK_JUMPER 1)
set(USBASP_HAS_HID_UART 1)
set(USBASP_HAS_DIAG 1)
set(USBASP_HAS_3MHZ 1)
set(USBASP_LED_STYLE USBASP_LED_PORT)
set(USBASP_HFUSE 0xc9)
set(USBASP_LFUSE 0xef)
# Compact Diagnostics Plane: no trigger engine / MEMOP page grains (flash wall).
set(USBASP_DIAG_TRACE_SLOTS 32)
