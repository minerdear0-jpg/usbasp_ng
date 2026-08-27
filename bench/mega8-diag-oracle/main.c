/* USBasp NG — Channel 2 on Канарейка (ATmega8 on Nano PCB).
 *
 * Runs AFTER the programmer releases RESET. Does not see ISP traffic
 * (that happens under reset). Reports what actually landed in flash /
 * EEPROM / MCUCSR, and the ISP pin sample taken before we touch DDR.
 *
 * Cage: YEL0 ISP + CH340 UART. Clock 16 MHz crystal. UART0 115200 8N1.
 */
#include <avr/io.h>
#include <avr/interrupt.h>
#include <avr/pgmspace.h>
#include <avr/wdt.h>
#include <avr/eeprom.h>
#include <stdint.h>
#include <string.h>

#ifndef F_CPU
#define F_CPU 16000000UL
#endif

#define BAUD 115200UL
#define UBRR_VAL ((F_CPU / 8 / BAUD) - 1)

#define FLASH_SIZE 8192U
#define CANARY_LEN 512U
#define CANARY_ADDR 0x1E00U /* last 512 B — dies first if page flush truncates */
#define PAGE 64U
#define SRAM_SCRATCH 64U

#define EE_MAGIC0 0x00
#define EE_MAGIC1 0x01
#define EE_VER 0x02
#define EE_LAST_CSR 0x03
#define EE_BOOT_LO 0x04
#define EE_BOOT_HI 0x05
#define EE_FAIL 0x06
#define EE_WDT_ARMED 0x07
#define EE_FAULT 0x08
#define EE_FAULT_ARG 0x09
#define EE_MAGIC_A 0xA5
#define EE_MAGIC_B 0x5A
#define EE_TEST_BASE 0x20
#define EE_TEST_LEN 16U

#define FLT_OFF 0
#define FLT_CANARY 1
#define FLT_CRC 2
#define FLT_PINS 3
#define FLT_RST_WDT 4
#define FLT_RST_POR 5
#define EE_MAGIC_B 0x5A
#define EE_TEST_BASE 0x20
#define EE_TEST_LEN 16U

#define BUILD_ID "20260826"
#define PS(s) uart_puts_P(PSTR(s))
#define LB(e) line_begin_P(PSTR(e))

/* Flash-only: linker --section-start=.canary=0x1E00. Not PROGMEM (that
 * parks the table after the vectors — useless against a short last page). */
const __flash uint8_t canary[CANARY_LEN]
    __attribute__((used, aligned(64), section(".canary"))) = {
#include "canary_bytes.inc"
};

static const char ident[] PROGMEM = "CANARY-M8";

volatile uint32_t g_ms;

static uint8_t boot_pinb;
static uint8_t boot_ddrb;
static uint8_t boot_portb;
static uint8_t boot_csr;
static uint8_t eeprom_erased;
static uint16_t boot_count;
static uint8_t fail_sticky;
static uint8_t fault_kind;
static uint8_t fault_arg;
static uint8_t line_buf[40];
static uint8_t line_len;

ISR(TIMER1_COMPA_vect)
{
    g_ms++;
}

static uint32_t millis(void)
{
    uint32_t v;
    uint8_t s = SREG;
    cli();
    v = g_ms;
    SREG = s;
    return v;
}

static uint8_t canary_expect(uint16_t off)
{
    uint8_t page = (uint8_t)(off / PAGE);
    uint8_t i = (uint8_t)(off % PAGE);

    switch (page) {
    case 0:
        return i;
    case 1:
        return (uint8_t)(0xFF - i);
    case 2:
        return (uint8_t)((i & 1) ? 0x55 : 0xAA);
    case 3:
        return 0x00;
    case 4:
        return 0xFF;
    case 5:
        return (uint8_t)(i * 7u + 0x2Au);
    case 6:
        return 0xA5;
    default:
        return 0x5A;
    }
}

static uint16_t crc16_update(uint16_t crc, uint8_t b)
{
    uint8_t i;

    crc ^= (uint16_t)b << 8;
    for (i = 0; i < 8; i++) {
        if (crc & 0x8000)
            crc = (uint16_t)((crc << 1) ^ 0x1021);
        else
            crc = (uint16_t)(crc << 1);
    }
    return crc;
}

