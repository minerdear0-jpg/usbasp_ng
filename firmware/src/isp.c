#include <avr/io.h>
#include <avr/interrupt.h>
#include "usbasp/isp.h"
#include "usbasp/sck.h"
#include "usbasp/clock.h"
#include "usbasp/protocol.h"
#include "usbasp/prog_state.h"
#include "usbasp/board.h"

uchar isp_hiaddr;
uchar (*ispTransmit)(uchar) = ispTransmit_sw;

void ispConnect(void)
{
    /* One pin per RMW: V-USB TX does in/ori/out on DDRB (PB0/PB1). */
    ISP_DDR |= (1 << ISP_SCK);
    ISP_DDR |= (1 << ISP_MOSI);
    ISP_DDR |= (1 << ISP_RST);
    ISP_OUT &= ~(1 << ISP_RST);
    ISP_OUT &= ~(1 << ISP_SCK);
    ISP_OUT |= (1 << ISP_MISO);
    /* 2011: RST high-low longer than two target SCK before ENABLEPROG. */
    clockWait(1);
    ISP_OUT |= (1 << ISP_RST);
    clockWait(1);
    ISP_OUT &= ~(1 << ISP_RST);
    /* 0xff: first flash access at 0 still writes Load Extended Address (dioannidis). */
    isp_hiaddr = 0xff;
}

void ispDisconnect(void)
{
    ISP_DDR &= ~(1 << ISP_RST);
    ISP_DDR &= ~(1 << ISP_SCK);
    ISP_DDR &= ~(1 << ISP_MOSI);
    ISP_OUT &= ~(1 << ISP_RST);
    ISP_OUT &= ~(1 << ISP_SCK);
    ISP_OUT &= ~(1 << ISP_MOSI);
    ISP_OUT &= ~(1 << ISP_MISO);
    isp_spi_hw_disable();
    if (board_sck_jumper_slow())
        ispSetSCKOption(USBASP_ISP_SCK_8);
    else
        ispSetSCKOption(prog_sck);
    /* Keep requested SCK: avrdude may reconnect in the same session. */
}

uchar ispTransmit_sw(uchar send_byte)
{
    uchar rec_byte = 0;
    uchar i;
    uchar sreg;

    /* No LED/USB here: bitbang is a timing path. cli is only PORTB RMW
     * vs INT0 (setup runs from usbPoll, I=1). */
    for (i = 0; i < 8; i++) {
        sreg = SREG;
        cli();
        if ((send_byte & 0x80) != 0)
            ISP_OUT |= (1 << ISP_MOSI);
        else
            ISP_OUT &= ~(1 << ISP_MOSI);
        send_byte = (uchar)(send_byte << 1);
        rec_byte = (uchar)(rec_byte << 1);
        if ((ISP_IN & (1 << ISP_MISO)) != 0)
            rec_byte++;
        ISP_OUT |= (1 << ISP_SCK);
        SREG = sreg;
        isp_sck_delay();
        sreg = SREG;
        cli();
        ISP_OUT &= ~(1 << ISP_SCK);
        SREG = sreg;
        isp_sck_delay();
    }
    return rec_byte;
}

uchar ispTransmit_hw(uchar send_byte)
{
    board_led_isp_activity();
    SPDR = send_byte;
    while (!(SPSR & (1 << SPIF)))
        ;
    return SPDR;
}

uchar ispEnterProgrammingMode(void)
{
    uchar check;
    uchar autoslow = 0;
    uchar jumper = (uchar)board_sck_jumper_slow();
    uchar sck = prog_sck;

    if (jumper) {
        sck = USBASP_ISP_SCK_8;
    } else if (sck == USBASP_ISP_SCK_AUTO) {
        autoslow = 1;
        sck = USBASP_ISP_SCK_1500;
        prog_sck = sck;
    }

    while (sck >= USBASP_ISP_SCK_0_5) {
        ispSetSCKOption(sck);
        uchar (*spiTx)(uchar) = ispTransmit;
        board_led_isp_activity();

        if (ispTransmit == ispTransmit_hw)
            isp_spi_hw_enable();

        uchar tries = 3;
        do {
            ISP_OUT |= (1 << ISP_RST);
            clockWait(1);
            ISP_OUT &= ~(1 << ISP_RST);
            clockWait(62); /* ~20 ms at 320 us ticks */

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

        isp_spi_hw_disable();
        if (jumper)
            break;
        if (sck <= USBASP_ISP_SCK_0_5)
            break;
        if (autoslow)
            sck = isp_sck_autoslow(sck);
        else
            sck--;
        if (sck < USBASP_ISP_SCK_0_5)
            break;
        prog_sck = sck;
        ispSetSCKOption(sck);
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
