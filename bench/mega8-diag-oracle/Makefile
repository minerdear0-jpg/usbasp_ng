# ATmega8 diag oracle — Channel 2 for USBasp2 / diagplane
MCU      ?= atmega8
F_CPU    ?= 16000000
PROG     ?= usbasp
PORT_B   ?= 8
TARGET   := mega8-diag-oracle
PYTHON   ?= python3

CC       := avr-gcc
OBJCOPY  := avr-objcopy
SIZE     := avr-size

AVR_INC  := /usr/lib/avr/include
AVR_LIB  := /usr/lib/avr/lib/avr4
CFLAGS   := -mmcu=$(MCU) -DF_CPU=$(F_CPU)UL -Os -Wall -Wextra -std=gnu99
CFLAGS   += -isystem $(AVR_INC)
LDFLAGS  := -mmcu=$(MCU) -B$(AVR_LIB) -L$(AVR_LIB) -L/usr/lib/avr/lib
LDFLAGS  += -Wl,--section-start=.canary=0x1E00

.PHONY: all clean flash size monitor diag-flash canary test crc

all: $(TARGET).hex size

canary_bytes.inc: gen_canary.py oracle_lib.py
	$(PYTHON) gen_canary.py

$(TARGET).elf: main.c canary_bytes.inc
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ main.c

$(TARGET).hex: $(TARGET).elf
	$(OBJCOPY) -O ihex -R .eeprom $< $@

size: $(TARGET).elf
	$(SIZE) -B $<

flash: $(TARGET).hex
	avrdude -c $(PROG) -p $(MCU) -B $(PORT_B) -U flash:w:$(TARGET).hex:i

diag-flash: $(TARGET).hex
	@echo "Other terminal: dist/diagplane.bin watch --serial YEL0"
	@echo "This UART:     $(PYTHON) harness.py monitor"
	avrdude -c $(PROG) -p $(MCU) -B $(PORT_B) -U flash:w:$(TARGET).hex:i

crc: $(TARGET).hex
	$(PYTHON) harness.py crc $(TARGET).hex

test:
	$(PYTHON) test_oracle.py

monitor:
	$(PYTHON) harness.py monitor

clean:
	rm -f $(TARGET).elf $(TARGET).hex $(TARGET).map canary_bytes.inc
