set(USBASP_MCU atmega88)
set(USBASP_F_CPU 12000000)
set(USBASP_PROFILE hiduart)
# Composite + Diagnostics Plane (USBASP_HAS_DIAG). See clone hiduart board comment.
set(USBASP_HAS_TPI 0)
set(USBASP_HAS_SCK_JUMPER 1)
set(USBASP_HAS_HID_UART 1)
set(USBASP_HAS_DIAG 1)
set(USBASP_HAS_3MHZ 1)
set(USBASP_LED_STYLE USBASP_LED_PORT)
set(USBASP_HFUSE 0xdd)
set(USBASP_LFUSE 0xff)
# Compact Diagnostics Plane: no trigger engine / MEMOP page grains (flash wall).
set(USBASP_DIAG_TRACE_SLOTS 32)
