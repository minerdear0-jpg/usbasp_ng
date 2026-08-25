/*
 * Edge-triggered ISP line sniffer for a spare ATmega8.
 *
 * Not USBasp NG. Do not flash this onto yellow-dot (DUT) or no-dot
 * (known-good programmer) unless you mean to replace that image.
 *
 * This bench has two clones. A third mega8 is required to sniff while
 * yellow programmes no-dot. Until then: FX2 LA (sigrok) or wait.
 *
 * Clone crystal is 12 MHz (not Arduino 16 MHz). Override with -DF_CPU=.
 *
 * Taps (inputs, pull-ups off, common GND):
 *   PB2 <- programmer RST   PB3 <- MOSI   PB4 <- MISO   PB5 <- SCK
 * UART TX = PD1, 38400 8N1 (U2X @ 12 MHz).
 *
 * Trigger: RST high (arm) then falling edge. Timer1 /8, dump CSV, halt.
 */

#ifndef F_CPU
#define F_CPU 12000000UL
#endif

#include <avr/io.h>
#include <stdint.h>

#define MAX_EVENTS 200
#define PIN_MASK ((1 << PB2) | (1 << PB3) | (1 << PB4) | (1 << PB5))
#define PRESCALE 8u
#define TIMEOUT_TICKS 62000u

typedef struct {
    uint16_t t;
    uint8_t state;
} event_t;

static event_t buf[MAX_EVENTS];

static void uart_init(void)
{
    /* 38400: U2X, UBRR = F_CPU/(8*baud)-1 → 38 @ 12 MHz, 51 @ 16 MHz */
    uint16_t ubrr = (uint16_t)(F_CPU / (8UL * 38400UL) - 1UL);
    UCSRA = (1 << U2X);
    UBRRH = (uint8_t)(ubrr >> 8);
    UBRRL = (uint8_t)ubrr;
    UCSRB = (1 << TXEN);
    UCSRC = (1 << URSEL) | (1 << UCSZ1) | (1 << UCSZ0);
}

static void uart_tx(uint8_t c)
{
    while (!(UCSRA & (1 << UDRE)))
        ;
    UDR = c;
}

static void uart_print(const char *s)
{
    while (*s)
        uart_tx((uint8_t)*s++);
}

static void uart_print_hex16(uint16_t v)
{
    static const char hex[] = "0123456789ABCDEF";
    uart_tx(hex[(v >> 12) & 0xF]);
    uart_tx(hex[(v >> 8) & 0xF]);
    uart_tx(hex[(v >> 4) & 0xF]);
    uart_tx(hex[v & 0xF]);
}

static void uart_print_u32(uint32_t n)
{
    char tmp[11];
    uint8_t p = 0;
    if (n == 0) {
        uart_tx('0');
        return;
    }
    while (n) {
        tmp[p++] = (char)('0' + (n % 10));
        n /= 10;
    }
    while (p)
        uart_tx((uint8_t)tmp[--p]);
}

int main(void)
{
    uint16_t idx;
    uint8_t prev;

    DDRB &= (uint8_t)~PIN_MASK;
    PORTB &= (uint8_t)~PIN_MASK;

    uart_init();
    uart_print("\r\n--- USBasp ISP sniffer ---\r\n");
    uart_print("# F_CPU=");
    uart_print_u32((uint32_t)F_CPU);
    uart_print(" prescale=");
    uart_print_u32(PRESCALE);
    uart_print("\r\n--- waiting for RST high (arm) ---\r\n");

    TCCR1A = 0;
    TCCR1B = (1 << CS11);

    while (!(PINB & (1 << PB2)))
        ;
    uart_print("--- armed, waiting for RST falling edge ---\r\n");

    while (PINB & (1 << PB2))
        ;

    TCNT1 = 0;
    prev = PINB & PIN_MASK;
    idx = 0;
    buf[idx].t = 0;
    buf[idx].state = prev;
    idx++;

    while (idx < MAX_EVENTS) {
        uint8_t cur = PINB & PIN_MASK;
        if (cur != prev) {
            buf[idx].t = TCNT1;
            buf[idx].state = cur;
            idx++;
            prev = cur;
        }
        if (TCNT1 > TIMEOUT_TICKS)
            break;
    }

    uart_print("--- CAPTURE DONE, ");
    uart_print_u32(idx);
    uart_print(" events ---\r\n");
    uart_print("t_hex;RST;MOSI;MISO;SCK\r\n");

    for (uint16_t i = 0; i < idx; i++) {
        uart_print_hex16(buf[i].t);
        uart_tx(';');
        uart_tx((buf[i].state & (1 << PB2)) ? '1' : '0');
        uart_tx(';');
        uart_tx((buf[i].state & (1 << PB3)) ? '1' : '0');
        uart_tx(';');
        uart_tx((buf[i].state & (1 << PB4)) ? '1' : '0');
        uart_tx(';');
        uart_tx((buf[i].state & (1 << PB5)) ? '1' : '0');
        uart_print("\r\n");
    }

    uart_print("--- END (reset sniffer to re-arm) ---\r\n");
    for (;;)
        ;
}
