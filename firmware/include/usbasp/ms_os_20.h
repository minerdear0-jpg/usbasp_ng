#ifndef USBASP_MS_OS_20_H_
#define USBASP_MS_OS_20_H_

#include <avr/pgmspace.h>
#include "usbdrv.h"

/* Microsoft OS 2.0 vendor bRequest. Not a USBasp FUNC (1–16 / 127). */
#define USBASP_MS_OS_VENDOR_CODE 0x5D
#define MS_OS_2_0_DESCRIPTOR_INDEX 0x07
#define USBDESCR_BOS 0x0F

#define USBASP_BOS_LEN 0x21
#define USBASP_MS_OS_20_SET_LEN 0x9E

extern const char usbasp_bos_descriptor[USBASP_BOS_LEN] PROGMEM;
extern const char usbasp_ms_os_20_set[USBASP_MS_OS_20_SET_LEN] PROGMEM;

#endif
