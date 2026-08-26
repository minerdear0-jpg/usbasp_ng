# USBISP-style clones: DDR-driven LEDs, PORTD left as input.
# No SCK jumper in firmware (USBASP_HAS_SCK_JUMPER=0). The PCB may still
# have JP3; it is ignored. Bench no-dot measured ~4018 flash, this profile.
set(USBASP_MCU atmega8)
set(USBASP_F_CPU 12000000)
set(USBASP_PROFILE classic)
# TPI FUNC compiled; capability bit off until silicon proof (see clone.cmake).
set(USBASP_HAS_TPI 0)
set(USBASP_HAS_SCK_JUMPER 0)
set(USBASP_HAS_HID_UART 0)
set(USBASP_HAS_3MHZ 1)
set(USBASP_LED_STYLE USBASP_LED_DDR)
set(USBASP_HFUSE 0xc9)
set(USBASP_LFUSE 0xef)
