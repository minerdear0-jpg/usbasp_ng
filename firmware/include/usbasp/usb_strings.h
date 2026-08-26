#ifndef USBASP_USB_STRINGS_H_
#define USBASP_USB_STRINGS_H_

/*
 * USB string descriptor indices in the Device Descriptor.
 * These are NOT V-USB USB_CFG_DESCR_PROPS_STRING_* flags
 * (0 = default string table in usbdrv.c; non-zero = custom).
 *
 * Classic: manufacturer + product only (legacy avrdude -c usbasp).
 * HIDUART may also advertise USB_STR_SERIAL (EEPROM).
 */
enum {
    USB_STR_NONE = 0,
    USB_STR_MANUFACTURER = 1, /* "www.fischl.de" via USB_CFG_VENDOR_NAME */
    USB_STR_PRODUCT = 2,      /* "USBasp" via USB_CFG_DEVICE_NAME */
    USB_STR_SERIAL = 3,
};

#endif