static uint16_t flash_crc16(uint16_t start, uint16_t len)
{
    uint16_t crc = 0xFFFF;
    uint16_t i;

    for (i = 0; i < len; i++)
        crc = crc16_update(crc, pgm_read_byte((const uint8_t *)(start + i)));
    return crc;
}

static void uart_init(void)
{
    UCSRA = (1 << U2X);
    UBRRH = (uint8_t)(UBRR_VAL >> 8);
    UBRRL = (uint8_t)UBRR_VAL;
    UCSRB = (1 << TXEN) | (1 << RXEN);
    UCSRC = (1 << URSEL) | (1 << UCSZ1) | (1 << UCSZ0);
}

static void uart_putc(char c)
{
    while (!(UCSRA & (1 << UDRE)))
        ;
    UDR = c;
}

static void uart_puts(const char *s)
{
    while (*s)
        uart_putc(*s++);
}

static void uart_puts_P(const char *s)
{
    char c;
    while ((c = (char)pgm_read_byte(s++)) != 0)
        uart_putc(c);
}

static void uart_put_dec8(uint32_t v)
{
    char buf[8];
    uint8_t i;

    v %= 100000000UL;
    for (i = 8; i-- > 0;) {
        buf[i] = (char)('0' + (v % 10u));
        v /= 10;
    }
    for (i = 0; i < 8; i++)
        uart_putc(buf[i]);
}

static void uart_put_u8(uint8_t v)
{
    uint8_t z = 0;
    uint8_t d;

    d = v / 100;
    if (d) {
        uart_putc((char)('0' + d));
        z = 1;
        v = (uint8_t)(v % 100);
    }
    d = v / 10;
    if (d || z) {
        uart_putc((char)('0' + d));
        v = (uint8_t)(v % 10);
    }
    uart_putc((char)('0' + v));
}

static void uart_put_u16(uint16_t v)
{
    char buf[5];
    uint8_t n = 0;

    if (v == 0) {
        uart_putc('0');
        return;
    }
    while (v) {
        buf[n++] = (char)('0' + (v % 10u));
        v /= 10;
    }
    while (n--)
        uart_putc(buf[n]);
}

static void uart_put_hex8(uint8_t v)
{
    static const char H[] PROGMEM = "0123456789ABCDEF";
    uart_putc((char)pgm_read_byte(&H[v >> 4]));
    uart_putc((char)pgm_read_byte(&H[v & 0x0F]));
}

static void uart_put_hex16(uint16_t v)
{
    uart_put_hex8((uint8_t)(v >> 8));
    uart_put_hex8((uint8_t)v);
}

static void line_begin_P(const char *event)
{
    uart_putc('@');
    uart_put_dec8(millis());
    uart_putc(' ');
    uart_puts_P(event);
}

static void uart_put_passfail(uint8_t fail)
{
    if (fail)
        PS("FAIL");
    else
        PS("PASS");
}

static void uart_put_ee_state(void)
{
    if (eeprom_erased)
        PS("chip_erased");
    else
        PS("live");
}

static void line_end(void)
{
    PS("\r\n");
}

static void emit_inject(uint8_t on)
{
    if (on)
        PS(",inject=1");
}

static void emit_fault(void)
{
    LB("FAULT");
    PS(",kind=");
    switch (fault_kind) {
    case FLT_CANARY:
        PS("canary");
        break;
    case FLT_CRC:
        PS("crc");
        break;
    case FLT_PINS:
        PS("pins");
        break;
    case FLT_RST_WDT:
        PS("reset-wdt");
        break;
    case FLT_RST_POR:
        PS("reset-por");
        break;
    default:
        PS("off");
        break;
    }
    PS(",arg=");
    uart_put_u8(fault_arg);
    line_end();
}

static void fault_set(uint8_t kind, uint8_t arg)
{
    fault_kind = kind;
    fault_arg = arg;
    eeprom_update_byte((uint8_t *)EE_FAULT, kind);
    eeprom_update_byte((uint8_t *)EE_FAULT_ARG, arg);
    emit_fault();
}

static void timer1_init(void)
{
    /* 16 MHz / 64 = 250 kHz, CTC OCR1A=249 → 1 ms */
    TCCR1A = 0;
    TCCR1B = (1 << WGM12) | (1 << CS11) | (1 << CS10);
    OCR1A = 249;
    TIMSK |= (1 << OCIE1A);
}

static void leds_init(void)
{
    DDRB |= (1 << PB5);
    DDRD |= (1 << PD2) | (1 << PD3) | (1 << PD4);
    PORTB &= ~(1 << PB5);
    PORTD &= ~((1 << PD2) | (1 << PD3) | (1 << PD4));
}

