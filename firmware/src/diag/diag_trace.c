#include "usbasp_config.h"

#if USBASP_HAS_DIAG

#include "diag/diag_ring.h"
#include "diag/diag_events.h"
#include "diag/diag_clock.h"
#include "usbasp/prog_state.h"

#include <string.h>

/*
 * Single TRACE ring. See diag_ring.h for ownership / lossy policy.
 */

#if (USBASP_DIAG_TRACE_SLOTS & (USBASP_DIAG_TRACE_SLOTS - 1)) != 0
#error USBASP_DIAG_TRACE_SLOTS must be a power of two
#endif

static diag_frame_t trace_frames[USBASP_DIAG_TRACE_SLOTS];
static uint8_t trace_head; /* next write (mod 256 counter; mask on index) */
static uint8_t trace_tail; /* next read */
static uint16_t trace_write_index;
static uint8_t trace_dropped; /* saturating 0..255 since last marker */
static uint8_t trace_overflow_sticky;
static uint8_t trace_overflow_marker; /* emit TRACE_OVERFLOW on next roomy push */
static uint8_t trace_state;

static uint8_t trace_len(void)
{
    return (uint8_t)(trace_head - trace_tail);
}

static void trace_pack(uint8_t out[8], const diag_frame_t *f)
{
    out[0] = f->type;
    out[1] = f->flags;
    out[2] = (uint8_t)(f->timestamp & 0xff);
    out[3] = (uint8_t)(f->timestamp >> 8);
    out[4] = f->a;
    out[5] = f->b;
    out[6] = 0;
    out[7] = prog_state;
}

static void trace_write_slot(const diag_frame_t *frame)
{
    diag_frame_t *slot = &trace_frames[trace_head & (USBASP_DIAG_TRACE_SLOTS - 1)];
    *slot = *frame;
    trace_head++;
    trace_write_index++;
}

static void trace_overwrite_oldest(void)
{
    trace_tail++;
    if (trace_dropped < 255)
        trace_dropped++;
    trace_overflow_sticky = 1;
    trace_overflow_marker = 1;
}

void diag_trace_init(void)
{
    trace_head = 0;
    trace_tail = 0;
    trace_write_index = 0;
    trace_dropped = 0;
    trace_overflow_sticky = 0;
    trace_overflow_marker = 0;
    trace_state = DIAG_CAP_STATE_IDLE;
}

void diag_trace_arm(void)
{
    trace_head = 0;
    trace_tail = 0;
    trace_write_index = 0;
    trace_dropped = 0;
    trace_overflow_sticky = 0;
    trace_overflow_marker = 0;
    trace_state = DIAG_CAP_STATE_ARMED;
}

void diag_trace_idle(void)
{
    trace_state = DIAG_CAP_STATE_IDLE;
}

void diag_trace_snapshot(diag_capture_meta_t *meta)
{
    if (!meta)
        return;
    meta->slots = (uint16_t)USBASP_DIAG_TRACE_SLOTS;
    meta->valid = (uint16_t)trace_len();
    meta->write_index = trace_write_index;
    meta->overflow = trace_overflow_sticky;
    meta->state = trace_state;
}

bool diag_trace_push(const diag_frame_t *frame)
{
    diag_frame_t ov;
    uint8_t is_meta;

    if (!frame)
        return false;
    /* FROZEN reserved for trigger PR — still accept pushes in IDLE/ARMED. */
    if (trace_state == DIAG_CAP_STATE_FROZEN)
        return false;

    is_meta = (frame->type == DIAG_TRACE_BEGIN || frame->type == DIAG_TRACE_END);

    /* Deferred overflow marker: only when there is room (no self-eviction).
     * Skip before TRACE_* metadata so BEGIN/END pairs stay contiguous. */
    if (trace_overflow_marker && !is_meta
        && trace_len() < USBASP_DIAG_TRACE_SLOTS) {
        ov.type = DIAG_TRACE_OVERFLOW;
        ov.flags = 0;
        ov.timestamp = diag_now_wire16();
        ov.a = trace_dropped;
        ov.b = 0;
        trace_write_slot(&ov);
        trace_dropped = 0;
        trace_overflow_marker = 0;
    }

    if (trace_len() >= USBASP_DIAG_TRACE_SLOTS)
        trace_overwrite_oldest();

    trace_write_slot(frame);
    return true;
}

uint8_t diag_trace_drain(uint8_t out[8])
{
    diag_frame_t f;
    uint16_t ts;

    (void)diag_now_wire16(); /* keep Timer1 epoch alive while host polls */

    if (trace_len() == 0) {
        /*
         * If marker still pending and ring empty, emit once from drain so a
         * quiet bus after a flood still reports the loss (no jammed push).
         */
        if (!trace_overflow_marker)
            return 0;
        ts = diag_now_wire16();
        f.type = DIAG_TRACE_OVERFLOW;
        f.flags = 0;
        f.timestamp = ts;
        f.a = trace_dropped;
        f.b = 0;
        trace_pack(out, &f);
        trace_dropped = 0;
        trace_overflow_marker = 0;
        return 1;
    }

    f = trace_frames[trace_tail & (USBASP_DIAG_TRACE_SLOTS - 1)];
    trace_tail++;
    trace_pack(out, &f);
    return 1;
}

#endif /* USBASP_HAS_DIAG */
