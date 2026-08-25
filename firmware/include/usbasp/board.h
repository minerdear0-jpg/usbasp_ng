#ifndef USBASP_BOARD_H_
#define USBASP_BOARD_H_

#include <stdint.h>

void board_init(void);
void board_usb_reset_pulse(void);
/* PC1: ISP “TX” (target). Silk “red” on Fischl. */
void board_led_red_on(void);
void board_led_red_off(void);
/* PC0: USB configured / “RX”. Silk “green”; clones often a second red. */
void board_led_green_on(void);
void board_led_green_off(void);
void board_usb_bus_reset(unsigned char resetStarts);
void board_usb_rx_activity(void);
void board_led_isp_activity(void);
void board_led_usb_update(void);

/* 1 if hardware jumper forces slow (~8 kHz) SCK (original J3 / PC2). */
int board_sck_jumper_slow(void);

#endif
