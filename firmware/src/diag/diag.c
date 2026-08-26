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
 * Lossy SPSC diagnostics ring. Overflow: drop + overflow_pending;
 * DIAG_TRACE_OVERFLOW is delivered from diag_poll_drain only (never
 * pushed while the ring is full).
 *
 * Timestamps: Timer1 monotonic diag_now(); P0/RC wire uses low 16 bits only.
 */

static diag_frame_t diag_frames[DIAG_RING_SIZE];
static volatile uint8_t diag_head; /* next write */
static volatile uint8_t diag_tail; /* next read */
static uint8_t diag_dropped;      /* saturating 0..255 */
static uint8_t diag_overflow_pending;
static uint8_t diag_reset_driven;
static diag_snapshot_t diag_fault_snapshot;
static uint8_t diag_sck_seen;
static uint8_t diag_sck_last;
static uint8_t diag_tr_last;
static uint8_t diag_mem_pages; /* saturating page-flush count for MEMOP END */
static uint8_t diag_mem_open;

extern uchar requested_sck;
extern uchar effective_sck;

static uint8_t diag_ring_len(void)
{
    return (uint8_t)(diag_head - diag_tail);
}

static uint8_t diag_transport(void)
{
    return (isp_bus.transfer == ispTransmit_sw)
        ? (uint8_t)DIAG_TRANSPORT_SW
        : (uint8_t)DIAG_TRANSPORT_HW;
}

static void diag_pack(uint8_t out[8], uint8_t type, uint8_t flags,
                      uint16_t ts, uint8_t a, uint8_t b)
{
    out[0] = type;
    out[1] = flags;
    out[2] = (uint8_t)(ts & 0xff);
    out[3] = (uint8_t)(ts >> 8);
    out[4] = a;
    out[5] = b;
    out[6] = 0;
    out[7] = prog_state;
}

bool diag_try_emit(uint8_t type, uint8_t flags, uint8_t a, uint8_t b)
{
    if (diag_ring_len() >= DIAG_RING_SIZE) {
        if (diag_dropped < 255)
            diag_dropped++;
        diag_overflow_pending = 1;
        return false;
    }

    diag_frame_t *f = &diag_frames[diag_head & (DIAG_RING_SIZE - 1)];
    f->type = type;
    f->flags = flags;
    f->timestamp = diag_now_wire16();
    f->a = a;
    f->b = b;
    diag_head++;
    return true;
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
    /* C3: copy immediately; never retain caller pointer.
     * Compact 4-frame wire. Full TX/RX also on ENABLEPROG. */
    uint8_t end;
    memcpy(&diag_fault_snapshot, s, sizeof(diag_fault_snapshot));

    /* F1: sck_req[7:4]|effective_sck[3:0], transport */
    (void)diag_try_emit(DIAG_FAULT_SNAPSHOT, DIAG_EP_START,
        (uint8_t)((diag_fault_snapshot.sck_req << 4)
                  | (diag_fault_snapshot.effective_sck & 0x0fu)),
        diag_fault_snapshot.transport);
    /* F2: RESET drive + FSM state */
    (void)diag_try_emit(DIAG_FAULT_SNAPSHOT, DIAG_EP_CONT,
        diag_fault_snapshot.reset_driven, diag_fault_snapshot.state);
    /* F3: TX[0..1] (TX[2..3] usually 00 00 for ENABLEPROG) */
    (void)diag_try_emit(DIAG_FAULT_SNAPSHOT, DIAG_EP_CONT,
        diag_fault_snapshot.tx[0], diag_fault_snapshot.tx[1]);
    /* F4: RX[0] + sw_delay; result in END|OK/FAIL */
    end = (uint8_t)(DIAG_EP_END
        | (diag_fault_snapshot.result ? DIAG_EP_RESULT_FAIL
                                      : DIAG_EP_RESULT_OK));
    (void)diag_try_emit(DIAG_FAULT_SNAPSHOT, end,
        diag_fault_snapshot.rx[0], diag_fault_snapshot.sw_delay);
}

void diag_note_enableprog_try(uint8_t path_flags, uint8_t check)
{
    /* SW forensics only — HW last-try notes fill the ring before PASS. */
    if (diag_transport() != DIAG_TRANSPORT_SW)
        return;
    (void)diag_try_emit(DIAG_ERROR, path_flags, check, sck_sw_delay);
}

void diag_memop_begin(uint8_t mem, uint8_t pagesize)
{
    /* avrdude often sets FIRST on every page; one START per open write. */
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
    /* Emit per flush — LAST flag is unreliable across avrdude versions. */
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
    diag_sck_seen = 0; /* force SCK_CONFIG once per session */
    (void)diag_try_emit(
        DIAG_HELLO,
        (uint8_t)(DIAG_CAP_SESSION | DIAG_CAP_TRANSACTION | DIAG_CAP_SNAPSHOT),
        DIAG_SCHEMA_V1,
        DIAG_PROFILE_COMPOSITE);
    (void)diag_try_emit(DIAG_SESSION_BEGIN, 0, requested_sck, effective_sck);
    diag_emit_sck_config();
    diag_reset_driven = DIAG_RESET_ASSERT;
    (void)diag_try_emit(DIAG_RESET, DIAG_RESET_ASSERT, 0, 0);
}

void diag_on_disconnect(void)
{
    diag_memop_end(DIAG_MEM_FLASH); /* close open write if LAST was missing */
    diag_reset_driven = DIAG_RESET_RELEASE;
    (void)diag_try_emit(DIAG_RESET, DIAG_RESET_RELEASE, 0, 0);
    (void)diag_try_emit(DIAG_SESSION_END, 0, 0, 0);
}

uint8_t diag_poll_drain(uint8_t out[8])
{
    diag_frame_t f;
    uint16_t ts = diag_now_wire16();

    if (diag_ring_len() == 0) {
        if (!diag_overflow_pending)
            return 0;
        diag_pack(out, DIAG_TRACE_OVERFLOW, 0, ts, diag_dropped, 0);
        diag_dropped = 0;
        diag_overflow_pending = 0;
        return 1;
    }

    f = diag_frames[diag_tail & (DIAG_RING_SIZE - 1)];
    diag_tail++;
    diag_pack(out, f.type, f.flags, f.timestamp, f.a, f.b);
    return 1;
}

#endif /* USBASP_HAS_DIAG */
