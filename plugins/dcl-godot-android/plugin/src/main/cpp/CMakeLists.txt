cmake_minimum_required(VERSION 3.18.1)

project("exoplayer_hwbuffer")

add_library(
    exoplayer_hwbuffer
    SHARED
    exoplayer_hwbuffer.cpp
)

# Link against Android libraries
target_link_libraries(
    exoplayer_hwbuffer
    android
    log
)

# Set C++ standard
set_target_properties(
    exoplayer_hwbuffer
    PROPERTIES
    CXX_STANDARD 17
    CXX_STANDARD_REQUIRED ON
)
