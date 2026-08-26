# Closed-loop target: ATmega8 on Nano PCB → flash via USBasp2, watch LEDs + ttyUSB0
MCU      ?= atmega8
F_CPU    ?= 16000000
PROG     ?= usbasp
PORT_B   ?= 8
TARGET   := mega8-nano-loop

CC       := avr-gcc
OBJCOPY  := avr-objcopy
SIZE     := avr-size

# CachyOS/Arch: avr-libc under /usr/lib/avr
AVR_INC  := /usr/lib/avr/include
AVR_LIB  := /usr/lib/avr/lib/avr4
CFLAGS   := -mmcu=$(MCU) -DF_CPU=$(F_CPU)UL -Os -Wall -Wextra -std=gnu99
CFLAGS   += -isystem $(AVR_INC)
LDFLAGS  := -mmcu=$(MCU) -B$(AVR_LIB) -L$(AVR_LIB) -L/usr/lib/avr/lib

.PHONY: all clean flash size monitor diag-flash

all: $(TARGET).hex size

$(TARGET).elf: main.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

$(TARGET).hex: $(TARGET).elf
	$(OBJCOPY) -O ihex -R .eeprom $< $@

size: $(TARGET).elf
	$(SIZE) -B $<

flash: $(TARGET).hex
	avrdude -c $(PROG) -p $(MCU) -B $(PORT_B) -U flash:w:$(TARGET).hex:i

# Flash while diagplane monitors USBasp2 (YEL0)
diag-flash: $(TARGET).hex
	@echo "Start in another terminal: dist/diagplane.bin watch --serial YEL0"
	avrdude -c $(PROG) -p $(MCU) -B $(PORT_B) -U flash:w:$(TARGET).hex:i

monitor:
	@echo "screen /dev/ttyUSB0 115200  (quit: Ctrl-A then K)"
	screen /dev/ttyUSB0 115200

clean:
	rm -f $(TARGET).elf $(TARGET).hex $(TARGET).map
