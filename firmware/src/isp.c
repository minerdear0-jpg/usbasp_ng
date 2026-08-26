#include <avr/io.h>
#include <avr/interrupt.h>
#include "usbasp/isp.h"
#include "usbasp/sck.h"
#include "usbasp/clock.h"
#include "usbasp/protocol.h"
#include "usbasp/prog_state.h"
#include "usbasp/board.h"
#include "diag/diag.h"

uchar isp_hiaddr;

/* Default: software transport. isp_bus_select_* keeps the triple consistent. */
struct isp_transport isp_bus = {
    .transfer = ispTransmit_sw,
    .enable = isp_spi_hw_disable,
    .disable = isp_spi_hw_disable,
};

void isp_bus_select_hw(void)
{
    isp_bus.transfer = ispTransmit_hw;
    isp_bus.enable = isp_spi_hw_enable;
    isp_bus.disable = isp_spi_hw_disable;
}

void isp_bus_select_sw(void)
{
    /* enable == disable: ensure SPE off when entering SW bitbang. */
    isp_bus.transfer = ispTransmit_sw;
    isp_bus.enable = isp_spi_hw_disable;
    isp_bus.disable = isp_spi_hw_disable;
}

void isp_out_set_bit(uchar bit)
{
    uchar sreg = SREG;

    cli();
    ISP_OUT |= (1 << bit);
    SREG = sreg;
}

void isp_out_clr_bit(uchar bit)
{
    uchar sreg = SREG;

    cli();
    ISP_OUT &= ~(1 << bit);
    SREG = sreg;
}

void ispConnect(void)
{
    /* One pin per RMW: V-USB TX does in/ori/out on DDRB (PB0/PB1). */
    ISP_DDR |= (1 << ISP_SCK);
    ISP_DDR |= (1 << ISP_MOSI);
    ISP_DDR |= (1 << ISP_RST);
    isp_out_clr_bit(ISP_RST);
    ISP_OUT &= ~(1 << ISP_SCK);
    ISP_OUT |= (1 << ISP_MISO);
    /* 2011: RST high-low longer than two target SCK before ENABLEPROG. */
    clockWait(1);
    isp_out_set_bit(ISP_RST);
    clockWait(1);
    isp_out_clr_bit(ISP_RST);
    /* 0xff: first flash access at 0 still writes Load Extended Address (dioannidis). */
    isp_hiaddr = 0xff;
}

void ispDisconnect(void)
{
    ISP_DDR &= ~(1 << ISP_RST);
    ISP_DDR &= ~(1 << ISP_SCK);
    ISP_DDR &= ~(1 << ISP_MOSI);
    isp_out_clr_bit(ISP_RST);
    ISP_OUT &= ~(1 << ISP_SCK);
    ISP_OUT &= ~(1 << ISP_MOSI);
    ISP_OUT &= ~(1 << ISP_MISO);
    isp_spi_hw_disable();
    isp_apply_host_sck();
    /* Keep requested SCK: avrdude may reconnect in the same session. */
}

uchar ispTransmit_sw(uchar send_byte)
{
    uchar rec_byte = 0;
    uchar i;
    uchar sreg;

    /* No LED/USB here: bitbang is a timing path. cli is only PORTB RMW
     * vs INT0 (usbPoll / usbFunctionSetup, I=1). */
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
    uchar sck = requested_sck;
    uint8_t tx[4];
    uint8_t rx[4];

    tx[0] = 0xAC;
    tx[1] = 0x53;
    tx[2] = 0x00;
    tx[3] = 0x00;
    rx[0] = rx[1] = rx[2] = rx[3] = 0;

    if (jumper) {
        sck = USBASP_ISP_SCK_8;
    } else if (sck == USBASP_ISP_SCK_AUTO) {
        autoslow = 1;
        sck = USBASP_ISP_SCK_1500;
    }

    while (sck >= USBASP_ISP_SCK_0_5) {
        ispSetSCKOption(sck);
        uchar (*spiTx)(uchar) = isp_bus.transfer;
        board_led_isp_activity();
        isp_bus.enable();

        uchar tries = 3;
        do {
            isp_out_set_bit(ISP_RST);
            clockWait(1);
            isp_out_clr_bit(ISP_RST);
            clockWait(62); /* ~20 ms at 320 us ticks */

            /* Capture last AVR enableprog exchange; no emit on timing path. */
            rx[0] = spiTx(tx[0]);
            rx[1] = spiTx(tx[1]);
            check = spiTx(tx[2]);
            rx[2] = check;
            rx[3] = spiTx(tx[3]);

            if (check == 0x53) {
                diag_report_enableprog(tx, rx, 0);
                return 0;
            }

            /* AT89S51/52 programming-enable echo */
            isp_out_set_bit(ISP_RST);
            clockWait(5);
            rx[0] = spiTx(tx[0]);
            rx[1] = spiTx(tx[1]);
            rx[2] = spiTx(tx[2]);
            check = spiTx(tx[3]);
            rx[3] = check;
            if (check == 0x69) {
                diag_report_enableprog(tx, rx, 0);
                return 0;
            }
        } while (--tries);

        isp_bus.disable();
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
        ispSetSCKOption(sck);
    }

    diag_report_enableprog(tx, rx, 1);
    return 1;
}

static void ispUpdateExtended(uint32_t address)
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

uchar ispReadFlash(uint32_t address)
{
    ispUpdateExtended(address);
    ispTransmit(0x20 | ((address & 1) << 3));
    ispTransmit((uchar)(address >> 9));
    ispTransmit((uchar)(address >> 1));
    return ispTransmit(0);
}

uchar ispWriteFlash(uint32_t address, uchar data, uchar pollmode)
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

uchar ispFlushPage(uint32_t address, uchar pollvalue)
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

/* ISP EEPROM address is 16-bit on the wire (0xA0/0xC0 load addr H/L). */
uchar ispReadEEPROM(uint16_t address)
{
    ispTransmit(0xA0);
    ispTransmit((uchar)(address >> 8));
    ispTransmit((uchar)address);
    return ispTransmit(0);
}

uchar ispWriteEEPROM(uint16_t address, uchar data)
{
    ispTransmit(0xC0);
    ispTransmit((uchar)(address >> 8));
    ispTransmit((uchar)address);
    ispTransmit(data);
    /* Conservative target EEPROM write wait: 30 × CLOCK_T_320us ≈ 9.6 ms.
     * Independent of ISP SCK; do not shorten without silicon evidence. */
    clockWait(30);
    return 0;
}
