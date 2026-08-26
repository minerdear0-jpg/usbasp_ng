#include "usbdrv.h"
#include "usbasp/prog_state.h"
#include "usbasp/ms_os_20.h"

usbMsgLen_t usbFunctionDescriptor(struct usbRequest *rq)
{
    if ((rq->wValue.bytes[1] == USBDESCR_BOS) && (rq->wValue.bytes[0] == 0x00)) {
        usbMsgPtr = (usbMsgPtr_t)&usbasp_bos_descriptor;
        return USBASP_BOS_LEN;
    }
    return 0;
}

/* V-USB: INT0 only clocks the bus. usbProcessRx() / this setup run in
 * usbPoll() from main with I=1, so INT0 may preempt ISP. */
usbMsgLen_t usbFunctionSetup(uchar data[8])
{
    if ((data[0] & USBRQ_TYPE_MASK) == USBRQ_TYPE_VENDOR
        && (data[0] & USBRQ_RCPT_MASK) == USBRQ_RCPT_DEVICE
        && data[1] == USBASP_MS_OS_VENDOR_CODE
        && data[4] == MS_OS_2_0_DESCRIPTOR_INDEX) {
        usbMsgFlags = USB_FLG_MSGPTR_IS_ROM;
        usbMsgPtr = (usbMsgPtr_t)&usbasp_ms_os_20_set;
        return USBASP_MS_OS_20_SET_LEN;
    }

    usbMsgLen_t len = usbasp_vendor_setup(data);
    usbMsgPtr = (usbMsgPtr_t)replyBuffer;
    return len;
}

uchar usbFunctionRead(uchar *data, uchar len)
{
    return usbasp_isp_read(data, len);
}

uchar usbFunctionWrite(uchar *data, uchar len)
{
    return usbasp_isp_write(data, len);
}
