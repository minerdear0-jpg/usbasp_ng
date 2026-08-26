#include <avr/io.h>
#include <avr/interrupt.h>
#include <avr/wdt.h>
#include "usbdrv.h"
#include "usbasp/clock.h"
#include "usbasp/board.h"
#include "usbasp_config.h"
#include "hiduart.h"
#if USBASP_HAS_DIAG
#include "diag/diag_clock.h"
#endif

int main(void)
{
    MCUSR = 0;
    wdt_disable();
    clockInit();
#if USBASP_HAS_DIAG
    diag_clock_init();
#endif
    board_init();
    board_usb_reset_pulse();
    usbInit();
    sei();
    for (;;) {
        hiduart_poll();
        usbPoll();
        board_led_usb_update();
    }
    return 0;
}
