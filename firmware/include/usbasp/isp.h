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
void isp_out_set_bit(uchar bit);
void isp_out_clr_bit(uchar bit);
uchar ispTransmit_sw(uchar send_byte);
uchar ispTransmit_hw(uchar send_byte);
uchar ispEnterProgrammingMode(void);
uchar ispReadEEPROM(unsigned int address);
uchar ispWriteFlash(unsigned long address, uchar data, uchar pollmode);
uchar ispFlushPage(unsigned long address, uchar pollvalue);
uchar ispReadFlash(unsigned long address);
uchar ispWriteEEPROM(unsigned int address, uchar data);

typedef uchar (*isp_transfer_fn)(uchar);

/* HW SPI vs software bitbang. Session code uses enable/disable, not SPCR. */
struct isp_transport {
    isp_transfer_fn transfer;
    void (*enable)(void);
    void (*disable)(void);
};

extern struct isp_transport isp_bus;

/* Keep transfer/enable/disable as one transport. SW enable clears HW SPI. */
void isp_bus_select_hw(void);
void isp_bus_select_sw(void);

/* Hot path: same as 2011 function-pointer call sites. */
#define ispTransmit (isp_bus.transfer)

#endif
