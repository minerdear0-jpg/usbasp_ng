#ifndef USBASP_DIAG_H_
#define USBASP_DIAG_H_

#include "usbasp_config.h"
#include "diag/diag_events.h"
#include "diag/diag_ring.h"

#if USBASP_HAS_DIAG

#include <stdbool.h>
#include <stdint.h>

typedef struct {
    uint8_t sck_req;       /* host requested_sck id */
    uint8_t effective_sck;
    uint8_t transport;
    uint8_t reset_driven; /* last RESET_ASSERT/RELEASE flag */
    uint8_t state;
    uint8_t result;
    uint8_t sw_delay;     /* sck_sw_delay at fault (SW path half-period units) */
    uint8_t tx[4];
    uint8_t rx[4];
} diag_snapshot_t;

/* Returns false if dropped. Callers MUST NOT retry or wait. */
bool diag_try_emit(uint8_t type, uint8_t flags, uint8_t a, uint8_t b);

/* CONNECT lifecycle: HELLO, CAPS×4, SESSION_BEGIN, SCK_CONFIG, RESET_ASSERT */
void diag_on_connect(void);

/* DISCONNECT: RESET_RELEASE, SESSION_END */
void diag_on_disconnect(void);

/* After SETISPSCK / apply: effective id + HW/SW transport */
void diag_emit_sck_config(void);

/* Four-frame semantic ENABLEPROG (TX×4 / RX×4 / result). */
void diag_emit_enableprog(const uint8_t tx[4], const uint8_t rx[4], uint8_t fail);

/*
 * Copies *s into persistent diag RAM immediately; never retains the pointer.
 * Then emits 4 compact DIAG_FAULT_SNAPSHOT frames from that copy.
 */
void diag_publish_snapshot(const diag_snapshot_t *s);

/* Emit ENABLEPROG; on fail also publish an atomic fault snapshot. */
void diag_report_enableprog(const uint8_t tx[4], const uint8_t rx[4], uint8_t fail);

/* After a failed enableprog exchange: check byte + sck_sw_delay (lossy).
 * SW path only — HW notes are noise before autoslow/PASS. */
void diag_note_enableprog_try(uint8_t path_flags, uint8_t check);

/* WRITEFLASH / READFLASH: START, CONT@page|chunk, END (kind from open op). */
void diag_memop_begin(uint8_t mem, uint8_t pagesize);
void diag_memop_page(uint32_t address, uint8_t fail);
void diag_memop_end(uint8_t mem); /* mem ignored; emits stored kind */

/* After ispDisconnect: DDR/PIN sample for dual-truth vs target ISP_PINS. */
void diag_emit_isp_pins(void);

/*
 * Consumer: fill out[8] with one frame (bytes 0..5) + pad/status.
 * Returns 1 if a frame was written, 0 if nothing to send (silence).
 */
uint8_t diag_poll_drain(uint8_t out[8]);

#else /* !USBASP_HAS_DIAG */

#define diag_try_emit(type, flags, a, b) (0)
#define diag_on_connect() ((void)0)
#define diag_on_disconnect() ((void)0)
#define diag_emit_sck_config() ((void)0)
#define diag_emit_enableprog(tx, rx, fail) ((void)0)
#define diag_publish_snapshot(s) ((void)0)
#define diag_report_enableprog(tx, rx, fail) ((void)0)
#define diag_note_enableprog_try(path, check) ((void)0)
#define diag_memop_begin(mem, pagesize) ((void)0)
#define diag_memop_page(addr, fail) ((void)0)
#define diag_memop_end(mem) ((void)0)
#define diag_emit_isp_pins() ((void)0)
#define diag_poll_drain(out) (0)

#endif

#endif
