#include <inttypes.h>
#include <avr/io.h>
#include "usbasp/clock.h"

void clockWait(uint8_t time)
{
    do {
        uint8_t starttime = TIMERVALUE;
        while ((uint8_t)(TIMERVALUE - starttime) < CLOCK_T_320us)
            ;
    } while (--time);
}
