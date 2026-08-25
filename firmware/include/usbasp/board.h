#ifndef USBASP_BOARD_H_
#define USBASP_BOARD_H_

#include <stdint.h>

void board_init(void);
void board_usb_reset_pulse(void);
void board_led_red_on(void);
void board_led_red_off(void);
void board_led_green_on(void);
void board_led_green_off(void);

/* 1 if hardware jumper forces slow (~8 kHz) SCK (original J3 / PC2). */
int board_sck_jumper_slow(void);

#endif
