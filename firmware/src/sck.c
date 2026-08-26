#include <avr/io.h>
#include <util/delay_basic.h>
#include "usbasp_config.h"
#include "usbasp/sck.h"
#include "usbasp/isp.h"
#include "usbasp/clock.h"
#include "usbasp/board.h"

uchar sck_sw_delay;
/*
 * SCK ownership:
 *   requested_sck  — last SETISPSCK from the host (AUTO allowed)
 *   effective_sck  — id actually applied to the wire (after jumper / AUTO / autoslow)
 * CONNECT / SETISPSCK / DISCONNECT must not write jumper or autoslow into requested_sck.
 */
uchar effective_sck = USBASP_ISP_SCK_1500;

extern uchar requested_sck;

void isp_spi_hw_enable(void)
{
    SPCR |= (1 << SPE) | (1 << MSTR);
}

void isp_spi_hw_disable(void)
{
    SPCR = 0;
}

void isp_sck_delay(void)
{
    /* Minimum half-period. INT0 (USB) may only stretch; it must not shorten.
     * usbFunctionSetup runs in usbPoll() with I=1 — see docs/COMPATIBILITY.md L2.5. */
    _delay_loop_2((uint16_t)sck_sw_delay * 16u);
}

uchar isp_sck_autoslow(uchar sck)
{
    if (sck > USBASP_ISP_SCK_375)
        return USBASP_ISP_SCK_375;
    if (sck > USBASP_ISP_SCK_93_75)
        return USBASP_ISP_SCK_93_75;
    if (sck > USBASP_ISP_SCK_16)
        return USBASP_ISP_SCK_16;
    if (sck > USBASP_ISP_SCK_0_5)
        return USBASP_ISP_SCK_0_5;
    return USBASP_ISP_SCK_AUTO;
}

void ispSetSCKOption(uchar option)
{
    if (option == USBASP_ISP_SCK_AUTO)
        option = USBASP_ISP_SCK_1500;

#if !USBASP_HAS_3MHZ
    if (option == USBASP_ISP_SCK_3000)
        option = USBASP_ISP_SCK_1500;
#endif

    if (option >= USBASP_ISP_SCK_93_75) {
        isp_bus_select_hw();
        SPSR = 0;
        sck_sw_delay = 1;

        switch (option) {
        case USBASP_ISP_SCK_3000:
            SPCR = 0;
            break;
        case USBASP_ISP_SCK_1500:
            SPSR = (1 << SPI2X);
            SPCR = (1 << SPR0);
            break;
        case USBASP_ISP_SCK_750:
            SPCR = (1 << SPR0);
            break;
        case USBASP_ISP_SCK_375:
            SPSR = (1 << SPI2X);
            SPCR = (1 << SPR1);
            break;
        case USBASP_ISP_SCK_187_5:
            SPCR = (1 << SPR1);
            break;
        case USBASP_ISP_SCK_93_75:
        default:
            SPCR = (1 << SPR1) | (1 << SPR0);
            break;
        }
    } else {
        SPCR = 0;
        isp_bus_select_sw();
        ISP_OUT &= ~(1 << ISP_SCK);
        sck_sw_delay = (uchar)(3u << (USBASP_ISP_SCK_32 - option));
    }
    effective_sck = option;
}

void isp_apply_host_sck(void)
{
    if (board_sck_jumper_slow())
        ispSetSCKOption(USBASP_ISP_SCK_8);
    else
        ispSetSCKOption(requested_sck);
}

int isp_sck_is_8khz(void)
{
    return (isp_bus.transfer == ispTransmit_sw)
        && (sck_sw_delay == (uchar)(3u << (USBASP_ISP_SCK_32 - USBASP_ISP_SCK_8)));
}
