#ifndef USBASP_UART_H
#define USBASP_UART_H

#include "cbuf.h"
#include "usbasp/protocol.h"
#include <stdint.h>

#define USBASP_UART_PARITY_MASK 0x03
#define USBASP_UART_PARITY_NONE 0x00
#define USBASP_UART_PARITY_EVEN 0x01
#define USBASP_UART_PARITY_ODD  0x02
#define USBASP_UART_STOP_MASK   0x04
#define USBASP_UART_STOP_1BIT   0x00
#define USBASP_UART_STOP_2BIT   0x04
#define USBASP_UART_BYTES_MASK  0x38
#define USBASP_UART_BYTES_5B    0x00
#define USBASP_UART_BYTES_6B    0x08
#define USBASP_UART_BYTES_7B    0x10
#define USBASP_UART_BYTES_8B    0x18
#define USBASP_UART_BYTES_9B    0x20

#define UART_STATE_ENABLED  16
#define UART_STATE_DISABLED 0
#define PROG_STATE_SET_REPORT 7

#if (defined __AVR_ATmega8__) || (defined __AVR_ATmega8A__)
#   define USBASPUART_UDR       UDR
#   define USBASPUART_UDRIE     UDRIE
#   define USBASPUART_UCSRA     UCSRA
#   define USBASPUART_UCSRB     UCSRB
#   define USBASPUART_UCSRC     UCSRC
#   define USBASPUART_U2X       U2X
#   define USBASPUART_UCSZ0     UCSZ0
#   define USBASPUART_UCSZ1     UCSZ1
#   define USBASPUART_UCSZ2     UCSZ2
#   define USBASPUART_UPM0      UPM0
#   define USBASPUART_UPM1      UPM1
#   define USBASPUART_USBS      USBS
#   define USBASPUART_UBRRL     UBRRL
#   define USBASPUART_UBRRH     UBRRH
#   define USBASPUART_RXCIE     RXCIE
#   define USBASPUART_RXEN      RXEN
#   define USBASPUART_TXEN      TXEN
#elif (defined __AVR_ATmega88__) || (defined __AVR_ATmega88PA__)
#   define USBASPUART_UDR       UDR0
#   define USBASPUART_UDRIE     UDRIE0
#   define USBASPUART_UCSRA     UCSR0A
#   define USBASPUART_UCSRB     UCSR0B
#   define USBASPUART_UCSRC     UCSR0C
#   define USBASPUART_U2X       U2X0
#   define USBASPUART_UCSZ0     UCSZ00
#   define USBASPUART_UCSZ1     UCSZ01
#   define USBASPUART_UCSZ2     UCSZ02
#   define USBASPUART_UPM0      UPM00
#   define USBASPUART_UPM1      UPM01
#   define USBASPUART_USBS      USBS0
#   define USBASPUART_UBRRL     UBRR0L
#   define USBASPUART_UBRRH     UBRR0H
#   define USBASPUART_RXCIE     RXCIE0
#   define USBASPUART_RXEN      RXEN0
#   define USBASPUART_TXEN      TXEN0
#endif

#define rx_Q_SIZE 128
#define tx_Q_SIZE 128

struct usbasp_cbuf {
    uint8_t m_getIdx;
    uint8_t m_putIdx;
    uint8_t m_entry[rx_Q_SIZE];
};

extern volatile struct usbasp_cbuf rx_Q;
extern volatile struct usbasp_cbuf tx_Q;

uchar uart_config(uchar *cfgData);
uchar uart_disable(void);

#endif