static void leds_set(uint8_t mask)
{
    if (mask & 1)
        PORTB |= (1 << PB5);
    else
        PORTB &= ~(1 << PB5);
    if (mask & 2)
        PORTD |= (1 << PD2);
    else
        PORTD &= ~(1 << PD2);
    if (mask & 4)
        PORTD |= (1 << PD3);
    else
        PORTD &= ~(1 << PD3);
    if (mask & 8)
        PORTD |= (1 << PD4);
    else
        PORTD &= ~(1 << PD4);
}

static uint16_t sram_free(void)
{
    extern uint8_t _end;
    return (uint16_t)(SP - (uint16_t)&_end);
}

static void emit_ready(void)
{
    LB("READY");
    PS(",who=canary,mcu=m8,f_cpu=");
#if F_CPU == 16000000UL
    PS("16000000");
#else
#error "READY f_cpu string: add this F_CPU or keep 16 MHz Nano"
#endif
    PS(",sig_expect=1E9307,build=");
    PS(BUILD_ID);
    PS(",ident=");
    uart_puts_P(ident);
    PS(",canary_off=");
    uart_put_hex16((uint16_t)(uintptr_t)&canary[0]);
    PS(",canary_len=");
    uart_put_hex16(CANARY_LEN);
    PS(",sram_free=");
    uart_put_u16(sram_free());
    PS(",tcnt1=");
    uart_put_hex16(TCNT1);
    line_end();
}

static void emit_reset_cause(void)
{
    LB("RESET_CAUSE");
    PS(",csr=");
    uart_put_hex8(boot_csr);
    PS(",porf=");
    if (fault_kind == FLT_RST_POR)
        uart_putc('1');
    else if (fault_kind == FLT_RST_WDT)
        uart_putc('0');
    else
        uart_putc((boot_csr & (1 << PORF)) ? '1' : '0');
    PS(",extrf=");
    if (fault_kind == FLT_RST_WDT || fault_kind == FLT_RST_POR)
        uart_putc('0');
    else
        uart_putc((boot_csr & (1 << EXTRF)) ? '1' : '0');
    PS(",borf=");
    uart_putc((boot_csr & (1 << BORF)) ? '1' : '0');
    PS(",wdrf=");
    if (fault_kind == FLT_RST_WDT)
        uart_putc('1');
    else if (fault_kind == FLT_RST_POR)
        uart_putc('0');
    else
        uart_putc((boot_csr & (1 << WDRF)) ? '1' : '0');
    PS(",eeprom=");
    uart_put_ee_state();
    PS(",boot=");
    uart_put_u16(boot_count);
    emit_inject(fault_kind == FLT_RST_WDT || fault_kind == FLT_RST_POR);
    line_end();
}

static void emit_isp_pins(uint8_t pinb, uint8_t ddrb, uint8_t portb, const char *tag)
{
    if (fault_kind == FLT_PINS)
        pinb |= (uint8_t)((1 << PB3) | (1 << PB5));
    LB("ISP_PINS");
    PS(",when=");
    uart_puts_P(tag);
    PS(",pinb=");
    uart_put_hex8(pinb);
    PS(",ddrb=");
    uart_put_hex8(ddrb);
    PS(",portb=");
    uart_put_hex8(portb);
    PS(",mosi=");
    uart_putc((pinb & (1 << PB3)) ? '1' : '0');
    PS(",miso=");
    uart_putc((pinb & (1 << PB4)) ? '1' : '0');
    PS(",sck=");
    uart_putc((pinb & (1 << PB5)) ? '1' : '0');
    emit_inject(fault_kind == FLT_PINS);
    line_end();
}

