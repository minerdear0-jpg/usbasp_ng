#include "usbdrv.h"
#include "usb_descriptors.h"
#include "usbasp/prog_state.h"
#include "usbasp/protocol.h"
#include "usbasp/endian.h"
#include "uart.h"

static uchar featureReport[8];
static uchar interruptBuffer[8];
static uchar monitorBuffer[8];
static uchar uart_state = UART_STATE_DISABLED;
static uchar hid_set_report;

usbMsgLen_t usbFunctionDescriptor(struct usbRequest *rq)
{
    if ((rq->wValue.bytes[1] == USBDESCR_BOS) && (rq->wValue.bytes[0] == 0x00)) {
        usbMsgPtr = (usbMsgPtr_t)&BOS_DESCRIPTOR;
        return sizeof(BOS_DESCRIPTOR);
    }
    return 0;
}

/* V-USB: INT0 only clocks the bus. This setup runs from usbPoll() with I=1. */
usbMsgLen_t usbFunctionSetup(uchar data[8])
{
    usbMsgLen_t len = 0;

    if ((data[0] & USBRQ_TYPE_MASK) == USBRQ_TYPE_VENDOR) {
        if ((data[0] & USBRQ_RCPT_MASK) == USBRQ_RCPT_DEVICE) {
            if (data[1] == USBASP_FUNC_CONNECT)
                uart_state = uart_disable();

            if ((data[1] == VENDOR_CODE) && (data[4] == MS_OS_2_0_DESCRIPTOR_INDEX)) {
                usbMsgFlags = USB_FLG_MSGPTR_IS_ROM;
                usbMsgPtr = (usbMsgPtr_t)&MS_2_0_OS_DESCRIPTOR_SET;
                return sizeof(MS_2_0_OS_DESCRIPTOR_SET);
            }

            len = usbasp_vendor_setup(data);

            if (data[1] == USBASP_FUNC_DISCONNECT)
                uart_state = uart_config(featureReport);

            if (data[1] != VENDOR_CODE)
                usbMsgPtr = (usbMsgPtr_t)replyBuffer;
            return len;
        }
    } else if ((data[0] & USBRQ_TYPE_MASK) == USBRQ_TYPE_CLASS) {
        if ((data[0] & USBRQ_RCPT_MASK) == USBRQ_RCPT_INTERFACE) {
            if (data[3] == 3) {
                switch (data[1]) {
                case USBRQ_HID_GET_REPORT:
                    usbMsgPtr = (usbMsgPtr_t)&featureReport;
                    return sizeof(featureReport);
                case USBRQ_HID_SET_REPORT:
                    if (usbasp_read_le16(&data[5]) != 0) {
                        hid_set_report = 1;
                        return USB_NO_MSG;
                    }
                    break;
                default:
                    break;
                }
            }
        }
    }

    usbMsgPtr = (usbMsgPtr_t)replyBuffer;
    return len;
}

uchar usbFunctionRead(uchar *data, uchar len)
{
    return usbasp_isp_read(data, len);
}

uchar usbFunctionWrite(uchar *data, uchar len)
{
    if (hid_set_report) {
        featureReport[0] = data[0];
        featureReport[1] = data[1];
        featureReport[2] = data[2];
        uart_state = uart_config(featureReport);
        hid_set_report = 0;
        return 1;
    }
    return usbasp_isp_write(data, len);
}

void usbFunctionWriteOut(uchar *data, uchar len)
{
    if (data[7] > 0) {
        if (data[7] < 8)
            len = data[7];
    } else {
        len = 0;
    }

    if (len && (USBASPUART_UCSRB & (1 << USBASPUART_RXCIE))) {
        if ((CBUF_Len(tx_Q)) + len > (tx_Q_SIZE - 8))
            usbDisableAllRequests();
        do {
            *CBUF_GetPushEntryPtr(tx_Q) = *data++;
            CBUF_AdvancePushIdx(tx_Q);
        } while (--len);
    }
}

static void hid_ep1_in(void)
{
    uint8_t count = 0;
    while ((!CBUF_IsEmpty(rx_Q)) && (count != 7)) {
        interruptBuffer[count++] = CBUF_Get(rx_Q, 0);
        CBUF_AdvancePopIdx(rx_Q);
    }
    interruptBuffer[7] = count;
    if ((!CBUF_IsEmpty(rx_Q)) && (count == 7)) {
        uint8_t tmp = CBUF_Get(rx_Q, 0);
        if (tmp > count) {
            interruptBuffer[count] = tmp;
            CBUF_AdvancePopIdx(rx_Q);
        }
    }
    usbSetInterrupt(interruptBuffer, sizeof(interruptBuffer));
}

static void hid_ep2_in(void)
{
    monitorBuffer[7] = (uchar)(prog_state | uart_state
        | (hid_set_report ? UART_HID_SET_REPORT : 0));
    usbSetInterrupt3(monitorBuffer, sizeof(monitorBuffer));
}

void hiduart_poll(void)
{
    if (!(USBASPUART_UCSRB & (1 << USBASPUART_UDRIE)) && !CBUF_IsEmpty(tx_Q))
        USBASPUART_UCSRB |= (1 << USBASPUART_UDRIE);
    else if (CBUF_IsEmpty(tx_Q)) {
        if (usbAllRequestsAreDisabled())
            usbEnableAllRequests();
    }
    if (usbInterruptIsReady())
        hid_ep1_in();
    if (usbInterruptIsReady3())
        hid_ep2_in();
}
