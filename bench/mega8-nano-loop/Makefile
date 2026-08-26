# Closed-loop target: ATmega8 on Nano PCB → flash via USBasp2, watch LEDs + ttyUSB0
MCU      ?= atmega8
F_CPU    ?= 16000000
PROG     ?= usbasp
PORT_B   ?= 8
TARGET   := mega8-nano-loop

# UART upload (Optiboot @ 115200 on CH340)
UART_PORT ?= /dev/ttyUSB0
UART_BAUD ?= 115200

# MiniCore-compatible fuses: 16 MHz crystal, BOD 2.7 V, 512 B boot, BOOTRST
LFUSE    ?= 0xBF
HFUSE    ?= 0xC4
UNLOCK   ?= 0x3F
LOCK     ?= 0x0F

OPTIBOOT := bootloader/optiboot_flash_atmega8_UART0_115200_16000000L_B5.hex

CC       := avr-gcc
OBJCOPY  := avr-objcopy
SIZE     := avr-size

# CachyOS/Arch: avr-libc under /usr/lib/avr
AVR_INC  := /usr/lib/avr/include
AVR_LIB  := /usr/lib/avr/lib/avr4
CFLAGS   := -mmcu=$(MCU) -DF_CPU=$(F_CPU)UL -Os -Wall -Wextra -std=gnu99
CFLAGS   += -isystem $(AVR_INC)
LDFLAGS  := -mmcu=$(MCU) -B$(AVR_LIB) -L$(AVR_LIB) -L/usr/lib/avr/lib

.PHONY: all clean flash flash-uart bootloader size monitor diag-flash

all: $(TARGET).hex size

$(TARGET).elf: main.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

$(TARGET).hex: $(TARGET).elf
	$(OBJCOPY) -O ihex -R .eeprom $< $@

size: $(TARGET).elf
	$(SIZE) -B $<

# ISP flash (chip erase). Wipes Optiboot — re-run `make bootloader` after if needed.
flash: $(TARGET).hex
	avrdude -c $(PROG) -p $(MCU) -B $(PORT_B) -U flash:w:$(TARGET).hex:i

# Flash while diagplane monitors USBasp2 (YEL0)
diag-flash: $(TARGET).hex
	@echo "Start in another terminal: dist/diagplane.bin watch --serial YEL0"
	avrdude -c $(PROG) -p $(MCU) -B $(PORT_B) -U flash:w:$(TARGET).hex:i

# One-time (or recovery): burn Optiboot + fuses via ISP. Keeps YEL0/USBasp for recovery.
bootloader:
	@test -f $(OPTIBOOT) || { echo "missing $(OPTIBOOT)"; exit 1; }
	avrdude -c $(PROG) -p $(MCU) -B $(PORT_B) \
		-U lock:w:$(UNLOCK):m \
		-U hfuse:w:$(HFUSE):m \
		-U lfuse:w:$(LFUSE):m \
		-U flash:w:$(OPTIBOOT):i \
		-U lock:w:$(LOCK):m

# Fast app update via CH340 + Optiboot (no ISP cable dance).
flash-uart: $(TARGET).hex
	avrdude -c arduino -p $(MCU) -P $(UART_PORT) -b $(UART_BAUD) \
		-U flash:w:$(TARGET).hex:i

monitor:
	@echo "screen $(UART_PORT) $(UART_BAUD)  (quit: Ctrl-A then K)"
	screen $(UART_PORT) $(UART_BAUD)

clean:
	rm -f $(TARGET).elf $(TARGET).hex $(TARGET).map