static uint8_t sram_test(void)
{
    uint8_t buf[SRAM_SCRATCH];
    uint8_t i, b, expect;
    uint8_t fail = 0;

    for (b = 1; b; b = (uint8_t)(b << 1)) {
        for (i = 0; i < SRAM_SCRATCH; i++)
            buf[i] = b;
        for (i = 0; i < SRAM_SCRATCH; i++) {
            if (buf[i] != b)
                fail = 1;
        }
    }
    for (b = 1; b; b = (uint8_t)(b << 1)) {
        expect = (uint8_t)~b;
        for (i = 0; i < SRAM_SCRATCH; i++)
            buf[i] = expect;
        for (i = 0; i < SRAM_SCRATCH; i++) {
            if (buf[i] != expect)
                fail = 1;
        }
    }
    for (i = 0; i < SRAM_SCRATCH; i++)
        buf[i] = (uint8_t)((i & 1) ? 0x55 : 0xAA);
    for (i = 0; i < SRAM_SCRATCH; i++) {
        expect = (uint8_t)((i & 1) ? 0x55 : 0xAA);
        if (buf[i] != expect)
            fail = 1;
    }
    LB("SRAM_TEST");
    PS(",len=");
    uart_put_u8(SRAM_SCRATCH);
    PS(",result=");
    uart_put_passfail(fail);
    line_end();
    return fail;
}

static uint8_t eeprom_test(void)
{
    uint8_t saved[EE_TEST_LEN];
    uint8_t i, fail = 0;
    uint8_t got;

    for (i = 0; i < EE_TEST_LEN; i++)
        saved[i] = eeprom_read_byte((const uint8_t *)(EE_TEST_BASE + i));
    for (i = 0; i < EE_TEST_LEN; i++)
        eeprom_update_byte((uint8_t *)(EE_TEST_BASE + i), i);
    for (i = 0; i < EE_TEST_LEN; i++) {
        got = eeprom_read_byte((const uint8_t *)(EE_TEST_BASE + i));
        if (got != i)
            fail = 1;
    }
    for (i = 0; i < EE_TEST_LEN; i++)
        eeprom_update_byte((uint8_t *)(EE_TEST_BASE + i), saved[i]);
    LB("EEPROM_TEST");
    PS(",addr=");
    uart_put_hex16(EE_TEST_BASE);
    PS(",len=");
    uart_put_hex16(EE_TEST_LEN);
    PS(",result=");
    uart_put_passfail(fail);
    line_end();
    return fail;
}

static uint8_t canary_test(void)
{
    uint16_t i;
    uint16_t fail_at = 0xFFFF;
    uint8_t page, pfail;
    uint8_t any = 0;
    uint16_t base = (uint16_t)(uintptr_t)&canary[0];

    if (base != CANARY_ADDR) {
        LB("CANARY");
        PS(",link=BAD,off=");
        uart_put_hex16(base);
        PS(",want=");
        uart_put_hex16(CANARY_ADDR);
        PS(",result=FAIL");
        line_end();
        return 1;
    }

    for (page = 0; page < (CANARY_LEN / PAGE); page++) {
        pfail = 0;
        for (i = 0; i < PAGE; i++) {
            uint16_t off = (uint16_t)page * PAGE + i;
            uint8_t got = canary[off];
            if (got != canary_expect(off)) {
                pfail = 1;
                if (fail_at == 0xFFFF)
                    fail_at = off;
            }
        }
        if (fault_kind == FLT_CANARY && page == fault_arg)
            pfail = 1;
        LB("CANARY");
        PS(",page=");
        uart_put_hex16((uint16_t)page * PAGE + base);
        PS(",result=");
        uart_put_passfail(pfail);
        emit_inject(fault_kind == FLT_CANARY && page == fault_arg);
        line_end();
        if (pfail)
            any = 1;
    }
    LB("CANARY_SUMMARY");
    PS(",off=");
    uart_put_hex16(base);
    PS(",fail_at=");
    uart_put_hex16(fail_at);
    PS(",result=");
    uart_put_passfail(any);
    line_end();
    return any;
}

static void emit_flash_crc(void)
{
    uint16_t crc = flash_crc16(0, FLASH_SIZE);

    if (fault_kind == FLT_CRC)
        crc ^= 0xFFFF;
    LB("FLASH_CRC");
    PS(",off=0000,len=");
    uart_put_hex16(FLASH_SIZE);
    PS(",crc=");
    uart_put_hex16(crc);
    emit_inject(fault_kind == FLT_CRC);
    line_end();
}

static void emit_counters(void)
{
    LB("COUNTERS");
    PS(",boot=");
    uart_put_u16(boot_count);
    PS(",fail=");
    uart_put_u8(fail_sticky);
    PS(",eeprom=");
    uart_put_ee_state();
    line_end();
}

static void emit_help(void)
{
    LB("HELP");
    PS(",cmds=help,info,selftest,flash-crc,canary,eeprom-test,sram-test,reset-cause,counters,isp-pins,arm,clear,wdt-test,fault,time");
    line_end();
}

