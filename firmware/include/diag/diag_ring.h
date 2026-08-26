#ifndef USBASP_DIAG_RING_H_
#define USBASP_DIAG_RING_H_

#include "usbasp_config.h"

#include <stdint.h>
#include <stdbool.h>

/*
 * Unified lossy TRACE ring (one physical buffer).
 *
 * Execution ownership (P0 / RC):
 *   producer = ISP / vendor_isp / diag_* in main context
 *   consumer = diag_poll_drain() via hiduart_poll() in main context
 *   neither side runs from a USB ISR — no cli() around the hot path.
 *
 * Policy: overwrite oldest when full; never block ISP/USB.
 * TRACE_OVERFLOW is deferred until the next push that has space
 * (never jammed into a full ring as an extra eviction).
 *
 * Lifecycle: IDLE ↔ ARMED → POST_CAPTURE → FROZEN (trigger layer).
 */

#ifndef USBASP_DIAG_TRACE_SLOTS
#define USBASP_DIAG_TRACE_SLOTS 64
#endif

#ifndef USBASP_DIAG_POST_CAPTURE_EVENTS
#define USBASP_DIAG_POST_CAPTURE_EVENTS 16
#endif

/* Power of two required (index mask). */
#define DIAG_RING_SIZE USBASP_DIAG_TRACE_SLOTS

#define DIAG_FRAME_WIRE_SIZE 6

#define DIAG_CAP_STATE_IDLE    0
#define DIAG_CAP_STATE_ARMED   1
#define DIAG_CAP_STATE_POST    2
#define DIAG_CAP_STATE_FROZEN  3

#define DIAG_TS_MODE_TIMER1_WIRE16 0

typedef struct {
    uint8_t type;
    uint8_t flags;
    uint16_t timestamp;
    uint8_t a;
    uint8_t b;
} diag_frame_t;

typedef struct {
    uint16_t slots;
    uint16_t valid;
    uint16_t write_index; /* cumulative pushes (wraps) */
    uint8_t overflow;     /* sticky until TRACE_END reports it */
    uint8_t state;
    uint8_t triggered;    /* 0/1 */
    uint8_t trigger_kind; /* DIAG_TRIG_* */
    uint16_t trigger_index;
    uint16_t trigger_timestamp;
    uint8_t post_count;   /* collected post events (≤ POST_CAPTURE) */
} diag_capture_meta_t;

#if USBASP_HAS_DIAG

void diag_trace_init(void);
void diag_trace_arm(void);   /* IDLE → ARMED, clear ring + trigger latch */
void diag_trace_idle(void);  /* → IDLE after TRACE_END */

bool diag_trace_push(const diag_frame_t *frame);
void diag_trace_snapshot(diag_capture_meta_t *meta);
uint8_t diag_trace_state(void);

/* Consumer for HID EP2 — same ownership as push (main). */
uint8_t diag_trace_drain(uint8_t out[8]);

#endif

#endif
