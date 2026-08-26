#include "usbasp_config.h"

#if USBASP_HAS_DIAG

#include "diag/diag.h"
#include "diag/diag_ring.h"
#include "diag/diag_clock.h"
#include "usbasp/clock.h"
#include "usbasp/isp.h"
#include "usbasp/sck.h"
#include "usbasp/prog_state.h"

#include <string.h>

/*
 * Diagnostics plane on the unified TRACE ring.
 * Timestamps: Timer1 via diag_now(); wire uses low 16 bits.
 */

static uint8_t diag_reset_driven;
static diag_snapshot_t diag_fault_snapshot;
static uint8_t diag_sck_seen;
static uint8_t diag_sck_last;
static uint8_t diag_tr_last;
static uint8_t diag_mem_pages;
static uint8_t diag_mem_open;
static uint8_t diag_trace_inited;

extern uchar requested_sck;
extern uchar effective_sck;

static uint8_t diag_transport(void)
{
    return (isp_bus.transfer == ispTransmit_sw)
        ? (uint8_t)DIAG_TRANSPORT_SW
        : (uint8_t)DIAG_TRANSPORT_HW;
}

static void diag_ensure_trace(void)
{
    if (diag_trace_inited)
        return;
    diag_trace_init();
    diag_trace_inited = 1;
}

bool diag_try_emit(uint8_t type, uint8_t flags, uint8_t a, uint8_t b)
{
    diag_frame_t f;

    diag_ensure_trace();
    f.type = type;
    f.flags = flags;
    f.timestamp = diag_now_wire16();
    f.a = a;
    f.b = b;
    return diag_trace_push(&f);
}

static void diag_emit_trace_begin(void)
{
    /* a=slots (u8 sat), b=wire frame size; flags=state|ts_mode<<4 */
    uint8_t slots = (uint8_t)USBASP_DIAG_TRACE_SLOTS;
    uint8_t fl = (uint8_t)(DIAG_CAP_STATE_ARMED
        | ((uint8_t)DIAG_TS_MODE_TIMER1_WIRE16 << 4));

    (void)diag_try_emit(DIAG_TRACE_BEGIN, fl, slots, DIAG_FRAME_WIRE_SIZE);
}

static void diag_emit_trace_end(void)
{
    diag_capture_meta_t meta;
    uint8_t end_fl;

    diag_trace_snapshot(&meta);
    /* START: valid LE; END: write_index LE; overflow in bit7 (not state — avoids EP_START clash). */
    (void)diag_try_emit(DIAG_TRACE_END, DIAG_EP_START,
        (uint8_t)(meta.valid & 0xffu),
        (uint8_t)((meta.valid >> 8) & 0xffu));
    end_fl = (uint8_t)(DIAG_EP_END | (meta.overflow ? 0x80u : 0u));
    (void)diag_try_emit(DIAG_TRACE_END, end_fl,
        (uint8_t)(meta.write_index & 0xffu),
        (uint8_t)((meta.write_index >> 8) & 0xffu));
}

void diag_emit_sck_config(void)
{
    uint8_t tr = diag_transport();

    if (diag_sck_seen && effective_sck == diag_sck_last && tr == diag_tr_last)
        return;
    diag_sck_seen = 1;
    diag_sck_last = effective_sck;
    diag_tr_last = tr;
    (void)diag_try_emit(DIAG_SCK_CONFIG, 0, effective_sck, tr);
}

void diag_emit_enableprog(const uint8_t tx[4], const uint8_t rx[4], uint8_t fail)
{
    uint8_t end = (uint8_t)(DIAG_EP_END
        | (fail ? DIAG_EP_RESULT_FAIL : DIAG_EP_RESULT_OK));

    (void)diag_try_emit(DIAG_ENABLEPROG, DIAG_EP_START, tx[0], tx[1]);
    (void)diag_try_emit(DIAG_ENABLEPROG, DIAG_EP_CONT, tx[2], tx[3]);
    (void)diag_try_emit(DIAG_ENABLEPROG, DIAG_EP_CONT, rx[0], rx[1]);
    (void)diag_try_emit(DIAG_ENABLEPROG, end, rx[2], rx[3]);
}

void diag_publish_snapshot(const diag_snapshot_t *s)
{
    uint8_t end;
    memcpy(&diag_fault_snapshot, s, sizeof(diag_fault_snapshot));

    (void)diag_try_emit(DIAG_FAULT_SNAPSHOT, DIAG_EP_START,
        (uint8_t)((diag_fault_snapshot.sck_req << 4)
                  | (diag_fault_snapshot.effective_sck & 0x0fu)),
        diag_fault_snapshot.transport);
    (void)diag_try_emit(DIAG_FAULT_SNAPSHOT, DIAG_EP_CONT,
        diag_fault_snapshot.reset_driven, diag_fault_snapshot.state);
    (void)diag_try_emit(DIAG_FAULT_SNAPSHOT, DIAG_EP_CONT,
        diag_fault_snapshot.tx[0], diag_fault_snapshot.tx[1]);
    end = (uint8_t)(DIAG_EP_END
        | (diag_fault_snapshot.result ? DIAG_EP_RESULT_FAIL
                                      : DIAG_EP_RESULT_OK));
    (void)diag_try_emit(DIAG_FAULT_SNAPSHOT, end,
        diag_fault_snapshot.rx[0], diag_fault_snapshot.sw_delay);
}