static uint8_t run_selftest(void)
{
    uint8_t fail = 0;

    fail = (uint8_t)(fail | sram_test());
    fail = (uint8_t)(fail | eeprom_test());
    fail = (uint8_t)(fail | canary_test());
    emit_flash_crc();
    if (fault_kind == FLT_CRC)
        fail = 1;
    if (fail) {
        fail_sticky = 1;
        if (eeprom_read_byte((const uint8_t *)EE_FAIL) != 0xFF)
            eeprom_update_byte((uint8_t *)EE_FAIL, 1);
    }
    LB("SELFTEST");
    PS(",result=");
    uart_put_passfail(fail);
    line_end();
    return fail;
}

static void ee_boot(void)
{
    uint8_t a = eeprom_read_byte((const uint8_t *)EE_MAGIC0);
    uint8_t b = eeprom_read_byte((const uint8_t *)EE_MAGIC1);
    uint16_t n;

    if (a == 0xFF && b == 0xFF) {
        eeprom_erased = 1;
        eeprom_update_byte((uint8_t *)EE_MAGIC0, EE_MAGIC_A);
        eeprom_update_byte((uint8_t *)EE_MAGIC1, EE_MAGIC_B);
        eeprom_update_byte((uint8_t *)EE_VER, 1);
        eeprom_update_byte((uint8_t *)EE_LAST_CSR, boot_csr);
        eeprom_update_byte((uint8_t *)EE_BOOT_LO, 1);
        eeprom_update_byte((uint8_t *)EE_BOOT_HI, 0);
        eeprom_update_byte((uint8_t *)EE_FAIL, 0);
        eeprom_update_byte((uint8_t *)EE_FAULT, 0);
        eeprom_update_byte((uint8_t *)EE_FAULT_ARG, 0);
        fault_kind = 0;
        fault_arg = 0;
        boot_count = 1;
        return;
    }
    eeprom_erased = 0;
    n = eeprom_read_byte((const uint8_t *)EE_BOOT_LO);
    n |= (uint16_t)eeprom_read_byte((const uint8_t *)EE_BOOT_HI) << 8;
    if (n != 0xFFFF && n < 0xFFFE)
        n++;
    eeprom_update_byte((uint8_t *)EE_BOOT_LO, (uint8_t)n);
    eeprom_update_byte((uint8_t *)EE_BOOT_HI, (uint8_t)(n >> 8));
    eeprom_update_byte((uint8_t *)EE_LAST_CSR, boot_csr);
    boot_count = n;
    fail_sticky = eeprom_read_byte((const uint8_t *)EE_FAIL);
    fault_kind = eeprom_read_byte((const uint8_t *)EE_FAULT);
    fault_arg = eeprom_read_byte((const uint8_t *)EE_FAULT_ARG);
    if (fault_kind == 0xFF)
        fault_kind = 0;
    if (fault_arg == 0xFF)
        fault_arg = 0;
}

static void maybe_wdt_result(void)
{
    uint8_t armed = eeprom_read_byte((const uint8_t *)EE_WDT_ARMED);

    if (armed != 1)
        return;
    eeprom_update_byte((uint8_t *)EE_WDT_ARMED, 0);
    LB("WATCHDOG_TEST");
    PS(",result=");
    uart_put_passfail((boot_csr & (1 << WDRF)) == 0);
    line_end();
}

static void cmd_wdt_test(void)
{
    eeprom_update_byte((uint8_t *)EE_WDT_ARMED, 1);
    LB("WATCHDOG_TEST");
    PS(",armed=1");
    line_end();
    wdt_enable(WDTO_2S);
    for (;;)
        ;
}

static void cmd_clear(void)
{
    eeprom_update_byte((uint8_t *)EE_BOOT_LO, 0);
    eeprom_update_byte((uint8_t *)EE_BOOT_HI, 0);
    eeprom_update_byte((uint8_t *)EE_FAIL, 0);
    fail_sticky = 0;
    boot_count = 0;
    LB("CLEAR");
    PS(",result=PASS");
    line_end();
}

