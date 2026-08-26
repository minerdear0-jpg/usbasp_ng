#ifndef USBASP_DIAG_RING_H_
#define USBASP_DIAG_RING_H_

#include <stdint.h>

/*
 * SPSC ring:
 *   producer = ISP / foreground vendor_isp context only
 *   consumer = diag_poll_drain() from main/poll loop
 *   no ISR producer in P0
 */

#define DIAG_RING_SIZE 16

typedef struct {
    uint8_t type;
    uint8_t flags;
    uint16_t timestamp;
    uint8_t a;
    uint8_t b;
} diag_frame_t;

#endif
