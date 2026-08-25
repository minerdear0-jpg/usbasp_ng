#include <avr/io.h>
#include <avr/interrupt.h>
#include "usbdrv.h"
#include "usbasp/clock.h"
#include "usbasp/board.h"
#include "hiduart.h"

int main(void)
{
    clockInit();
    board_init();
    board_usb_reset_pulse();
    board_led_green_on();
    usbInit();
    sei();
    for (;;) {
        hiduart_poll();
        usbPoll();
    }
    return 0;
}
