#ifndef USBASP_MS_OS_20_H_
#define USBASP_MS_OS_20_H_

#include <avr/pgmspace.h>
#include "usbdrv.h"
#include "usbasp/ms_os_vendor.h"

#define USBDESCR_BOS 0x0F

#define USBASP_BOS_LEN 0x21
#define USBASP_MS_OS_20_SET_LEN 0xAE

extern const char usbasp_bos_descriptor[USBASP_BOS_LEN] PROGMEM;
extern const char usbasp_ms_os_20_set[USBASP_MS_OS_20_SET_LEN] PROGMEM;

#endif
