#ifndef USBASP_SCK_H_
#define USBASP_SCK_H_

#include "usbasp/protocol.h"

extern uchar sck_sw_delay;

void ispSetSCKOption(uchar option);
void isp_spi_hw_enable(void);
void isp_spi_hw_disable(void);
void isp_sck_delay(void);

#endif