void diag_note_enableprog_try(uint8_t path_flags, uint8_t check)
{
    if (diag_transport() != DIAG_TRANSPORT_SW)
        return;
    (void)diag_try_emit(DIAG_ERROR, path_flags, check, sck_sw_delay);
}

void diag_memop_begin(uint8_t mem, uint8_t pagesize)
{
    if (diag_mem_open)
        return;
    diag_mem_open = 1;
    diag_mem_pages = 0;
    (void)diag_try_emit(DIAG_MEMOP, DIAG_EP_START, mem, pagesize);
}

void diag_memop_page(void)
{
    if (diag_mem_pages < 255)
        diag_mem_pages++;
    if (diag_mem_open) {
        (void)diag_try_emit(DIAG_MEMOP,
            (uint8_t)(DIAG_EP_END | DIAG_EP_RESULT_OK),
            DIAG_MEM_FLASH, diag_mem_pages);
    }
}

void diag_memop_end(uint8_t mem)
{
    if (!diag_mem_open)
        return;
    diag_mem_open = 0;
    (void)diag_try_emit(DIAG_MEMOP,
        (uint8_t)(DIAG_EP_END | DIAG_EP_RESULT_OK), mem, diag_mem_pages);
}

void diag_report_enableprog(const uint8_t tx[4], const uint8_t rx[4], uint8_t fail)
{
    diag_emit_enableprog(tx, rx, fail);
    if (!fail)
        return;

    {
        diag_snapshot_t local;

        local.sck_req = requested_sck;
        local.effective_sck = effective_sck;
        local.transport = diag_transport();
        local.reset_driven = diag_reset_driven;
        local.state = prog_state;
        local.result = 1;
        local.sw_delay = sck_sw_delay;
        memcpy(local.tx, tx, 4);
        memcpy(local.rx, rx, 4);
        diag_publish_snapshot(&local);
    }
}

void diag_on_connect(void)
{
    uint32_t fcap;
    uint32_t bcap;

    diag_ensure_trace();
    diag_trace_arm();

    diag_sck_seen = 0;
    (void)diag_try_emit(
        DIAG_HELLO,
        (uint8_t)(DIAG_CAP_SESSION | DIAG_CAP_TRANSACTION | DIAG_CAP_SNAPSHOT
                  | DIAG_CAP_TIMESTAMP | DIAG_CAP_TRACE),
        DIAG_SCHEMA_V1,
        DIAG_PROFILE_COMPOSITE);

    fcap = DIAG_FCAP_SESSION | DIAG_FCAP_SNAPSHOT | DIAG_FCAP_TIMESTAMP
        | DIAG_FCAP_TRACE;
    bcap = 0;
#if USBASP_HAS_SCK_JUMPER
    bcap |= BOARD_CAP_SCK_JUMPER;
#endif
    (void)diag_try_emit(DIAG_CAPS, DIAG_EP_START,
        (uint8_t)(fcap & 0xffu), (uint8_t)((fcap >> 8) & 0xffu));
    (void)diag_try_emit(DIAG_CAPS, DIAG_EP_CONT,
        (uint8_t)((fcap >> 16) & 0xffu), (uint8_t)((fcap >> 24) & 0xffu));
    (void)diag_try_emit(DIAG_CAPS, DIAG_EP_CONT,
        (uint8_t)(bcap & 0xffu), (uint8_t)((bcap >> 8) & 0xffu));
    (void)diag_try_emit(DIAG_CAPS, DIAG_EP_END,
        (uint8_t)((bcap >> 16) & 0xffu), (uint8_t)((bcap >> 24) & 0xffu));

    diag_emit_trace_begin();

    (void)diag_try_emit(DIAG_SESSION_BEGIN, 0, requested_sck, effective_sck);
    diag_emit_sck_config();
    diag_reset_driven = DIAG_RESET_ASSERT;
    (void)diag_try_emit(DIAG_RESET, DIAG_RESET_ASSERT, 0, 0);
}

void diag_on_disconnect(void)
{
    diag_memop_end(DIAG_MEM_FLASH);
    diag_reset_driven = DIAG_RESET_RELEASE;
    (void)diag_try_emit(DIAG_RESET, DIAG_RESET_RELEASE, 0, 0);
    diag_emit_trace_end();
    (void)diag_try_emit(DIAG_SESSION_END, 0, 0, 0);
    diag_trace_idle();
}

uint8_t diag_poll_drain(uint8_t out[8])
{
    diag_ensure_trace();
    return diag_trace_drain(out);
}

#endif /* USBASP_HAS_DIAG */
