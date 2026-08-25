#include "usbdrv.h"
#include "usbasp/prog_state.h"

usbMsgLen_t usbFunctionSetup(uchar data[8])
{
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
