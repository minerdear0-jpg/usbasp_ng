#ifndef USBASP_PROG_STATE_H_
#define USBASP_PROG_STATE_H_

#include "usbasp/protocol.h"
#include "usbdrv.h"

extern uchar prog_sck;
extern uchar prog_state;
extern uchar prog_address_newmode;
extern unsigned long prog_address;
extern unsigned int prog_nbytes;
extern unsigned int prog_pagesize;
extern uchar prog_blockflags;
extern unsigned int prog_pagecounter;
extern uchar replyBuffer[8];

usbMsgLen_t usbasp_vendor_setup(uchar data[8]);
uchar usbasp_isp_read(uchar *data, uchar len);
uchar usbasp_isp_write(uchar *data, uchar len);

#endif
