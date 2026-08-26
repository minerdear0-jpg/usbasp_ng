#ifndef USBASP_SCK_H_
#define USBASP_SCK_H_

#include "usbasp/protocol.h"

extern uchar sck_sw_delay;
/* Last id actually applied to the wire (after AUTO→1500, jumper, autoslow). */
extern uchar effective_sck;

void ispSetSCKOption(uchar option);
/* Jumper 8 kHz else host SETISPSCK id (`prog_sck`). Does not write `prog_sck`. */
void isp_apply_host_sck(void);
void isp_spi_hw_enable(void);
void isp_spi_hw_disable(void);
void isp_sck_delay(void);
/* 1 if ispSetSCKOption applied software 8 kHz (jumper or SETISPSCK id 5). */
int isp_sck_is_8khz(void);
/* Next slower AUTO step: 1.5 MHz → 375 kHz → 93.75 kHz → 16 kHz → 500 Hz.
 * Returns USBASP_ISP_SCK_AUTO when there is no slower step. */
uchar isp_sck_autoslow(uchar sck);

#endif
