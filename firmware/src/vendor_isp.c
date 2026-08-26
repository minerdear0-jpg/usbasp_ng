#include "usbasp_config.h"
#include "usbasp/prog_state.h"
#include "usbasp/protocol.h"
#include "usbasp/endian.h"
#include "usbasp/isp.h"
#include "usbasp/sck.h"
#include "usbasp/tpi.h"
#include "usbasp/tpi_defs.h"
#include "usbasp/clock.h"
#include "usbasp/board.h"
#include "diag/diag.h"

uchar requested_sck = USBASP_ISP_SCK_AUTO;
uchar prog_state = PROG_STATE_IDLE;
uchar prog_address_newmode = 0;
uint32_t prog_address;
uint16_t prog_nbytes = 0;
uint16_t prog_pagesize;
uchar prog_blockflags;
uint16_t prog_pagecounter;
uchar replyBuffer[8];

void prog_reset_state(void)
{
    prog_state = PROG_STATE_IDLE;
    prog_nbytes = 0;
    prog_pagecounter = 0;
    prog_pagesize = 0;
    prog_blockflags = 0;
    prog_address_newmode = 0;
    prog_address = 0;
}

/* Enter USB data-stage only when host asked for a non-zero transfer. */
static usbMsgLen_t prog_begin_transfer(uchar state, uint16_t nbytes)
{
    prog_nbytes = nbytes;
    if (nbytes == 0) {
        prog_state = PROG_STATE_IDLE;
        return 0;
    }
    prog_state = state;
    return USB_NO_MSG;
}

usbMsgLen_t usbasp_vendor_setup(uchar data[8])
{
    usbMsgLen_t len = 0;

    switch (data[1]) {
    case USBASP_FUNC_CONNECT:
        /* JP3 forces 8 kHz on the wire; do not overwrite the host SETISPSCK id. */
        prog_reset_state();
        isp_apply_host_sck();
        ispConnect();
        diag_on_connect();
        break;

    case USBASP_FUNC_DISCONNECT:
        ispDisconnect();
        prog_reset_state();
        diag_on_disconnect();
        break;

    case USBASP_FUNC_TRANSMIT:
        board_led_isp_activity();
        replyBuffer[0] = ispTransmit(data[2]);
        replyBuffer[1] = ispTransmit(data[3]);
        replyBuffer[2] = ispTransmit(data[4]);
        replyBuffer[3] = ispTransmit(data[5]);
        len = 4;
        break;

    case USBASP_FUNC_READFLASH:
        if (!prog_address_newmode)
            prog_address = usbasp_read_le16(&data[2]);
        len = prog_begin_transfer(PROG_STATE_READFLASH, usbasp_read_le16(&data[6]));
        break;

    case USBASP_FUNC_READEEPROM:
        /* Wire address is 16-bit; ignore high bits if SETLONGADDRESS was used. */
        if (!prog_address_newmode)
            prog_address = usbasp_read_le16(&data[2]);
        else
            prog_address &= 0xffffu;
        len = prog_begin_transfer(PROG_STATE_READEEPROM, usbasp_read_le16(&data[6]));
        break;

    case USBASP_FUNC_ENABLEPROG:
        replyBuffer[0] = ispEnterProgrammingMode();
        len = 1;
        break;

    case USBASP_FUNC_WRITEFLASH:
        if (!prog_address_newmode)
            prog_address = usbasp_read_le16(&data[2]);
        prog_pagesize = data[4];
        prog_blockflags = data[5] & 0x0F;
        prog_pagesize += (uint16_t)(((uint16_t)data[5] & 0xF0) << 4);
        if (prog_blockflags & PROG_BLOCKFLAG_FIRST)
            prog_pagecounter = prog_pagesize;
        len = prog_begin_transfer(PROG_STATE_WRITEFLASH, usbasp_read_le16(&data[6]));
        break;

    case USBASP_FUNC_WRITEEEPROM:
        if (!prog_address_newmode)
            prog_address = usbasp_read_le16(&data[2]);
        else
            prog_address &= 0xffffu;
        prog_pagesize = 0;
        prog_blockflags = 0;
        len = prog_begin_transfer(PROG_STATE_WRITEEEPROM, usbasp_read_le16(&data[6]));
        break;

    case USBASP_FUNC_SETLONGADDRESS:
        prog_address_newmode = 1;
        prog_address = usbasp_read_le32(&data[2]);
        break;

    case USBASP_FUNC_SETISPSCK:
        requested_sck = data[2];
        isp_apply_host_sck();
        diag_emit_sck_config();
        replyBuffer[0] = 0;
        len = 1;
        break;

    case USBASP_FUNC_TPI_CONNECT:
        tpi_dly_cnt = usbasp_read_le16(&data[2]);
        isp_out_set_bit(ISP_RST);
        ISP_DDR |= (1 << ISP_RST);
        clockWait(3);
        isp_out_clr_bit(ISP_RST);
        clockWait(16);
        tpi_init();
        break;

    case USBASP_FUNC_TPI_DISCONNECT:
        tpi_send_byte(TPI_OP_SSTCS(TPISR));
        tpi_send_byte(0);
        clockWait(10);
        isp_out_set_bit(ISP_RST);
        clockWait(5);
        isp_out_clr_bit(ISP_RST);
        clockWait(5);
        ISP_DDR &= ~(1 << ISP_RST);
        ISP_DDR &= ~(1 << ISP_SCK);
        ISP_DDR &= ~(1 << ISP_MOSI);
        isp_out_clr_bit(ISP_RST);
        ISP_OUT &= ~(1 << ISP_SCK);
        ISP_OUT &= ~(1 << ISP_MOSI);
        break;

    case USBASP_FUNC_TPI_RAWREAD:
        replyBuffer[0] = tpi_recv_byte();
        len = 1;
        break;

    case USBASP_FUNC_TPI_RAWWRITE:
        tpi_send_byte(data[2]);
        break;

    case USBASP_FUNC_TPI_READBLOCK:
        prog_address = usbasp_read_le16(&data[2]);
        len = prog_begin_transfer(PROG_STATE_TPI_READ, usbasp_read_le16(&data[6]));
        break;

    case USBASP_FUNC_TPI_WRITEBLOCK:
        prog_address = usbasp_read_le16(&data[2]);
        len = prog_begin_transfer(PROG_STATE_TPI_WRITE, usbasp_read_le16(&data[6]));
        break;

    case USBASP_FUNC_GETCAPABILITIES:
        /* Advertise TPI only when board profile enables it after silicon proof. */
#if USBASP_HAS_TPI
        replyBuffer[0] = USBASP_CAP_TPI;
#else
        replyBuffer[0] = 0;
#endif
        replyBuffer[1] = 0;
        replyBuffer[2] = 0;
#if USBASP_HAS_3MHZ
        replyBuffer[3] = (uchar)(USBASP_CAP_3MHZ >> 24);
#else
        replyBuffer[3] = 0;
#endif
        len = 4;
        break;

    default:
        break;
    }

    return len;
}

