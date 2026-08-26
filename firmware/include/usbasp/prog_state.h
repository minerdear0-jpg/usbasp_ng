#ifndef USBASP_PROG_STATE_H_
#define USBASP_PROG_STATE_H_

#include <stdint.h>
#include "usbasp/protocol.h"
#include "usbdrv.h"

extern uchar requested_sck;
extern uchar prog_state;
extern uchar prog_address_newmode;
/* Flash / SETLONGADDRESS: 32-bit. EEPROM wire cmds use only low 16 bits. */
extern uint32_t prog_address;
extern uint16_t prog_nbytes;
extern uint16_t prog_pagesize;
extern uchar prog_blockflags;
extern uint16_t prog_pagecounter;
extern uchar replyBuffer[8];

void prog_reset_state(void);

usbMsgLen_t usbasp_vendor_setup(uchar data[8]);
uchar usbasp_isp_read(uchar *data, uchar len);
uchar usbasp_isp_write(uchar *data, uchar len);

#endif