static void cmd_fault(char *s)
{
    while (*s == ' ')
        s++;
    if (*s == 0) {
        emit_fault();
        return;
    }
    if (!strcmp(s, "off") || !strcmp(s, "none")) {
        fault_set(FLT_OFF, 0);
        return;
    }
    if (!strncmp(s, "canary", 6)) {
        uint8_t p;

        s += 6;
        while (*s == ' ')
            s++;
        p = (uint8_t)(*s ? (uint8_t)(*s - '0') : 7);
        if (p > 7 || (*s && s[1] != 0)) {
            LB("ERROR");
            PS(",fault=canary");
            line_end();
            return;
        }
        fault_set(FLT_CANARY, p);
        return;
    }
    if (!strcmp(s, "crc")) {
        fault_set(FLT_CRC, 0);
        return;
    }
    if (!strcmp(s, "pins")) {
        fault_set(FLT_PINS, 0);
        return;
    }
    if (!strcmp(s, "reset wdt") || !strcmp(s, "reset-wdt")) {
        fault_set(FLT_RST_WDT, 0);
        return;
    }
    if (!strcmp(s, "reset por") || !strcmp(s, "reset-por")) {
        fault_set(FLT_RST_POR, 0);
        return;
    }
    LB("ERROR");
    PS(",unknown=");
    uart_puts(s);
    line_end();
}

static void handle_cmd(char *s)
{
    while (*s == ' ' || *s == '>')
        s++;
    if (s[0] == 0)
        return;
    if (!strcmp(s, "help"))
        emit_help();
    else if (!strcmp(s, "info") || !strcmp(s, "ready"))
        emit_ready();
    else if (!strcmp(s, "selftest"))
        run_selftest();
    else if (!strcmp(s, "flash-crc"))
        emit_flash_crc();
    else if (!strcmp(s, "canary"))
        canary_test();
    else if (!strcmp(s, "eeprom-test"))
        eeprom_test();
    else if (!strcmp(s, "sram-test"))
        sram_test();
    else if (!strcmp(s, "reset-cause"))
        emit_reset_cause();
    else if (!strcmp(s, "counters"))
        emit_counters();
    else if (!strcmp(s, "isp-pins"))
        emit_isp_pins(PINB, DDRB, PORTB, PSTR("now"));
    else if (!strcmp(s, "arm")) {
        LB("ARMED");
        PS(",ready_for_isp=1");
        line_end();
    } else if (!strncmp(s, "fault", 5) && (s[5] == 0 || s[5] == ' '))
        cmd_fault(s + 5);
    else if (!strcmp(s, "clear"))
        cmd_clear();
    else if (!strcmp(s, "wdt-test"))
        cmd_wdt_test();
    else if (!strcmp(s, "time")) {
        LB("TIME");
        PS(",tcnt1=");
        uart_put_hex16(TCNT1);
        line_end();
    } else {
        LB("ERROR");
        PS(",unknown=");
        uart_puts(s);
        line_end();
    }
}

static void poll_uart(void)
{
    char c;

    if (!(UCSRA & (1 << RXC)))
        return;
    c = (char)UDR;
    if (c == '\r')
        return;
    if (c == '\n') {
        line_buf[line_len] = 0;
        handle_cmd((char *)line_buf);
        line_len = 0;
        return;
    }
    if ((uint8_t)(line_len + 1) < (uint8_t)sizeof(line_buf))
        line_buf[line_len++] = (uint8_t)c;
}

int main(void)
{
    uint32_t last_led = 0;
    uint32_t last_hb = 0;
    uint8_t step = 0;
    uint8_t self_fail;

    boot_csr = MCUCSR;
    MCUCSR = 0;
    wdt_disable();

    boot_pinb = PINB;
    boot_ddrb = DDRB;
    boot_portb = PORTB;

    uart_init();
    timer1_init();
    leds_init();
    sei();

    ee_boot();

    PS("\r\n");
    emit_ready();
    emit_fault();
    emit_reset_cause();
    emit_isp_pins(boot_pinb, boot_ddrb, boot_portb, PSTR("reset"));
    maybe_wdt_result();
    LB("APP_START");
    PS(",build=");
    PS(BUILD_ID);
    line_end();
    self_fail = run_selftest();

    for (;;) {
        uint32_t now;

        poll_uart();
        now = millis();
        if ((uint32_t)(now - last_led) >= (self_fail ? 80u : 180u)) {
            last_led = now;
            leds_set(1u << (step & 3));
            step++;
        }
        if ((uint32_t)(now - last_hb) >= 3000u) {
            last_hb = now;
            LB("HEARTBEAT");
            PS(",boot=");
            uart_put_u16(boot_count);
            PS(",fail=");
            uart_put_u8(self_fail || fail_sticky);
            line_end();
        }
    }
}
