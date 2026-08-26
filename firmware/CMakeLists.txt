cmake_minimum_required(VERSION 3.16)

if(NOT CMAKE_TOOLCHAIN_FILE)
    set(CMAKE_TOOLCHAIN_FILE "${CMAKE_CURRENT_SOURCE_DIR}/cmake/avr-gcc.cmake" CACHE FILEPATH "" FORCE)
endif()

project(usbasp-ng C ASM)

set(BOARD "usbasp-atmega8-clone" CACHE STRING "Board profile in firmware/boards/")
set_property(CACHE BOARD PROPERTY STRINGS
    usbasp-atmega8-clone
    usbasp-atmega8-usbisp
    usbasp-atmega88
    usbasp-hiduart-atmega8
    usbasp-hiduart-atmega88)

include("${CMAKE_CURRENT_SOURCE_DIR}/boards/${BOARD}.cmake")

set(MCU ${USBASP_MCU})
set(F_CPU ${USBASP_F_CPU})

set(USBASP_SERIAL "0000" CACHE STRING "HIDUART USB iSerial, exactly 4 chars [A-Za-z0-9]")
string(LENGTH "${USBASP_SERIAL}" _usbasp_serial_len)
if(NOT _usbasp_serial_len EQUAL 4)
    message(FATAL_ERROR "USBASP_SERIAL must be exactly 4 characters (got '${USBASP_SERIAL}')")
endif()
if(NOT USBASP_SERIAL MATCHES "^[A-Za-z0-9][A-Za-z0-9][A-Za-z0-9][A-Za-z0-9]$")
    message(FATAL_ERROR "USBASP_SERIAL must be [A-Za-z0-9] x4 (got '${USBASP_SERIAL}')")
endif()
string(SUBSTRING "${USBASP_SERIAL}" 0 1 USBASP_SERIAL_CH0)
string(SUBSTRING "${USBASP_SERIAL}" 1 1 USBASP_SERIAL_CH1)
string(SUBSTRING "${USBASP_SERIAL}" 2 1 USBASP_SERIAL_CH2)
string(SUBSTRING "${USBASP_SERIAL}" 3 1 USBASP_SERIAL_CH3)

if(USBASP_HAS_HID_UART)
    set(USB_CFG_HAVE_INTRIN_ENDPOINT 1)
    set(USB_CFG_HAVE_INTRIN_ENDPOINT3 1)
    set(USB_CFG_EP3_NUMBER 2)
    set(USB_CFG_IMPLEMENT_HALT 1)
    set(USB_CFG_IMPLEMENT_FN_WRITEOUT 1)
    set(USB_CFG_HAVE_FLOWCONTROL 1)
    set(USB_CFG_LONG_TRANSFERS 1)
    set(USB_CFG_MAX_BUS_POWER 50)
    set(USB_CFG_DEVICE_CLASS 0)
    set(USB_CFG_INTERFACE_CLASS 0)
    set(USB_CFG_HID_REPORT_DESCRIPTOR_LENGTH 0x22)
    set(USB_CFG_DESCR_PROPS_DEVICE "USB_PROP_LENGTH(18)")
    set(USB_CFG_DESCR_PROPS_CONFIGURATION "USB_PROP_LENGTH(75)")
    set(USB_CFG_DESCR_PROPS_STRING_SERIAL_NUMBER "(USB_PROP_IS_EEPROM | USB_PROP_EEPROM_STRING_LENGTH(4))")
    set(USB_CFG_DESCR_PROPS_UNKNOWN "USB_PROP_IS_DYNAMIC")
    set(USB_CFG_SERIAL_BLOCK "")
    # 2.01 so Win11 UsbFlags differs from classic 2.00; VID/PID stay 16c0:05dc
    set(USB_CFG_DEVICE_VERSION "0x01, 0x02")
