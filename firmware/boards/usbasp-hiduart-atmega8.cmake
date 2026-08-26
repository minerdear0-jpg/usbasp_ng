# Separate product: composite WCID + HID. Research/dev image with Diagnostics Plane
# (USBASP_HAS_DIAG). Not the Windows/Arduino default — use classic for that.
set(USBASP_MCU atmega8)
set(USBASP_F_CPU 12000000)
set(USBASP_PROFILE hiduart)
# TPI: advertised; not silicon-validated yet. See clone board comment.
set(USBASP_HAS_TPI 0)
set(USBASP_HAS_SCK_JUMPER 1)
set(USBASP_HAS_HID_UART 1)
set(USBASP_HAS_DIAG 1)
set(USBASP_HAS_3MHZ 1)
set(USBASP_LED_STYLE USBASP_LED_PORT)
set(USBASP_HFUSE 0xc9)
set(USBASP_LFUSE 0xef)
