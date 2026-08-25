#include <avr/io.h>
#include "usbasp_config.h"
#include "usbdrv.h"
#include "usbasp/board.h"
#include "usbasp/clock.h"

#if USBASP_LED_STYLE == USBASP_LED_PORT
/* Fischl 2011: LEDs active-low on PORTC, DDRC already outputs.
 * PC1 = busy, PC0 = USB configured (not a second colour on many clones). */
#define ledRedOn()    PORTC &= ~(1 << PC1)
#define ledRedOff()   PORTC |= (1 << PC1)
#define ledGreenOn()  PORTC &= ~(1 << PC0)
#define ledGreenOff() PORTC |= (1 << PC0)
#elif USBASP_LED_STYLE == USBASP_LED_DDR
/* USBISP clones: drive by enabling the pin as output (open-drain style). */
#define ledRedOff()   DDRC &= ~(1 << PC1)
#define ledRedOn()    DDRC |= (1 << PC1)
#define ledGreenOff() DDRC &= ~(1 << PC0)
#define ledGreenOn()  DDRC |= (1 << PC0)
#else
#error "USBASP_LED_STYLE"
#endif

void board_init(void)
{
    /* Do not drive PORTD: USBISP clones share that port (nerdralph). */
    PORTB = 0;

#if USBASP_LED_STYLE == USBASP_LED_PORT
    DDRC = 0x03;
    PORTC = 0xfe;
#else
    DDRC = 0;
    PORTC = 0xfe;
#endif
}

void board_usb_reset_pulse(void)
{
    /* SE0 on USB D+/D- (PB0/PB1) for >10 ms */
    DDRB = ~0;
    clockWait(31); /* ~10 ms at 320 us ticks */
    DDRB = 0;
}

void board_led_red_on(void) { ledRedOn(); }
void board_led_red_off(void) { ledRedOff(); }
void board_led_green_on(void) { ledGreenOn(); }
void board_led_green_off(void) { ledGreenOff(); }

/* Timer0 /64 (clockInit). Count elapsed TCNT0 — not “MSB edges seen in main”.
 * During a dump usbFunctionRead blocks in SPI; main would starve a 1 Hz LED. */
#define T0_OVF_HALF ((F_CPU / 64 / 2) / 256) /* 0.5 s → 1 Hz */
#define T0_OVF_IDLE ((F_CPU / 64 / 10) / 256) /* ~100 ms after last kick */
#define T0_OVF_2HZ_HALF ((F_CPU / 64 / 4) / 256) /* 0.25 s → 2 Hz */

static uchar usb_live, t0_prev, rx_phase, tx_hold, t0_hi, isp_t0, isp_div, jp_phase;
static uint16_t t0_frac, half_ovf, idle_ovf, jp_half;

static void usb_xfer_kick(void)
{
    if (!usb_live) {
        usb_live = 1;
        rx_phase = 1;
        half_ovf = 0;
        t0_frac = 0;
        ledGreenOn();
    }
    idle_ovf = 0;
}

static void led_time_step(void)
{
    uchar now = TIMERVALUE;
    t0_frac += (uchar)(now - t0_prev);
    t0_prev = now;
    while (t0_frac >= 256) {
        t0_frac -= 256;
        jp_half++;
        if (jp_half >= T0_OVF_2HZ_HALF) {
            jp_half = 0;
            jp_phase ^= 1;
        }
        if (!usb_live)
            continue;
        half_ovf++;
        if (half_ovf >= T0_OVF_HALF) {
            half_ovf = 0;
            rx_phase ^= 1;
        }
        if (rx_phase)
            ledGreenOn();
        else
            ledGreenOff();
        idle_ovf++;
        if (idle_ovf >= T0_OVF_IDLE) {
            usb_live = 0;
            half_ovf = 0;
            rx_phase = 0;
        }
    }
}

void board_usb_bus_reset(unsigned char resetStarts)
{
    if (resetStarts) {
        usbConfiguration = 0;
        usb_live = tx_hold = 0;
        rx_phase = isp_div = jp_phase = 0;
        t0_frac = half_ovf = idle_ovf = jp_half = 0;
        ledGreenOff();
        ledRedOff();
    }
}

void board_usb_rx_activity(void)
{
    usb_xfer_kick();
    led_time_step();
}

void board_led_isp_activity(void)
{
    uchar hi;

    usb_xfer_kick();
    led_time_step();

    tx_hold = 24;
    hi = TIMERVALUE & 0x80;
    if (hi == isp_t0)
        return;
    isp_t0 = hi;
    isp_div++;
    if (isp_div & 0x20)
        ledRedOn();
    else
        ledRedOff();
}

void board_led_usb_update(void)
{
    uchar hi = TIMERVALUE & 0x80;

    led_time_step();

    if (hi && !t0_hi) {
        if (tx_hold && --tx_hold == 0) {
            isp_div = 0;
            ledRedOff();
        }
    }
    t0_hi = hi;

    if (!usb_live) {
        /* PC0 (USB/RX, left lamp on the clone): 2 Hz while JP3/PC2 is closed. */
        if (usbConfiguration && board_sck_jumper_slow()) {
            if (jp_phase)
                ledGreenOn();
            else
                ledGreenOff();
        } else if (usbConfiguration)
            ledGreenOn();
        else
            ledGreenOff();
    }
}

int board_sck_jumper_slow(void)
{
#if USBASP_HAS_SCK_JUMPER
    return (PINC & (1 << PC2)) == 0;
#else
    return 0;
#endif
}
