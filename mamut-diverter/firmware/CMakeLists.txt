cmake_minimum_required(VERSION 3.16)
project(mamut_diverter_firmware C)

set(CMAKE_C_STANDARD 11)

option(MAMUT_PLATFORM "Target platform" "posix")

include_directories(include)

file(GLOB_RECURSE SOURCES src/*.c)

# Remove platform-specific HAL files, add correct one
list(FILTER SOURCES EXCLUDE REGEX "src/hal/.*\\.c$")
list(APPEND SOURCES src/hal/hal_${MAMUT_PLATFORM}.c)

# Remove diverter drivers not needed (all included by default)

add_library(mamut_firmware ${SOURCES})
target_include_directories(mamut_firmware PUBLIC include)

# Test executable
if(CMAKE_PROJECT_NAME STREQUAL PROJECT_NAME)
    enable_testing()
    add_subdirectory(test)
endif()
