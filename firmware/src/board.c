#include <avr/io.h>
#include "usbasp_config.h"
#include "usbasp/board.h"
#include "usbasp/clock.h"

#if USBASP_LED_STYLE == USBASP_LED_PORT
/* Fischl 2011: LEDs active-low on PORTC, DDRC already outputs. */
#define ledRedOn()    PORTC &= ~(1 << PC1)
#define ledRedOff()   PORTC |= (1 << PC1)
#define ledGreenOn()  PORTC &= ~(1 << PC0)
#define ledGreenOff() PORTC |= (1 << PC0)
#elif USBASP_LED_STYLE == USBASP_LED_DDR
/* USBISP clones: drive by enabling the pin as output (open-drain style). */
#define ledRedOff()   DDRC &= ~(1 << PC1)
#define ledRedOn()    DDRC |= (1 << PC1)
#define ledGreenOff() DDRC &= ~(1 << PC0)
#define ledGreenOn()  DDRC |= (1 << PC0)
#else
#error "USBASP_LED_STYLE"
#endif

void board_init(void)
{
    /* Do not drive PORTD: USBISP clones share that port (nerdralph). */
    PORTB = 0;

#if USBASP_LED_STYLE == USBASP_LED_PORT
    DDRC = 0x03;
    PORTC = 0xfe;
#else
    DDRC = 0;
    PORTC = 0xfe;
#endif
}

void board_usb_reset_pulse(void)
{
    /* SE0 on USB D+/D- (PB0/PB1) for >10 ms */
    DDRB = ~0;
    clockWait(31); /* ~10 ms at 320 us ticks */
    DDRB = 0;
}

void board_led_red_on(void) { ledRedOn(); }
void board_led_red_off(void) { ledRedOff(); }
void board_led_green_on(void) { ledGreenOn(); }
void board_led_green_off(void) { ledGreenOff(); }

int board_sck_jumper_slow(void)
{
#if USBASP_HAS_SCK_JUMPER
    return (PINC & (1 << PC2)) == 0;
#else
    return 0;
#endif
}
