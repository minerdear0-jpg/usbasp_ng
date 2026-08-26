/* Closed-loop smoke for ATmega8 on a Nano-style PCB.
 * Clock: 16 MHz crystal (stock Nano). UART0 @ 115200 → CH340 → /dev/ttyUSB0.
 * LEDs: try D13 (L) plus D2/D3/D4 — whichever are wired will show a chase.
 */
#include <avr/io.h>
#include <avr/interrupt.h>
#include <util/delay.h>
#include <stdint.h>

#ifndef F_CPU
#define F_CPU 16000000UL
#endif

#define BAUD 115200UL
/* U2X mode: UBRR = F_CPU / (8 * BAUD) - 1  → 16 at 16 MHz / 115200 */
#define UBRR_VAL ((F_CPU / 8 / BAUD) - 1)

static void uart_init(void)
{
    UCSRA = (1 << U2X);
    UBRRH = (uint8_t)(UBRR_VAL >> 8);
    UBRRL = (uint8_t)UBRR_VAL;
    UCSRB = (1 << TXEN) | (1 << RXEN);
    UCSRC = (1 << URSEL) | (1 << UCSZ1) | (1 << UCSZ0); /* 8N1 */
}

static void uart_putc(char c)
{
    while (!(UCSRA & (1 << UDRE)))
        ;
    UDR = c;
}

static void uart_puts(const char *s)
{
    while (*s)
        uart_putc(*s++);
}

static void leds_init(void)
{
    /* Nano L = D13 = PB5; extras D2/D3/D4 = PD2/PD3/PD4 */
    DDRB |= (1 << PB5);
    DDRD |= (1 << PD2) | (1 << PD3) | (1 << PD4);
    PORTB &= ~(1 << PB5);
    PORTD &= ~((1 << PD2) | (1 << PD3) | (1 << PD4));
}

static void leds_set(uint8_t mask)
{
    if (mask & 1)
        PORTB |= (1 << PB5);
    else
        PORTB &= ~(1 << PB5);
    if (mask & 2)
        PORTD |= (1 << PD2);
    else
        PORTD &= ~(1 << PD2);
    if (mask & 4)
        PORTD |= (1 << PD3);
    else
        PORTD &= ~(1 << PD3);
    if (mask & 8)
        PORTD |= (1 << PD4);
    else
        PORTD &= ~(1 << PD4);
}

int main(void)
{
    uint8_t step = 0;
    uint16_t ticks = 0;

    leds_init();
    uart_init();
    sei();

    uart_puts("\r\nUSBasp NG closed-loop\r\n");
    uart_puts("target=ATmega8-on-Nano F_CPU=16MHz\r\n");
    uart_puts("press RESET for restart banner\r\n");

    for (;;) {
        leds_set(1u << (step & 3));
        step++;
        _delay_ms(120);
        if (++ticks >= 25) { /* ~3 s */
            ticks = 0;
            uart_puts("ping LEDs+UART ok\r\n");
        }
    }
}
