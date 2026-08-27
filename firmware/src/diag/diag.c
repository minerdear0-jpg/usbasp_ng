#include "usbasp_config.h"

#if USBASP_HAS_DIAG

#include "diag/diag.h"
#include "diag/diag_ring.h"
#include "diag/diag_clock.h"
#include "diag/diag_trigger.h"
#include "usbasp/clock.h"
#include "usbasp/isp.h"
#include "usbasp/sck.h"
#include "usbasp/prog_state.h"

#include <avr/io.h>
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
static uint8_t diag_mem_psize; /* pagesize from START (sat 255); 0 = byte mode */
static uint8_t diag_mem_kind;  /* DIAG_MEM_* of open op */
static uint8_t diag_mem_open;
static uint8_t diag_mem_last_emitted; /* pages count at last CONT emit */
static uint16_t diag_mem_last_base;   /* low 16 of last page/chunk base */
static uint8_t diag_trace_inited;

#if USBASP_DIAG_MEMOP_PAGES
/* Ring is finite; full mega8 write ≈128 flushes. Keep FAIL + canary + stride. */
#ifndef DIAG_MEMOP_PAGE_STRIDE
#define DIAG_MEMOP_PAGE_STRIDE 8
#endif
/* ATmega8 oracle canary: 8×64 B @ 0x1E00..0x1FFF */
#define DIAG_MEMOP_CANARY_LO 0x1E00u
#define DIAG_MEMOP_CANARY_HI 0x2000u

static uint8_t diag_memop_page_emit_p(uint16_t base, uint8_t fail)
{
    uint16_t idx;

    if (fail)
        return 1;
    if (diag_mem_pages <= 1)
        return 1;
    if (base >= DIAG_MEMOP_CANARY_LO && base < DIAG_MEMOP_CANARY_HI)
        return 1;
    if (diag_mem_psize == 0)
        return 1;
    idx = (uint16_t)(base / diag_mem_psize);
    return (uint8_t)((idx % DIAG_MEMOP_PAGE_STRIDE) == 0);
}
#endif

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
    uint8_t fl;

    diag_trace_snapshot(&meta);
    /* 4 frames — keep 6-byte wire; no schema bump.
     * F0 START: valid LE
     * F1 CONT:  write_index LE; overflow bit7
     * F2 CONT:  trigger_kind, post_count; fired bit7
     * F3 END:   trigger_index LE; frame.timestamp = trigger_timestamp
     */
    (void)diag_try_emit(DIAG_TRACE_END, DIAG_EP_START,
        (uint8_t)(meta.valid & 0xffu),
        (uint8_t)((meta.valid >> 8) & 0xffu));
    fl = (uint8_t)(DIAG_EP_CONT | (meta.overflow ? 0x80u : 0u));
    (void)diag_try_emit(DIAG_TRACE_END, fl,
        (uint8_t)(meta.write_index & 0xffu),
        (uint8_t)((meta.write_index >> 8) & 0xffu));
    fl = (uint8_t)(DIAG_EP_CONT | (meta.triggered ? 0x80u : 0u));
    (void)diag_try_emit(DIAG_TRACE_END, fl,
        meta.trigger_kind, meta.post_count);
    {
        diag_frame_t endf;

        endf.type = DIAG_TRACE_END;
        endf.flags = DIAG_EP_END;
        endf.timestamp = meta.trigger_timestamp;
        endf.a = (uint8_t)(meta.trigger_index & 0xffu);
        endf.b = (uint8_t)((meta.trigger_index >> 8) & 0xffu);
        (void)diag_trace_push(&endf);
    }
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
    /*
     * Coalesce successive chunks of the same mem op (avrdude may set FIRST
     * on every WRITEFLASH block; READFLASH is many setups).
     * Switching kind (esp. WRITE→READ verify) finishes the previous op.
     */
    if (diag_mem_open) {
        if (diag_mem_kind == mem)
            return;
        diag_memop_end(diag_mem_kind);
    }
    diag_mem_open = 1;
    diag_mem_kind = mem;
    diag_mem_pages = 0;
    diag_mem_psize = pagesize;
    diag_mem_last_emitted = 0;
    diag_mem_last_base = 0;
    (void)diag_try_emit(DIAG_MEMOP, DIAG_EP_START, mem, pagesize);
}

void diag_memop_page(uint32_t address, uint8_t fail)
{
#if !USBASP_DIAG_MEMOP_PAGES
    (void)address;
    (void)fail;
    if (diag_mem_pages < 255)
        diag_mem_pages++;
#else
    uint32_t base = address;
    uint8_t flags;
    uint16_t base16;

    if (diag_mem_pages < 255)
        diag_mem_pages++;
    if (!diag_mem_open)
        return;
    /* Write flushes: align to page base. Read chunks: keep start address. */
    if (diag_mem_psize != 0 && diag_mem_kind == DIAG_MEM_FLASH)
        base = address - (address % diag_mem_psize);
    base16 = (uint16_t)base;
    diag_mem_last_base = base16;
    if (!diag_memop_page_emit_p(base16, fail))
        return;
    diag_mem_last_emitted = diag_mem_pages;
    flags = (uint8_t)(DIAG_EP_CONT
        | (fail ? DIAG_EP_RESULT_FAIL : DIAG_EP_RESULT_OK));
    (void)diag_try_emit(DIAG_MEMOP, flags,
        (uint8_t)(base16 >> 8), (uint8_t)base16);
#endif
}

