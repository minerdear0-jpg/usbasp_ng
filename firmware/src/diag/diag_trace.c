#include "usbasp_config.h"

#if USBASP_HAS_DIAG

#include "diag/diag_ring.h"
#include "diag/diag_events.h"
#include "diag/diag_clock.h"
#include "diag/diag_trigger.h"
#include "usbasp/prog_state.h"

#include <string.h>

/*
 * Single TRACE ring + capture freeze lifecycle.
 * Trigger predicates: diag_trigger.h (evaluated after push).
 */

#if (USBASP_DIAG_TRACE_SLOTS & (USBASP_DIAG_TRACE_SLOTS - 1)) != 0
#error USBASP_DIAG_TRACE_SLOTS must be a power of two
#endif

static diag_frame_t trace_frames[USBASP_DIAG_TRACE_SLOTS];
static uint8_t trace_head;
static uint8_t trace_tail;
static uint16_t trace_write_index;
static uint8_t trace_dropped;
static uint8_t trace_overflow_sticky;
static uint8_t trace_overflow_marker;
static uint8_t trace_state;

static uint8_t trig_fired;
static uint8_t trig_kind_latched;
static uint16_t trig_index;
static uint16_t trig_timestamp;
static uint8_t post_left;
static uint8_t post_collected;

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

static void trace_clear_trigger_latch(void)
{
    trig_fired = 0;
    trig_kind_latched = DIAG_TRIG_NONE;
    trig_index = 0;
    trig_timestamp = 0;
    post_left = 0;
    post_collected = 0;
}

void diag_trigger_on_event(const diag_frame_t *frame, uint16_t write_index)
{
#if !USBASP_HAS_DIAG_TRIGGER
    (void)frame;
    (void)write_index;
#else
    const diag_trigger_t *cfg;

    if (!frame)
        return;
    if (trace_state == DIAG_CAP_STATE_FROZEN || trace_state == DIAG_CAP_STATE_IDLE)
        return;

    cfg = diag_trigger_cfg();

    if (trace_state == DIAG_CAP_STATE_ARMED) {
        if (!diag_trigger_match(frame, cfg))
            return;
        trig_fired = 1;
        trig_kind_latched = cfg->kind;
        trig_index = write_index;
        trig_timestamp = frame->timestamp;
        post_collected = 0;
        if (USBASP_DIAG_POST_CAPTURE_EVENTS == 0) {
            trace_state = DIAG_CAP_STATE_FROZEN;
            post_left = 0;
        } else {
            trace_state = DIAG_CAP_STATE_POST;
            post_left = (uint8_t)USBASP_DIAG_POST_CAPTURE_EVENTS;
        }
        return;
    }

    if (trace_state == DIAG_CAP_STATE_POST) {
        if (post_left > 0) {
            post_left--;
            if (post_collected < 255)
                post_collected++;
        }
        if (post_left == 0)
            trace_state = DIAG_CAP_STATE_FROZEN;
    }
#endif
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
    trace_clear_trigger_latch();
    diag_trigger_init();
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
    trace_clear_trigger_latch();
    /* Keep configured predicate; default ENABLEPROG_FAIL from init. */
}

void diag_trace_idle(void)
{
    trace_state = DIAG_CAP_STATE_IDLE;
}

uint8_t diag_trace_state(void)
{
    return trace_state;
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
    meta->triggered = trig_fired;
    meta->trigger_kind = trig_kind_latched;
    meta->trigger_index = trig_index;
    meta->trigger_timestamp = trig_timestamp;
    meta->post_count = post_collected;
}

bool diag_trace_push(const diag_frame_t *frame)
{
#if !USBASP_HAS_DIAG_TRIGGER
    /* Compact boards: lossy ring only (no freeze / post / trigger). */
    if (!frame)
        return false;
    if (trace_len() >= USBASP_DIAG_TRACE_SLOTS)
        trace_overwrite_oldest();
    trace_write_slot(frame);
    return true;
#else
    diag_frame_t ov;
    uint8_t is_meta;
    uint8_t is_trailer;
    uint16_t wi_after;

    if (!frame)
        return false;

    is_meta = (frame->type == DIAG_TRACE_BEGIN || frame->type == DIAG_TRACE_END);
    is_trailer = (frame->type == DIAG_RESET || frame->type == DIAG_SESSION_END);

    if (trace_state == DIAG_CAP_STATE_FROZEN) {
        /* Capture history frozen: no normal events. TRACE_END footer may
         * overwrite to guarantee metadata delivery; other trailers need space. */
        if (frame->type == DIAG_TRACE_END) {
            if (trace_len() >= USBASP_DIAG_TRACE_SLOTS)
                trace_overwrite_oldest();
            trace_write_slot(frame);
            return true;
        }
        if (is_trailer && trace_len() < USBASP_DIAG_TRACE_SLOTS) {
            trace_write_slot(frame);
            return true;
        }
        return false;
    }

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
        diag_trigger_on_event(&ov, trace_write_index);
    }

    if (trace_state == DIAG_CAP_STATE_FROZEN) {
        if (frame->type == DIAG_TRACE_END) {
            if (trace_len() >= USBASP_DIAG_TRACE_SLOTS)
                trace_overwrite_oldest();
            trace_write_slot(frame);
            return true;
        }
        if (is_trailer && trace_len() < USBASP_DIAG_TRACE_SLOTS) {
            trace_write_slot(frame);
            return true;
        }
        return false;
    }

    if (trace_len() >= USBASP_DIAG_TRACE_SLOTS)
        trace_overwrite_oldest();

    trace_write_slot(frame);
    wi_after = trace_write_index;

    if (!is_meta)
        diag_trigger_on_event(frame, wi_after);

    return true;
#endif
}

uint8_t diag_trace_drain(uint8_t out[8])
{
    diag_frame_t f;
    uint16_t ts;

    (void)diag_now_wire16();

    if (trace_len() == 0) {
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
