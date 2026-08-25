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
target_include_directories(${APP_NAME} PRIVATE
    "${CMAKE_CURRENT_BINARY_DIR}/generated"
    "${CMAKE_CURRENT_SOURCE_DIR}/include"
    "${CMAKE_CURRENT_SOURCE_DIR}/third_party/vusb"
    "${CMAKE_CURRENT_SOURCE_DIR}/src_hid")

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

add_custom_command(TARGET ${APP_NAME} POST_BUILD
    COMMAND ${CMAKE_OBJCOPY} -j .text -j .data -O ihex
            $<TARGET_FILE:${APP_NAME}> ${APP_NAME}.hex
    COMMAND avr-size --format=avr --mcu=${MCU} $<TARGET_FILE:${APP_NAME}>
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
    COMMENT "objcopy ${APP_NAME}.hex")

set(USBASP_ISP "usbasp" CACHE STRING "avrdude programmer id used to ISP-flash this hex")

add_custom_target(flash
    COMMAND avrdude -c ${USBASP_ISP} -p ${MCU}
            -U flash:w:${CMAKE_CURRENT_BINARY_DIR}/${APP_NAME}.hex:i
    DEPENDS ${APP_NAME}
    COMMENT "ISP-flash ${APP_NAME}.hex onto ${MCU} (close J2 on the USBasp being programmed)")

add_custom_target(fuses
    COMMAND avrdude -c ${USBASP_ISP} -p ${MCU} -u
            -U hfuse:w:${USBASP_HFUSE}:m
            -U lfuse:w:${USBASP_LFUSE}:m
    COMMENT "Program fuses hfuse=${USBASP_HFUSE} lfuse=${USBASP_LFUSE}")

message(STATUS "Board=${BOARD} MCU=${MCU} F_CPU=${F_CPU} profile=${USBASP_PROFILE}")
message(STATUS "Flash: avrdude -c usbasp -p ${MCU} -U flash:w:${APP_NAME}.hex:i")
message(STATUS "Fuses: hfuse=${USBASP_HFUSE} lfuse=${USBASP_LFUSE}")
