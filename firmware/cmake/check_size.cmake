# Used as: cmake -DMCU=atmega8 -DELF=... -P check_size.cmake
if(NOT MCU OR NOT ELF)
    message(FATAL_ERROR "check_size.cmake needs -DMCU= and -DELF=")
endif()

# atmega88 contains the substring atmega8 — match exact names.
if(MCU STREQUAL "atmega8")
    set(FLASH_MAX 8192)
    set(RAM_MAX 1024)
elseif(MCU STREQUAL "atmega88" OR MCU STREQUAL "atmega88p")
    set(FLASH_MAX 8192)
    set(RAM_MAX 1024)
elseif(MCU STREQUAL "atmega328p" OR MCU STREQUAL "atmega328")
    set(FLASH_MAX 32768)
    set(RAM_MAX 2048)
else()
    message(FATAL_ERROR "No size budget for MCU=${MCU}")
endif()

execute_process(
    COMMAND avr-size --format=avr --mcu=${MCU} ${ELF}
    OUTPUT_VARIABLE SIZE_OUT
    RESULT_VARIABLE SIZE_RC)
if(NOT SIZE_RC EQUAL 0)
    message(FATAL_ERROR "avr-size failed on ${ELF}")
endif()

string(REGEX MATCH "Program:[ \t]+([0-9]+)" _prog "${SIZE_OUT}")
set(PROG_BYTES "${CMAKE_MATCH_1}")
string(REGEX MATCH "Data:[ \t]+([0-9]+)" _data "${SIZE_OUT}")
set(DATA_BYTES "${CMAKE_MATCH_1}")

if(PROG_BYTES STREQUAL "" OR DATA_BYTES STREQUAL "")
    message(FATAL_ERROR "Could not parse avr-size output:\n${SIZE_OUT}")
endif()

message(STATUS "Size ${MCU}: flash ${PROG_BYTES}/${FLASH_MAX}  ram ${DATA_BYTES}/${RAM_MAX}")

if(PROG_BYTES GREATER FLASH_MAX)
    message(FATAL_ERROR "Flash overflow: ${PROG_BYTES} > ${FLASH_MAX}")
endif()
if(DATA_BYTES GREATER RAM_MAX)
    message(FATAL_ERROR "RAM overflow: ${DATA_BYTES} > ${RAM_MAX}")
endif()
