/*
 * Edge-triggered ISP line sniffer.
 *
 * Not USBasp NG. Do not flash onto yellow-dot or no-dot unless replacing NG.
 *
 * Boards:
 *   ATmega8 clone  — 12 MHz, UART PD1, leave USB unplugged (PB0/PB1 = D-/D+).
 *   Arduino Nano   — ATmega328P 16 MHz, USB-UART is PD0/PD1 (keep USB plugged).
 *
 * Taps (inputs, pull-ups off, common GND only — do not share 5V with the
 * programmer if both are USB-powered):
 *   PB2 RST   PB3 MOSI   PB4 MISO   PB5 SCK
 * Nano: D10 D11 D12 D13 (D13 LED sits on SCK; light load, usually OK).
 *
 * Trigger: RST high (arm) then falling edge. Timer1 /8, dump CSV, halt.
 */

#ifndef F_CPU
#if defined(__AVR_ATmega328P__) || defined(__AVR_ATmega328__)
#define F_CPU 16000000UL
#else
#define F_CPU 12000000UL
#endif
#endif

#include <avr/io.h>
#include <avr/wdt.h>
#include <stdint.h>

#if defined(__AVR_ATmega328P__) || defined(__AVR_ATmega328__)
#define MAX_EVENTS 480
#define UART_UCSRA UCSR0A
#define UART_UCSRB UCSR0B
#define UART_UCSRC UCSR0C
#define UART_UBRRH UBRR0H
#define UART_UBRRL UBRR0L
#define UART_UDR UDR0
#define UART_U2X U2X0
#define UART_TXEN TXEN0
#define UART_UDRE UDRE0
#define UART_UCSZ0 UCSZ00
#define UART_UCSZ1 UCSZ01
#define UART_UCSRC_EXTRA 0
#else
#define MAX_EVENTS 200
#define UART_UCSRA UCSRA
#define UART_UCSRB UCSRB
#define UART_UCSRC UCSRC
#define UART_UBRRH UBRRH
#define UART_UBRRL UBRRL
#define UART_UDR UDR
#define UART_U2X U2X
#define UART_TXEN TXEN
#define UART_UDRE UDRE
#define UART_UCSZ0 UCSZ0
#define UART_UCSZ1 UCSZ1
#define UART_UCSRC_EXTRA (1 << URSEL)
#endif

#define PIN_MASK ((1 << PB2) | (1 << PB3) | (1 << PB4) | (1 << PB5))
#define PRESCALE 8u
#define TIMEOUT_TICKS 62000u

typedef struct {
    uint16_t t;
    uint8_t state;
} __attribute__((packed)) event_t;

static event_t buf[MAX_EVENTS];

static void uart_init(void)
{
    /* 38400: U2X, UBRR = F_CPU/(8*baud)-1 → 38 @ 12 MHz, 51 @ 16 MHz */
    uint16_t ubrr = (uint16_t)(F_CPU / (8UL * 38400UL) - 1UL);
    UART_UCSRA = (1 << UART_U2X);
    UART_UBRRH = (uint8_t)(ubrr >> 8);
    UART_UBRRL = (uint8_t)ubrr;
    UART_UCSRB = (1 << UART_TXEN);
    UART_UCSRC = (uint8_t)(UART_UCSRC_EXTRA | (1 << UART_UCSZ1) | (1 << UART_UCSZ0));
}

static void uart_tx(uint8_t c)
{
    while (!(UART_UCSRA & (1 << UART_UDRE)))
        ;
    UART_UDR = c;
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

    MCUSR = 0;
    wdt_disable();

    DDRB &= (uint8_t)~PIN_MASK;
    PORTB &= (uint8_t)~PIN_MASK;

    uart_init();
    uart_print("\r\n--- USBasp ISP sniffer ---\r\n");
#if defined(__AVR_ATmega328P__) || defined(__AVR_ATmega328__)
    uart_print("# board=nano328p\r\n");
#else
    uart_print("# board=atmega8\r\n");
#endif
    uart_print("# F_CPU=");
    uart_print_u32((uint32_t)F_CPU);
    uart_print(" prescale=");
    uart_print_u32(PRESCALE);
    uart_print(" max_events=");
    uart_print_u32(MAX_EVENTS);
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