uchar usbasp_isp_read(uchar *data, uchar len)
{
    uchar i;

    if (prog_state != PROG_STATE_READFLASH
        && prog_state != PROG_STATE_READEEPROM
        && prog_state != PROG_STATE_TPI_READ) {
        return 0xff;
    }

    if (prog_state == PROG_STATE_TPI_READ) {
        if (prog_nbytes && len > prog_nbytes)
            len = (uchar)prog_nbytes;
        tpi_read_block((uint16_t)prog_address, data, len);
        prog_address += len;
        if (prog_nbytes > len)
            prog_nbytes -= len;
        else
            prog_nbytes = 0;
        if ((len < 8) || (prog_nbytes == 0))
            prog_state = PROG_STATE_IDLE;
        return len;
    }

    board_led_isp_activity();
    for (i = 0; i < len; i++) {
        if (prog_state == PROG_STATE_READFLASH)
            data[i] = ispReadFlash(prog_address);
        else
            data[i] = ispReadEEPROM((uint16_t)prog_address);
        prog_address++;
        if (prog_nbytes)
            prog_nbytes--;
    }

    if ((len < 8) || (prog_nbytes == 0))
        prog_state = PROG_STATE_IDLE;

    return len;
}

uchar usbasp_isp_write(uchar *data, uchar len)
{
    uchar retVal = 0;
    uchar i;

    if (prog_state != PROG_STATE_WRITEFLASH
        && prog_state != PROG_STATE_WRITEEEPROM
        && prog_state != PROG_STATE_TPI_WRITE) {
        return 0xff;
    }

    if (prog_state == PROG_STATE_TPI_WRITE) {
        if (prog_nbytes && len > prog_nbytes)
            len = (uchar)prog_nbytes;
        tpi_write_block((uint16_t)prog_address, data, len);
        prog_address += len;
        if (prog_nbytes > len)
            prog_nbytes -= len;
        else
            prog_nbytes = 0;
        if (prog_nbytes == 0) {
            prog_state = PROG_STATE_IDLE;
            return 1;
        }
        return 0;
    }

    board_led_isp_activity();
    for (i = 0; i < len; i++) {
        /* Never fall through WRITEFLASH → EEPROM when nbytes ends mid-packet. */
        if (prog_state == PROG_STATE_WRITEFLASH) {
            if (prog_pagesize == 0) {
                ispWriteFlash(prog_address, data[i], 1);
            } else {
                ispWriteFlash(prog_address, data[i], 0);
                prog_pagecounter--;
                if (prog_pagecounter == 0) {
                    ispFlushPage(prog_address, data[i]);
                    prog_pagecounter = prog_pagesize;
                }
            }
        } else if (prog_state == PROG_STATE_WRITEEEPROM) {
            ispWriteEEPROM((uint16_t)prog_address, data[i]);
        } else {
            break;
        }

        if (prog_nbytes)
            prog_nbytes--;

        if (prog_nbytes == 0) {
            prog_state = PROG_STATE_IDLE;
            if ((prog_blockflags & PROG_BLOCKFLAG_LAST)
                && (prog_pagecounter != prog_pagesize)) {
                ispFlushPage(prog_address, data[i]);
            }
            retVal = 1;
            prog_address++;
            break; /* ignore remainder of this USB OUT packet */
        }

        prog_address++;
    }

    return retVal;
}
