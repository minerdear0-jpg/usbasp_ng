# Compatibility shim: CMake is canonical (see firmware/CMakeLists.txt).
# Convenience wrapper: one board profile per configure.
#   make BOARD=usbasp-atmega8-clone
#   make BOARD=usbasp-hiduart-atmega8
#   make flash   # ISP hex; HIDUART also writes .eep after (chip erase wipes EEPROM)

BOARD ?= usbasp-atmega8-clone
BUILDDIR := build/$(BOARD)
ISP ?= usbasp
SERIAL ?= 0000

MCU := $(shell sh scripts/cmake-set.sh USBASP_MCU boards/$(BOARD).cmake)
HFUSE := $(shell sh scripts/cmake-set.sh USBASP_HFUSE boards/$(BOARD).cmake)
LFUSE := $(shell sh scripts/cmake-set.sh USBASP_LFUSE boards/$(BOARD).cmake)

ifneq ($(findstring hiduart,$(BOARD)),)
HEX_NAME := usbasp-hiduart
else
HEX_NAME := usbasp
endif

BOARDS := usbasp-atmega8-clone usbasp-atmega8-usbisp usbasp-atmega88 \
	usbasp-hiduart-atmega8 usbasp-hiduart-atmega88

.PHONY: all configure hex clean test flash eeprom fuses probe all-boards

all: hex

configure:
	cmake -S . -B $(BUILDDIR) -G Ninja -DBOARD=$(BOARD) -DUSBASP_SERIAL=$(SERIAL)

hex: configure
	cmake --build $(BUILDDIR)

all-boards:
	@for b in $(BOARDS); do $(MAKE) BOARD=$$b hex; done

clean:
	rm -rf build

test:
	python3 tests/compatibility/avrdude/test_contract.py
	python3 tests/core/test_endian.py
	python3 tests/core/test_sck_autoslow.py
	python3 tests/core/test_writeflash_page.py
	python3 tests/core/test_classic_isolation.py
	python3 tests/core/test_serial_eep.py
	python3 tests/core/test_tpi.py
	python3 tests/core/test_hiduart_desc.py
	python3 tests/core/test_reference_not_built.py
	python3 tests/core/test_classic_msos20.py
	python3 tests/core/test_usb_descriptor_golden.py
	python3 tests/core/test_board_fuses.py
	python3 tests/core/test_requested_sck_writes.py
	python3 tests/core/test_ms_os_vendor_code.py
	python3 tests/core/test_has_tpi_gate.py
	python3 ../host/test_usbaspctl_profile.py

flash: hex
	avrdude -c $(ISP) -p $(MCU) -U flash:w:$(BUILDDIR)/$(HEX_NAME).hex:i
ifneq ($(findstring hiduart,$(BOARD)),)
	avrdude -c $(ISP) -p $(MCU) -U eeprom:w:$(BUILDDIR)/$(HEX_NAME).eep:i
endif

eeprom: hex
	@test "$(HEX_NAME)" = "usbasp-hiduart" || { echo "eeprom is HIDUART-only"; false; }
	avrdude -c $(ISP) -p $(MCU) -U eeprom:w:$(BUILDDIR)/$(HEX_NAME).eep:i

# Fuses: values also live in boards/<BOARD>/fuses.txt (must match cmake).
#   make fuses CONFIRM_FUSES=1
fuses:
ifndef CONFIRM_FUSES
	@echo "Refusing fuses: cheap clones often have hfuse=0xd9, not 2011 0xc9."
	@echo "Board fuse file: boards/$(BOARD)/fuses.txt (hfuse=$(HFUSE) lfuse=$(LFUSE))"
	@echo "Re-run: make fuses CONFIRM_FUSES=1 BOARD=$(BOARD)"
	@false
else
	avrdude -c $(ISP) -p $(MCU) -u -U hfuse:w:$(HFUSE):m -U lfuse:w:$(LFUSE):m
endif

# Programmer already running NG: talk to an ATmega8 on the ISP header.
probe:
	avrdude -c $(ISP) -p atmega8 -U signature:r:-:h
