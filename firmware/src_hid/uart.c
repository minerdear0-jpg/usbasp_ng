#include <avr/io.h>
#include <avr/interrupt.h>
#include "usbdrv.h"
#include "uart.h"
#include "cbuf.h"

volatile struct usbasp_cbuf rx_Q;
volatile struct usbasp_cbuf tx_Q;

volatile uint8_t dataByte;

void __vector_usart_rxc_wrapped(void) __attribute__((signal));
void __vector_usart_rxc_wrapped(void)
{
    if (!CBUF_IsFull(rx_Q)) {
        *CBUF_GetPushEntryPtr(rx_Q) = dataByte;
        CBUF_AdvancePushIdx(rx_Q);
    }
}

#if (defined __AVR_ATmega8__) || (defined __AVR_ATmega8A__)
ISR(USART_RXC_vect, ISR_NAKED)
{
#elif (defined __AVR_ATmega88__) || (defined __AVR_ATmega88PA__)
ISR(USART_RX_vect, ISR_NAKED)
{
#endif
    __asm__ volatile(
        "lds     __tmp_reg__, %0\n"
        "sts     %1, __tmp_reg__\n"
        "rjmp __vector_usart_rxc_wrapped\n"
        :: "m"(USBASPUART_UDR), "m"(dataByte));
}

void __vector_usart_udre_wrapped(void) __attribute__((signal));
void __vector_usart_udre_wrapped(void)
{
    if (!CBUF_IsEmpty(tx_Q)) {
        USBASPUART_UDR = *CBUF_GetPopEntryPtr(tx_Q);
        CBUF_AdvancePopIdx(tx_Q);
    } else {
        USBASPUART_UCSRB &= (uchar)~(1 << USBASPUART_UDRIE);
    }
}

ISR(USART_UDRE_vect, ISR_NAKED)
{
    __asm__ volatile("rjmp __vector_usart_udre_wrapped\n");
}

uchar uart_disable(void)
{
    PORTD &= ~(1 << PIND0);
    USBASPUART_UCSRB = 0;
    CBUF_Init(tx_Q);
    CBUF_Init(rx_Q);
    if (usbAllRequestsAreDisabled())
        usbEnableAllRequests();
    return UART_STATE_DISABLED;
}

static void uart_config_int(uint16_t baud, uint8_t par, uint8_t stop, uint8_t bytes)
{
    CBUF_Init(tx_Q);
    CBUF_Init(rx_Q);
    USBASPUART_UCSRA = (1 << USBASPUART_U2X);

    uint8_t byte = 0;
    switch (par) {
    case USBASP_UART_PARITY_EVEN:
        byte |= (1 << USBASPUART_UPM1);
        break;
    case USBASP_UART_PARITY_ODD:
        byte |= (1 << USBASPUART_UPM1) | (1 << USBASPUART_UPM0);
        break;
    default:
        break;
    }
    if (stop == USBASP_UART_STOP_2BIT)
        byte |= (1 << USBASPUART_USBS);
    switch (bytes) {
    case USBASP_UART_BYTES_6B:
        byte |= (1 << USBASPUART_UCSZ0);
        break;
    case USBASP_UART_BYTES_7B:
        byte |= (1 << USBASPUART_UCSZ1);
        break;
    case USBASP_UART_BYTES_8B:
        byte |= (1 << USBASPUART_UCSZ1) | (1 << USBASPUART_UCSZ0);
        break;
    case USBASP_UART_BYTES_9B:
        byte |= (1 << USBASPUART_UCSZ2) | (1 << USBASPUART_UCSZ1) | (1 << USBASPUART_UCSZ0);
        break;
    default:
        break;
    }

#if defined(USBASPUART_URSEL)
    USBASPUART_UCSRC = (1 << USBASPUART_URSEL) | byte;
#else
    USBASPUART_UCSRC = byte;
#endif
    USBASPUART_UBRRH = (unsigned char)(baud >> 8);
    USBASPUART_UBRRL = (unsigned char)baud;
    USBASPUART_UCSRB = (1 << USBASPUART_RXCIE) | (1 << USBASPUART_RXEN) | (1 << USBASPUART_TXEN);
    PORTD |= (1 << PIND0);
}

uchar uart_config(uchar *cfgData)
{
    if ((cfgData[1] << 8) | cfgData[0]) {
        uart_config_int(
            (uint16_t)((cfgData[1] << 8) | cfgData[0]),
            cfgData[2] & USBASP_UART_PARITY_MASK,
            cfgData[2] & USBASP_UART_STOP_MASK,
            cfgData[2] & USBASP_UART_BYTES_MASK);
        return UART_STATE_ENABLED;
    }
    return uart_disable();
}
