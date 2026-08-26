#ifndef USBASP_DIAG_EVENTS_H_
#define USBASP_DIAG_EVENTS_H_

/* USBASP-NG DIAG v1 — binary wire types (host presents text). */

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

/* DIAG_ERROR flags — ENABLEPROG attempt note (B8/B22 forensics) */
#define DIAG_ERR_EP_AVR             0x01  /* check after AC 53 00 00 (expect 0x53) */
#define DIAG_ERR_EP_AT89            0x02  /* check after AT89 path (expect 0x69) */

/* DIAG_RESET flags — programmer drive intent, not pin sense */
#define DIAG_RESET_ASSERT           0x01
#define DIAG_RESET_RELEASE          0x02

/* DIAG_ENABLEPROG flags (PR2) */
#define DIAG_EP_START               0x01
#define DIAG_EP_CONT                0x02
#define DIAG_EP_END                 0x04
#define DIAG_EP_RESULT_OK           0x10
#define DIAG_EP_RESULT_FAIL         0x20

/* DIAG_HELLO flags — internal diag caps, not FUNC 127 */
#define DIAG_CAP_SESSION            0x01
#define DIAG_CAP_TRANSACTION        0x02
#define DIAG_CAP_SNAPSHOT           0x04
#define DIAG_CAP_TRACE              0x08
#define DIAG_CAP_SCK_STATS          0x10

#define DIAG_PROFILE_UNKNOWN        0
#define DIAG_PROFILE_COMPOSITE      1  /* hiduart product image */

#define DIAG_TRANSPORT_HW           0
#define DIAG_TRANSPORT_SW           1

#endif
