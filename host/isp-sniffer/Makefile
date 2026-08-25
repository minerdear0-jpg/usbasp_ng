MCU ?= atmega8
F_CPU ?= 12000000
CC = avr-gcc
OBJCOPY = avr-objcopy

CFLAGS = -mmcu=$(MCU) -DF_CPU=$(F_CPU)UL -Os -Wall -std=gnu99

.PHONY: all clean

all: capture_sniffer.hex

capture_sniffer.elf: capture_sniffer.c
	$(CC) $(CFLAGS) -o $@ $<

capture_sniffer.hex: capture_sniffer.elf
	$(OBJCOPY) -O ihex -R .eeprom $< $@

clean:
	rm -f capture_sniffer.elf capture_sniffer.hex
