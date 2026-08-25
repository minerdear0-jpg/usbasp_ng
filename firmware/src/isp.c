#include <avr/io.h>
#include "usbasp_config.h"
#include "usbasp/isp.h"
#include "usbasp/clock.h"
#include "usbasp/protocol.h"

uchar sck_sw_delay;
uchar isp_hiaddr;
uchar (*ispTransmit)(uchar);

static inline void spiHWenable(void)
{
    SPCR |= (1 << SPE) | (1 << MSTR);
}

static inline void spiHWdisable(void)
{
    SPCR = 0;
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
        ispTransmit = ispTransmit_hw;
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
        ispTransmit = ispTransmit_sw;
        sck_sw_delay = (uchar)(3u << (USBASP_ISP_SCK_32 - option));
    }
}

static void ispDelay(void)
{
    uint8_t starttime = TIMERVALUE;
    while ((uint8_t)(TIMERVALUE - starttime) < sck_sw_delay)
        ;
}

void ispConnect(void)
{
    ISP_DDR |= (1 << ISP_SCK);
    ISP_DDR |= (1 << ISP_MOSI);
    ISP_DDR |= (1 << ISP_RST);
    ISP_OUT |= (1 << ISP_MISO);
    isp_hiaddr = 0xff;
}

void ispDisconnect(void)
{
    ISP_DDR &= ~((1 << ISP_RST) | (1 << ISP_SCK) | (1 << ISP_MOSI));
    ISP_OUT &= ~(1 << ISP_MISO);
    ISP_OUT &= ~(1 << ISP_MOSI);
    spiHWdisable();
    /* Keep requested SCK: avrdude may reconnect in the same session. */
}

uchar ispTransmit_sw(uchar send_byte)
{
    uchar rec_byte = 0;
    uchar i;
    for (i = 0; i < 8; i++) {
        if ((send_byte & 0x80) != 0)
            ISP_OUT |= (1 << ISP_MOSI);
        else
            ISP_OUT &= ~(1 << ISP_MOSI);
        send_byte = (uchar)(send_byte << 1);
        rec_byte = (uchar)(rec_byte << 1);
        if ((ISP_IN & (1 << ISP_MISO)) != 0)
            rec_byte++;
        ISP_OUT |= (1 << ISP_SCK);
        ispDelay();
        ISP_OUT &= ~(1 << ISP_SCK);
        ispDelay();
    }
    return rec_byte;
}

uchar ispTransmit_hw(uchar send_byte)
{
    SPDR = send_byte;
    while (!(SPSR & (1 << SPIF)))
        ;
    return SPDR;
}

uchar ispEnterProgrammingMode(void)
{
    uchar check;
    extern uchar prog_sck;

    if (prog_sck == USBASP_ISP_SCK_AUTO)
        prog_sck = USBASP_ISP_SCK_1500;

    while (prog_sck >= USBASP_ISP_SCK_0_5) {
        uchar (*spiTx)(uchar) = ispTransmit;

        if (ispTransmit == ispTransmit_hw)
            spiHWenable();

        uchar tries = 3;
        do {
            ISP_OUT |= (1 << ISP_RST);
            clockWait(1);
            ISP_OUT &= ~(1 << ISP_RST);
            clockWait(62); /* ~20 ms */

            spiTx(0xAC);
            spiTx(0x53);
            check = spiTx(0);
            spiTx(0);

            if (check == 0x53)
                return 0;

            /* AT89S51/52 programming-enable echo */
            ISP_OUT |= (1 << ISP_RST);
            clockWait(5);
            spiTx(0xAC);
            spiTx(0x53);
            spiTx(0);
            check = spiTx(0);
            if (check == 0x69)
                return 0;
        } while (--tries);

        spiHWdisable();
        ispSetSCKOption(--prog_sck);
    }

    return 1;
}

static void ispUpdateExtended(unsigned long address)
{
    uchar curr_hiaddr = (uchar)(address >> 17);
    if (isp_hiaddr != curr_hiaddr) {
        isp_hiaddr = curr_hiaddr;
        ispTransmit(0x4D);
        ispTransmit(0x00);
        ispTransmit(isp_hiaddr);
        ispTransmit(0x00);
    }
}

uchar ispReadFlash(unsigned long address)
{
    ispUpdateExtended(address);
    ispTransmit(0x20 | ((address & 1) << 3));
    ispTransmit((uchar)(address >> 9));
    ispTransmit((uchar)(address >> 1));
    return ispTransmit(0);
}

uchar ispWriteFlash(unsigned long address, uchar data, uchar pollmode)
{
    ispUpdateExtended(address);
    ispTransmit(0x40 | ((address & 1) << 3));
    ispTransmit((uchar)(address >> 9));
    ispTransmit((uchar)(address >> 1));
    ispTransmit(data);

    if (pollmode == 0)
        return 0;

    if (data == 0x7F) {
        clockWait(15);
        return 0;
    }

    uchar retries = 30;
    uint8_t starttime = TIMERVALUE;
    while (retries != 0) {
        if (ispReadFlash(address) != 0x7F)
            return 0;
        if ((uint8_t)(TIMERVALUE - starttime) > CLOCK_T_320us) {
            starttime = TIMERVALUE;
            retries--;
        }
    }
    return 1;
}

uchar ispFlushPage(unsigned long address, uchar pollvalue)
{
    ispUpdateExtended(address);
    ispTransmit(0x4C);
    ispTransmit((uchar)(address >> 9));
    ispTransmit((uchar)(address >> 1));
    ispTransmit(0);

    if (pollvalue == 0xFF) {
        clockWait(15);
        return 0;
    }

    uchar retries = 30;
    uint8_t starttime = TIMERVALUE;
    while (retries != 0) {
        if (ispReadFlash(address) != 0xFF)
            return 0;
        if ((uint8_t)(TIMERVALUE - starttime) > CLOCK_T_320us) {
            starttime = TIMERVALUE;
            retries--;
        }
    }
    return 1;
}

uchar ispReadEEPROM(unsigned int address)
{
    ispTransmit(0xA0);
    ispTransmit((uchar)(address >> 8));
    ispTransmit((uchar)address);
    return ispTransmit(0);
}

uchar ispWriteEEPROM(unsigned int address, uchar data)
{
    ispTransmit(0xC0);
    ispTransmit((uchar)(address >> 8));
    ispTransmit((uchar)address);
    ispTransmit(data);
    clockWait(30);
    return 0;
}
