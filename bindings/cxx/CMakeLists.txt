cmake_minimum_required(VERSION 3.15)
project(sentinel-cxx VERSION 2.0.1 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Options
option(SENTINEL_CXX_BUILD_TESTS "Build tests" ON)
option(SENTINEL_CXX_BUILD_EXAMPLES "Build examples" ON)
option(SENTINEL_CXX_SHARED "Build shared library" ON)

# Find Sentinel library
# This assumes the Rust library is built and installed
# In practice, you might want to build the Rust library as part of this CMake project

if(NOT SENTINEL_CXX_LIBRARY)
    find_library(SENTINEL_CXX_LIBRARY
        NAMES sentinel-cxx sentinel_cxx
        PATHS ${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release
              ${CMAKE_CURRENT_SOURCE_DIR}/../../../target/debug
    )
endif()

if(NOT SENTINEL_CXX_INCLUDE_DIR)
    find_path(SENTINEL_CXX_INCLUDE_DIR
        NAMES sentinel-cxx.h
        PATHS ${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release
              ${CMAKE_CURRENT_SOURCE_DIR}/../../../target/debug
    )
endif()

if(NOT SENTINEL_CXX_LIBRARY OR NOT SENTINEL_CXX_INCLUDE_DIR)
    message(FATAL_ERROR "Sentinel C library not found. Please build the Rust crate first with 'cargo build --release'")
endif()

# Create the C++ library
set(SOURCES
    src/sentinel.cpp
)

set(HEADERS
    include/sentinel/sentinel.hpp
)

add_library(sentinel-cxx ${SOURCES} ${HEADERS})

if(SENTINEL_CXX_SHARED)
    set_target_properties(sentinel-cxx PROPERTIES
        VERSION ${PROJECT_VERSION}
        SOVERSION ${PROJECT_VERSION_MAJOR}
    )
endif()

target_include_directories(sentinel-cxx
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${SENTINEL_CXX_INCLUDE_DIR}
)

target_link_libraries(sentinel-cxx PRIVATE ${SENTINEL_CXX_LIBRARY})

# Install library
install(TARGETS sentinel-cxx
    EXPORT sentinel-cxx-targets
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
    INCLUDES DESTINATION include
)

install(DIRECTORY include/
    DESTINATION include
    FILES_MATCHING PATTERN "*.hpp"
)

# Export targets
install(EXPORT sentinel-cxx-targets
    FILE sentinel-cxx-targets.cmake
    NAMESPACE sentinel::
    DESTINATION lib/cmake/sentinel-cxx
)

# Create config file
include(CMakePackageConfigHelpers)
write_basic_package_version_file(
    sentinel-cxx-config-version.cmake
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)

configure_package_config_file(
    cmake/sentinel-cxx-config.cmake.in
    sentinel-cxx-config.cmake
    INSTALL_DESTINATION lib/cmake/sentinel-cxx
    PATH_VARS
        CMAKE_INSTALL_INCLUDEDIR
)

install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/sentinel-cxx-config.cmake
    ${CMAKE_CURRENT_BINARY_DIR}/sentinel-cxx-config-version.cmake
    DESTINATION lib/cmake/sentinel-cxx
)

# Build examples
if(SENTINEL_CXX_BUILD_EXAMPLES)
    # C example
    add_executable(c_example examples/c_example.c)
    target_include_directories(c_example PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
    target_link_libraries(c_example PRIVATE sentinel-cxx)

    # C++ example
    add_executable(cpp_example examples/cpp_example.cpp)
    target_link_libraries(cpp_example PRIVATE sentinel-cxx)
endif()

# Build tests
if(SENTINEL_CXX_BUILD_TESTS)
    enable_testing()

    # C++ tests
    add_executable(cpp_tests tests/cpp_tests.cpp)
    target_link_libraries(cpp_tests PRIVATE sentinel-cxx)

    add_test(NAME cpp_tests COMMAND cpp_tests)
endif()

# Print configuration summary
message(STATUS "Sentinel C++ Configuration:")
message(STATUS "  Library: ${SENTINEL_CXX_LIBRARY}")
message(STATUS "  Include: ${SENTINEL_CXX_INCLUDE_DIR}")
message(STATUS "  Build tests: ${SENTINEL_CXX_BUILD_TESTS}")
message(STATUS "  Build examples: ${SENTINEL_CXX_BUILD_EXAMPLES}")
message(STATUS "  Shared library: ${SENTINEL_CXX_SHARED}")