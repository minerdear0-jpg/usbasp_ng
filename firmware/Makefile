# Convenience wrapper: one board profile per configure.
#   make BOARD=usbasp-atmega8-clone
#   make BOARD=usbasp-hiduart-atmega8
#   make flash   # ISP-write hex onto a USBasp with J2 closed (uses another usbasp)

BOARD ?= usbasp-atmega8-clone
BUILDDIR := build/$(BOARD)
ISP ?= usbasp

MCU := $(shell sed -n 's/^set(USBASP_MCU \([^)]*\))/\1/p' boards/$(BOARD).cmake)
HFUSE := $(shell sed -n 's/^set(USBASP_HFUSE \([^)]*\))/\1/p' boards/$(BOARD).cmake)
LFUSE := $(shell sed -n 's/^set(USBASP_LFUSE \([^)]*\))/\1/p' boards/$(BOARD).cmake)

ifneq ($(findstring hiduart,$(BOARD)),)
HEX_NAME := usbasp-hiduart
else
HEX_NAME := usbasp
endif

.PHONY: all configure hex clean test flash fuses probe

all: hex

configure:
	cmake -S . -B $(BUILDDIR) -G Ninja -DBOARD=$(BOARD)

hex: configure
	cmake --build $(BUILDDIR)

clean:
	rm -rf build

test:
	python3 tests/compatibility/avrdude/test_contract.py
	python3 tests/core/test_endian.py

flash: hex
	avrdude -c $(ISP) -p $(MCU) -U flash:w:$(BUILDDIR)/$(HEX_NAME).hex:i

fuses:
	avrdude -c $(ISP) -p $(MCU) -u -U hfuse:w:$(HFUSE):m -U lfuse:w:$(LFUSE):m

# Programmer already running NG: talk to an ATmega8 on the ISP header.
probe:
	avrdude -c $(ISP) -p atmega8 -U signature:r:-:h