else()
    set(USB_CFG_HAVE_INTRIN_ENDPOINT 0)
    set(USB_CFG_HAVE_INTRIN_ENDPOINT3 0)
    set(USB_CFG_EP3_NUMBER 3)
    set(USB_CFG_IMPLEMENT_HALT 0)
    set(USB_CFG_IMPLEMENT_FN_WRITEOUT 0)
    set(USB_CFG_HAVE_FLOWCONTROL 0)
    set(USB_CFG_LONG_TRANSFERS 0)
    set(USB_CFG_MAX_BUS_POWER 50)
    set(USB_CFG_DEVICE_CLASS "0xff")
    set(USB_CFG_INTERFACE_CLASS 0)
    set(USB_CFG_HID_REPORT_DESCRIPTOR_LENGTH 0)
    set(USB_CFG_DESCR_PROPS_DEVICE 0)
    set(USB_CFG_DESCR_PROPS_CONFIGURATION 0)
    set(USB_CFG_DESCR_PROPS_STRING_SERIAL_NUMBER 0)
    set(USB_CFG_DESCR_PROPS_UNKNOWN 0)
    set(USB_CFG_SERIAL_BLOCK "")
    set(USB_CFG_DEVICE_VERSION "0x00, 0x02")
endif()

configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/usbconfig.h.in"
    "${CMAKE_CURRENT_BINARY_DIR}/generated/usbconfig.h"
    @ONLY)
configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/usbasp_config.h.in"
    "${CMAKE_CURRENT_BINARY_DIR}/generated/usbasp_config.h"
    @ONLY)

set(COMMON_SRC
    src/clock.c
    src/board.c
    src/sck.c
    src/isp.c
    src/vendor_isp.c
    src/tpi.S
    third_party/vusb/usbdrv.c
    third_party/vusb/usbdrvasm.S
    third_party/vusb/oddebug.c)

if(USBASP_HAS_HID_UART)
    set(APP_SRC src_hid/main.c src_hid/usb_setup.c src_hid/uart.c)
    set(APP_NAME usbasp-hiduart)
else()
    set(APP_SRC src/main.c src/usb_setup.c)
    set(APP_NAME usbasp)
endif()

add_executable(${APP_NAME} ${COMMON_SRC} ${APP_SRC})
set(USBASP_INCLUDES
    "${CMAKE_CURRENT_BINARY_DIR}/generated"
    "${CMAKE_CURRENT_SOURCE_DIR}/include"
    "${CMAKE_CURRENT_SOURCE_DIR}/third_party/vusb")
if(USBASP_HAS_HID_UART)
    list(APPEND USBASP_INCLUDES "${CMAKE_CURRENT_SOURCE_DIR}/src_hid")
endif()
target_include_directories(${APP_NAME} PRIVATE ${USBASP_INCLUDES})

target_compile_options(${APP_NAME} PRIVATE
    -mmcu=${MCU}
    -DF_CPU=${F_CPU}UL
    -Os
    -Wall
    -Wextra
    -fno-move-loop-invariants
    -fno-tree-scev-cprop
    -fno-inline-small-functions
    -ffunction-sections
    -fdata-sections)

target_compile_options(${APP_NAME} PRIVATE $<$<COMPILE_LANGUAGE:ASM>:-x assembler-with-cpp>)

target_link_options(${APP_NAME} PRIVATE
    -mmcu=${MCU}
    -Wl,--gc-sections
    -Wl,-Map,$<TARGET_FILE:${APP_NAME}>.map)

set_target_properties(${APP_NAME} PROPERTIES OUTPUT_NAME ${APP_NAME} SUFFIX ".elf")

set(USBASP_ISP "usbasp" CACHE STRING "avrdude programmer id used to ISP-flash this hex")

