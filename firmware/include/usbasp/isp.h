#ifndef USBASP_ISP_H_
#define USBASP_ISP_H_

#include "usbasp/protocol.h"

#define ISP_OUT  PORTB
#define ISP_IN   PINB
#define ISP_DDR  DDRB
#define ISP_RST  PB2
#define ISP_MOSI PB3
#define ISP_MISO PB4
#define ISP_SCK  PB5

void ispConnect(void);
void ispDisconnect(void);
uchar ispTransmit_sw(uchar send_byte);
uchar ispTransmit_hw(uchar send_byte);
uchar ispEnterProgrammingMode(void);
uchar ispReadEEPROM(unsigned int address);
uchar ispWriteFlash(unsigned long address, uchar data, uchar pollmode);
uchar ispFlushPage(unsigned long address, uchar pollvalue);
uchar ispReadFlash(unsigned long address);
uchar ispWriteEEPROM(unsigned int address, uchar data);
extern uchar (*ispTransmit)(uchar);
void ispSetSCKOption(uchar sckoption);

#endif
