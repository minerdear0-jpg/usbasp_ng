#include "usbasp_config.h"

#if USBASP_HAS_DIAG

#include "diag/diag_clock.h"

#include <avr/io.h>
#include <avr/interrupt.h>

/*
 * Soft extension of TCNT1 without a Timer1 overflow ISR (keeps V-USB model clean).
 * TOV1 is sticky; we clear it and bump epoch only inside diag_now() under cli.
 */

static uint16_t diag_clock_epoch;

void diag_clock_init(void)
{
    uint8_t sreg = SREG;
    cli();

    TCCR1A = 0;
    /* Normal mode, clk/8 — same on mega8 / mega88 / mega328P. */
    TCCR1B = (1 << CS11);
    TCNT1 = 0;
    diag_clock_epoch = 0;

#if defined(TIFR1)
    TIFR1 = (1 << TOV1);
#else
    TIFR = (1 << TOV1);
#endif

#if defined(TIMSK1)
    TIMSK1 &= (uint8_t)~(1 << TOIE1);
#elif defined(TIMSK)
    TIMSK &= (uint8_t)~(1 << TOIE1);
#endif

    SREG = sreg;
}

diag_tick_t diag_now(void)
{
    uint16_t cnt;
    uint16_t epoch;
    uint8_t sreg = SREG;

    cli();
    cnt = TCNT1;
#if defined(TIFR1)
    if (TIFR1 & (1 << TOV1)) {
        TIFR1 = (1 << TOV1);
        cnt = TCNT1;
        diag_clock_epoch++;
    }
#else
    if (TIFR & (1 << TOV1)) {
        TIFR = (1 << TOV1);
        cnt = TCNT1;
        diag_clock_epoch++;
    }
#endif
    epoch = diag_clock_epoch;
    SREG = sreg;

    return ((diag_tick_t)epoch << 16) | (diag_tick_t)cnt;
}

diag_tick_t diag_elapsed(diag_tick_t start, diag_tick_t end)
{
    return end - start;
}

#endif /* USBASP_HAS_DIAG */