void diag_memop_end(uint8_t mem)
{
    (void)mem;
    if (!diag_mem_open)
        return;
#if USBASP_DIAG_MEMOP_PAGES
    /* Ensure last page/chunk is visible even if stride skipped it. */
    if (diag_mem_pages != 0 && diag_mem_last_emitted != diag_mem_pages) {
        diag_mem_last_emitted = diag_mem_pages;
        (void)diag_try_emit(DIAG_MEMOP,
            (uint8_t)(DIAG_EP_CONT | DIAG_EP_RESULT_OK),
            (uint8_t)(diag_mem_last_base >> 8),
            (uint8_t)diag_mem_last_base);
    }
#endif
    diag_mem_open = 0;
    (void)diag_try_emit(DIAG_MEMOP,
        (uint8_t)(DIAG_EP_END | DIAG_EP_RESULT_OK),
        diag_mem_kind, diag_mem_pages);
}

void diag_report_enableprog(const uint8_t tx[4], const uint8_t rx[4], uint8_t fail)
{
    diag_emit_enableprog(tx, rx, fail);
#if !USBASP_HAS_DIAG_TRIGGER
    (void)fail;
#else
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
#endif
}

void diag_on_connect(void)
{
    diag_ensure_trace();
    diag_trace_arm();

    diag_sck_seen = 0;
    (void)diag_try_emit(
        DIAG_HELLO,
        (uint8_t)(DIAG_CAP_SESSION | DIAG_CAP_TRANSACTION | DIAG_CAP_SNAPSHOT
                  | DIAG_CAP_TIMESTAMP | DIAG_CAP_TRACE),
        DIAG_SCHEMA_V1,
        DIAG_PROFILE_COMPOSITE);

#if USBASP_HAS_DIAG_TRIGGER
    {
        uint32_t fcap;
        uint32_t bcap;

        fcap = DIAG_FCAP_SESSION | DIAG_FCAP_SNAPSHOT | DIAG_FCAP_TIMESTAMP
            | DIAG_FCAP_TRACE | DIAG_FCAP_LINE_FAULT;
        fcap |= DIAG_FCAP_TRIGGER | DIAG_FCAP_PRETRIGGER;
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
    }
#endif

    diag_emit_trace_begin();

    (void)diag_try_emit(DIAG_SESSION_BEGIN, 0, requested_sck, effective_sck);
    diag_emit_sck_config();
    diag_reset_driven = DIAG_RESET_ASSERT;
    (void)diag_try_emit(DIAG_RESET, DIAG_RESET_ASSERT, 0, 0);
}

void diag_emit_isp_pins(void)
{
#if !USBASP_DIAG_MEMOP_PAGES
    return;
#else
    uint8_t mask = (uint8_t)((1u << ISP_RST) | (1u << ISP_MOSI)
        | (1u << ISP_MISO) | (1u << ISP_SCK));
    uint8_t ddr = (uint8_t)(ISP_DDR & mask);
    uint8_t pin = (uint8_t)(ISP_IN & mask);
    uint8_t drive = (uint8_t)((1u << ISP_RST) | (1u << ISP_MOSI)
        | (1u << ISP_SCK));
    uint8_t flags = DIAG_PINS_AFTER_DISC;

    /* Hi-Z claim: RST/MOSI/SCK must not remain outputs. */
    if ((ddr & drive) != 0)
        flags |= DIAG_EP_RESULT_FAIL;
    else
        flags |= DIAG_EP_RESULT_OK;
    (void)diag_try_emit(DIAG_ISP_PINS, flags, ddr, pin);
#endif
}

void diag_emit_line_fault(uint8_t bit_or_mask, uint8_t flags, uint8_t pin_sample)
{
    (void)diag_try_emit(DIAG_LINE_FAULT, flags, bit_or_mask, pin_sample);
}

void diag_on_disconnect(void)
{
    diag_memop_end(DIAG_MEM_FLASH);
    /* Footer first while freeze latch is still readable. */
    diag_emit_trace_end();
    diag_reset_driven = DIAG_RESET_RELEASE;
    (void)diag_try_emit(DIAG_RESET, DIAG_RESET_RELEASE, 0, 0);
    /* ispDisconnect() already ran — sample Hi-Z claim. */
    diag_emit_isp_pins();
    (void)diag_try_emit(DIAG_SESSION_END, 0, 0, 0);
    diag_trace_idle();
}

uint8_t diag_poll_drain(uint8_t out[8])
{
    diag_ensure_trace();
    return diag_trace_drain(out);
}

#endif /* USBASP_HAS_DIAG */