add_custom_command(TARGET ${APP_NAME} POST_BUILD
    COMMAND ${CMAKE_OBJCOPY} -j .text -j .data -O ihex
            $<TARGET_FILE:${APP_NAME}> ${APP_NAME}.hex
    COMMAND ${CMAKE_COMMAND}
            -DMCU=${MCU}
            -DELF=$<TARGET_FILE:${APP_NAME}>
            -P ${CMAKE_CURRENT_SOURCE_DIR}/cmake/check_size.cmake
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
    COMMENT "objcopy ${APP_NAME}.hex and enforce flash/RAM budget")

if(USBASP_HAS_HID_UART)
    add_custom_command(TARGET ${APP_NAME} POST_BUILD
        COMMAND ${CMAKE_OBJCOPY} -j .eeprom --change-section-lma .eeprom=0
                -O ihex $<TARGET_FILE:${APP_NAME}> ${APP_NAME}.eep
        WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
        COMMENT "objcopy ${APP_NAME}.eep")
    add_custom_target(eeprom
        COMMAND avrdude -c ${USBASP_ISP} -p ${MCU}
                -U eeprom:w:${CMAKE_CURRENT_BINARY_DIR}/${APP_NAME}.eep:i
        DEPENDS ${APP_NAME}
        COMMENT "ISP-write HIDUART serial EEPROM ${USBASP_SERIAL}")
endif()

option(USBASP_CONFIRM_FUSES
    "Allow the fuses target (clones often have hfuse 0xd9, not the 2011 0xc9)"
    OFF)

if(USBASP_HAS_HID_UART)
    add_custom_target(flash
        COMMAND avrdude -c ${USBASP_ISP} -p ${MCU}
                -U flash:w:${CMAKE_CURRENT_BINARY_DIR}/${APP_NAME}.hex:i
        COMMAND avrdude -c ${USBASP_ISP} -p ${MCU}
                -U eeprom:w:${CMAKE_CURRENT_BINARY_DIR}/${APP_NAME}.eep:i
        DEPENDS ${APP_NAME}
        COMMENT "ISP-flash ${APP_NAME}.hex then restore EEPROM serial ${USBASP_SERIAL} (chip erase wipes EESAVE-off clones)")
else()
    add_custom_target(flash
        COMMAND avrdude -c ${USBASP_ISP} -p ${MCU}
                -U flash:w:${CMAKE_CURRENT_BINARY_DIR}/${APP_NAME}.hex:i
        DEPENDS ${APP_NAME}
        COMMENT "ISP-flash ${APP_NAME}.hex onto ${MCU} (close J2 on the USBasp being programmed)")
endif()

if(USBASP_CONFIRM_FUSES)
    add_custom_target(fuses
        COMMAND avrdude -c ${USBASP_ISP} -p ${MCU} -u
                -U hfuse:w:${USBASP_HFUSE}:m
                -U lfuse:w:${USBASP_LFUSE}:m
        COMMENT "Program fuses hfuse=${USBASP_HFUSE} lfuse=${USBASP_LFUSE}")
else()
    add_custom_target(fuses
        COMMAND ${CMAKE_COMMAND} -E echo
            "Refusing fuses: clones may already be hfuse=0xd9 lfuse=0xef."
        COMMAND ${CMAKE_COMMAND} -E echo
            "Reconfigure with -DUSBASP_CONFIRM_FUSES=ON or: make fuses CONFIRM_FUSES=1"
        COMMAND ${CMAKE_COMMAND} -E false
        COMMENT "Fuses blocked until USBASP_CONFIRM_FUSES=ON")
endif()

message(STATUS "Board=${BOARD} MCU=${MCU} F_CPU=${F_CPU} profile=${USBASP_PROFILE}")
message(STATUS "Flash: avrdude -c usbasp -p ${MCU} -U flash:w:${APP_NAME}.hex:i")
message(STATUS "Fuses: hfuse=${USBASP_HFUSE} lfuse=${USBASP_LFUSE}")
if(USBASP_HAS_HID_UART)
    message(STATUS "HIDUART serial (EEPROM): ${USBASP_SERIAL}")
endif()
