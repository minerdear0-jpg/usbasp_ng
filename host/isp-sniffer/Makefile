# mega8 clone (12 MHz):  make
# Arduino Nano 328P:     make nano
MCU ?= atmega8
F_CPU ?= 12000000
HEX ?= capture_sniffer.hex
ELF ?= capture_sniffer.elf
CC = avr-gcc
OBJCOPY = avr-objcopy

CFLAGS = -mmcu=$(MCU) -DF_CPU=$(F_CPU)UL -Os -Wall -Wextra -std=gnu99

.PHONY: all nano clean

all: $(HEX)

nano:
	$(MAKE) MCU=atmega328p F_CPU=16000000 HEX=capture_sniffer-nano.hex \
		ELF=capture_sniffer-nano.elf all

$(ELF): capture_sniffer.c
	$(CC) $(CFLAGS) -o $@ $<

$(HEX): $(ELF)
	$(OBJCOPY) -O ihex -R .eeprom $< $@

clean:
	rm -f capture_sniffer.elf capture_sniffer.hex \
		capture_sniffer-nano.elf capture_sniffer-nano.hex
