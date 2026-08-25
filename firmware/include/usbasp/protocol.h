/*
 * USBasp NG — host-visible protocol (avrdude / Fischl 2011).
 * L0–L2 must not change without a compatibility review.
 * See docs/COMPATIBILITY.md
 */

#ifndef USBASP_PROTOCOL_H_
#define USBASP_PROTOCOL_H_

#include <stdint.h>

#ifndef uchar
#define uchar unsigned char
#endif

/* USB identity (L0) */
#define USBASP_VID 0x16c0
#define USBASP_PID 0x05dc

/* USB function call identifiers (bRequest) */
#define USBASP_FUNC_CONNECT         1
#define USBASP_FUNC_DISCONNECT      2
#define USBASP_FUNC_TRANSMIT        3
#define USBASP_FUNC_READFLASH       4
#define USBASP_FUNC_ENABLEPROG      5
#define USBASP_FUNC_WRITEFLASH      6
#define USBASP_FUNC_READEEPROM      7
#define USBASP_FUNC_WRITEEEPROM     8
#define USBASP_FUNC_SETLONGADDRESS  9
#define USBASP_FUNC_SETISPSCK       10
#define USBASP_FUNC_TPI_CONNECT     11
#define USBASP_FUNC_TPI_DISCONNECT  12
#define USBASP_FUNC_TPI_RAWREAD     13
#define USBASP_FUNC_TPI_RAWWRITE    14
#define USBASP_FUNC_TPI_READBLOCK   15
#define USBASP_FUNC_TPI_WRITEBLOCK  16
#define USBASP_FUNC_GETCAPABILITIES 127

/* Capability bitmap as avrdude packs 4 LE bytes:
 *   caps = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
 * Classic GETCAPABILITIES must be: TPI in byte 0, zeros in 1–2,
 * USBASP_CAP_3MHZ in byte 3 when the board can do 3 MHz SCK.
 */
#define USBASP_CAP_TPI   0x01
#define USBASP_CAP_3MHZ  (1UL << 24)

/* Programming state (internal) */
#define PROG_STATE_IDLE        0
#define PROG_STATE_WRITEFLASH  1
#define PROG_STATE_READFLASH   2
#define PROG_STATE_READEEPROM  3
#define PROG_STATE_WRITEEEPROM 4
#define PROG_STATE_TPI_READ    5
#define PROG_STATE_TPI_WRITE   6

#define PROG_BLOCKFLAG_FIRST 1
#define PROG_BLOCKFLAG_LAST  2

/* ISP SCK speed identifiers (SETISPSCK wValue low byte) */
#define USBASP_ISP_SCK_AUTO   0
#define USBASP_ISP_SCK_0_5    1
#define USBASP_ISP_SCK_1      2
#define USBASP_ISP_SCK_2      3
#define USBASP_ISP_SCK_4      4
#define USBASP_ISP_SCK_8      5
#define USBASP_ISP_SCK_16     6
#define USBASP_ISP_SCK_32     7
#define USBASP_ISP_SCK_93_75  8
#define USBASP_ISP_SCK_187_5  9
#define USBASP_ISP_SCK_375    10
#define USBASP_ISP_SCK_750    11
#define USBASP_ISP_SCK_1500   12
#define USBASP_ISP_SCK_3000   13

#endif /* USBASP_PROTOCOL_H_ */
