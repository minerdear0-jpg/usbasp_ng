#ifndef USBASP_DIAG_EVENTS_H_
#define USBASP_DIAG_EVENTS_H_

/*
 * Diagplane **protocol** schema v1 — EP2 wire types (host presents text).
 * Host client (diagplane binary) is versioned separately; bump SCHEMA only
 * for incompatible wire changes.
 */

#define DIAG_SCHEMA_V1              1

#define DIAG_HELLO                  1
#define DIAG_SESSION_BEGIN          2
#define DIAG_SESSION_END            3
#define DIAG_RESET                  4
#define DIAG_SCK_CONFIG             5
#define DIAG_ENABLEPROG             6  /* PR2 */
#define DIAG_SPI_BYTE               7  /* TRACE later */
#define DIAG_SCK_STATS              8  /* P2 */
#define DIAG_FAULT_SNAPSHOT         9  /* PR2 */
#define DIAG_TRACE_OVERFLOW         10
#define DIAG_ERROR                  11
#define DIAG_MEMOP                  12  /* flash/eeprom/read block markers */
#define DIAG_CAPS                   13  /* firmware + board capability bitsets */
#define DIAG_TRACE_BEGIN            14  /* capture metadata: arm / slots */
#define DIAG_TRACE_END              15  /* capture metadata: valid / overflow */
#define DIAG_ISP_PINS               16  /* programmer ISP pin DDR/PIN after disconnect */
#define DIAG_LINE_FAULT             17  /* PINx did not follow PORT after drive */

/* DIAG_ERROR flags — ENABLEPROG attempt note (B8/B22 forensics) */
#define DIAG_ERR_EP_AVR             0x01  /* check after AC 53 00 00 (expect 0x53) */
#define DIAG_ERR_EP_AT89            0x02  /* check after AT89 path (expect 0x69) */

/*
 * DIAG_MEMOP — flash/eeprom/read block markers (not per-byte TRACE).
 *   START:           a=mem, b=pagesize (sat 255)
 *   CONT|OK/FAIL:    a:b = page base address low 16 (byte addr)
 *   END|OK/FAIL:     a=mem, b=pages flushed (sat 255)
 */
#define DIAG_MEM_FLASH              0
#define DIAG_MEM_EEPROM             1
#define DIAG_MEM_READFLASH          2

/* DIAG_RESET flags — programmer drive intent, not pin sense */
#define DIAG_RESET_ASSERT           0x01
#define DIAG_RESET_RELEASE          0x02

/*
 * DIAG_ISP_PINS — after ispDisconnect (Hi-Z claim).
 * flags: DIAG_PINS_AFTER_DISC | OK/FAIL
 * FAIL if MOSI/SCK/RST still outputs (DDR bit set).
 * a = DDRB & ISP mask (RST|MOSI|MISO|SCK)
 * b = PINB & ISP mask
 */
#define DIAG_PINS_AFTER_DISC        0x01

/* DIAG_LINE_FAULT — PINx vs last PORT write (not a sniffer).
 * flags: DRIVE_HIGH or DRIVE_LOW | OK/FAIL
 * a = bit (PB2 RST, PB3 MOSI, PB5 SCK); OK summary: a = drive mask
 * b = PINB & ISP_mask
 */
#define DIAG_LINE_DRIVE_HIGH        0x01
#define DIAG_LINE_DRIVE_LOW         0x02

/* DIAG_ENABLEPROG / DIAG_CAPS sequence flags */
#define DIAG_EP_START               0x01
#define DIAG_EP_CONT                0x02
#define DIAG_EP_END                 0x04
#define DIAG_EP_RESULT_OK           0x10
#define DIAG_EP_RESULT_FAIL         0x20

/*
 * DIAG_HELLO.flags — legacy compact uint8 (stable).
 * Full bitsets ride DIAG_CAPS (uint32 LE firmware + board).
 */
#define DIAG_CAP_SESSION            0x01
#define DIAG_CAP_TRANSACTION        0x02
#define DIAG_CAP_SNAPSHOT           0x04
#define DIAG_CAP_TRACE              0x08
#define DIAG_CAP_SCK_STATS          0x10
#define DIAG_CAP_TIMESTAMP          0x20

/* Firmware diagnostics capabilities (uint32, DIAG_CAPS frames 0..1) */
#define DIAG_FCAP_SESSION           (1u << 0)
#define DIAG_FCAP_SNAPSHOT          (1u << 1)
#define DIAG_FCAP_TIMESTAMP         (1u << 2)
#define DIAG_FCAP_TRACE             (1u << 3)
#define DIAG_FCAP_TRIGGER           (1u << 4)
#define DIAG_FCAP_PRETRIGGER        (1u << 5)
#define DIAG_FCAP_SCK_STATS         (1u << 6)
#define DIAG_FCAP_LINE_FAULT        (1u << 7)

/* Board / physical capabilities (uint32, DIAG_CAPS frames 2..3) */
#define BOARD_CAP_TARGET_UART       (1u << 0)
#define BOARD_CAP_SCK_JUMPER        (1u << 1)
#define BOARD_CAP_PHYSICAL_CAPTURE  (1u << 2)

#define DIAG_PROFILE_UNKNOWN        0
#define DIAG_PROFILE_COMPOSITE      1  /* hiduart product image */

#define DIAG_TRANSPORT_HW           0
#define DIAG_TRANSPORT_SW           1

#endif
