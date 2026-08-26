set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR avr)
set(CMAKE_C_COMPILER avr-gcc)
set(CMAKE_ASM_COMPILER avr-gcc)
set(CMAKE_OBJCOPY avr-objcopy)
set(CMAKE_OBJDUMP avr-objdump)
set(CMAKE_SIZE avr-size)
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)
set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)

# CachyOS/Arch: avr-libc under /usr/lib/avr; stock avr-gcc looks in /usr/avr.
# Prefer compile flags over include_directories(): the latter can miss some
# target compile rules after reconfigure (seen on HIDUART rebuilds).
# Per-MCU -B/-L for crt is applied in CMakeLists.txt after BOARD.
if(EXISTS "/usr/lib/avr/include" AND NOT EXISTS "/usr/avr/include")
    add_compile_options(-isystem /usr/lib/avr/include)
    set(USBASP_AVR_LIBC_PREFIX "/usr/lib/avr" CACHE INTERNAL "")
endif()
