#include <avr/io.h>
#include <avr/interrupt.h>
#include "usbdrv.h"
#include "usbasp/clock.h"
#include "usbasp/board.h"

int main(void)
{
    clockInit();
    board_init();
    board_usb_reset_pulse();
    usbInit();
    sei();
    for (;;) {
        usbPoll();
        board_led_usb_update();
    }
    return 0;
}
