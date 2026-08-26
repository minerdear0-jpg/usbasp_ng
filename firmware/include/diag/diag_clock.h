#ifndef USBASP_DIAG_CLOCK_H_
#define USBASP_DIAG_CLOCK_H_

#include <stdint.h>

/*
 * Diagnostics Plane monotonic clock (Timer1).
 *
 * - Source: Timer1 normal mode, prescaler /8 (not the ISP clockWait path).
 * - At F_CPU=12 MHz: tick ≈ 0.667 µs; 16-bit period ≈ 43.69 ms.
 * - Soft high halfword from TOV1, cleared lazily in diag_now() — no overflow ISR.
 * - Call diag_now() at least once per Timer1 period while time must stay continuous
 *   (hiduart_poll → diag_poll_drain already does).
 *
 * Wire P0/RC still carries uint16 (low 16 bits). Full uint32 is for firmware /
 * future DIAG v2 — do not put 32-bit T on EP2 yet.
 */

typedef uint32_t diag_tick_t;

void diag_clock_init(void);

/* Monotonic firmware tick (wraps after ~39.8 min at 12 MHz /8). */
diag_tick_t diag_now(void);

/* Unsigned elapsed ticks; correct across wrap of the 32-bit value. */
diag_tick_t diag_elapsed(diag_tick_t start, diag_tick_t end);

/* Low 16 bits for P0/RC wire frames. */
static inline uint16_t diag_now_wire16(void)
{
    return (uint16_t)diag_now();
}

#endif /* USBASP_DIAG_CLOCK_H_ */
