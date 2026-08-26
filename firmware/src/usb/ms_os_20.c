/*
 * Classic USB device descriptor (bcdUSB 2.01 so Windows asks for BOS)
 * plus BOS + MS OS 2.0 WinUSB binding. One vendor interface, EP0 only.
 * Not the composite descriptor set.
 */

#include "usbasp/ms_os_20.h"

#define USBDESCR_DEVICE_CAPABILITY_TYPE 0x10
#define USBDESCR_DEVICE_CAPABILITY_PLATFORM 0x05
#define MS_OS_20_SET_HEADER_DESCRIPTOR 0x00, 0x00
#define MS_OS_20_FEATURE_COMPATIBLE_ID 0x03, 0x00
#define MS_OS_20_FEATURE_REG_PROPERTY 0x04, 0x00
#define MS_OS_20_REG_PROPERTY_REG_SZ 0x01, 0x00

/* V-USB uses this when USB_CFG_DESCR_PROPS_DEVICE is USB_PROP_LENGTH(18). */
PROGMEM const char usbDescriptorDevice[] = {
    18,
    USBDESCR_DEVICE,
    0x01, 0x02, /* bcdUSB 2.01 — BOS */
    USB_CFG_DEVICE_CLASS,
    USB_CFG_DEVICE_SUBCLASS,
    0,
    8,
    (char)USB_CFG_VENDOR_ID,
    (char)USB_CFG_DEVICE_ID,
    USB_CFG_DEVICE_VERSION, /* bcdDevice 2.02 */
    USB_CFG_DESCR_PROPS_STRING_VENDOR != 0 ? 1 : 0,
    USB_CFG_DESCR_PROPS_STRING_PRODUCT != 0 ? 2 : 0,
    0, /* iSerial none */
    1,
};

PROGMEM const char usbasp_bos_descriptor[USBASP_BOS_LEN] = {
    0x05,
    USBDESCR_BOS,
    0x21, 0x00,
    0x01,
    0x1C,
    USBDESCR_DEVICE_CAPABILITY_TYPE,
    USBDESCR_DEVICE_CAPABILITY_PLATFORM,
    0x00,
    0xDF, 0x60, 0xDD, 0xD8, 0x89, 0x45, 0xC7, 0x4C,
    0x9C, 0xD2, 0x65, 0x9D, 0x9E, 0x64, 0x8A, 0x9F,
    0x00, 0x00, 0x03, 0x06,
    0x9E, 0x00,
    USBASP_MS_OS_VENDOR_CODE,
    0x00,
};

/* Device-level WINUSB + DeviceInterfaceGUID. wTotalLength 0x9E. */
PROGMEM const char usbasp_ms_os_20_set[USBASP_MS_OS_20_SET_LEN] = {
    0x0A, 0x00,
    MS_OS_20_SET_HEADER_DESCRIPTOR,
    0x00, 0x00, 0x03, 0x06,
    0x9E, 0x00,

    0x14, 0x00,
    MS_OS_20_FEATURE_COMPATIBLE_ID,
    'W', 'I', 'N', 'U', 'S', 'B', 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,

    0x80, 0x00,
    MS_OS_20_FEATURE_REG_PROPERTY,
    MS_OS_20_REG_PROPERTY_REG_SZ,
    0x28, 0x00,
    'D', 0x00, 'e', 0x00, 'v', 0x00, 'i', 0x00, 'c', 0x00,
    'e', 0x00, 'I', 0x00, 'n', 0x00, 't', 0x00, 'e', 0x00,
    'r', 0x00, 'f', 0x00, 'a', 0x00, 'c', 0x00, 'e', 0x00,
    'G', 0x00, 'U', 0x00, 'I', 0x00, 'D', 0x00, 0x00, 0x00,
    0x4e, 0x00,
    '{', 0x00, 'A', 0x00, 'D', 0x00, '5', 0x00, '7', 0x00,
    'D', 0x00, '3', 0x00, 'B', 0x00, '9', 0x00, '-', 0x00,
    '1', 0x00, '1', 0x00, '6', 0x00, '6', 0x00, '-', 0x00,
    '4', 0x00, '3', 0x00, 'F', 0x00, '8', 0x00, '-', 0x00,
    '8', 0x00, '7', 0x00, '9', 0x00, '0', 0x00, '-', 0x00,
    '0', 0x00, 'B', 0x00, 'E', 0x00, '1', 0x00, '4', 0x00,
    'D', 0x00, 'D', 0x00, 'C', 0x00, '7', 0x00, '5', 0x00,
    '0', 0x00, '4', 0x00, '}', 0x00, 0x00, 0x00,
};

_Static_assert(sizeof(usbDescriptorDevice) == 18, "device descriptor");
_Static_assert(sizeof(usbasp_bos_descriptor) == USBASP_BOS_LEN, "BOS");
_Static_assert(sizeof(usbasp_ms_os_20_set) == USBASP_MS_OS_20_SET_LEN, "MS OS 2.0 set");
