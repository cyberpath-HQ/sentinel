cmake_minimum_required(VERSION 3.15)
project(sentinel-cxx VERSION 2.0.1 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Options
option(SENTINEL_CXX_BUILD_SHARED "Build shared library" ON)
option(SENTINEL_CXX_BUILD_STATIC "Build static library" ON)
option(SENTINEL_CXX_BUILD_TESTS "Build tests" ON)
option(SENTINEL_CXX_BUILD_EXAMPLES "Build examples" ON)
option(SENTINEL_CXX_ENABLE_ASYNC "Enable async operations" ON)

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
set(CXX_SOURCES
    src/sentinel.cpp
)

set(CXX_HEADERS
    include/sentinel/sentinel.hpp
)

# Configure library type
if(SENTINEL_CXX_BUILD_SHARED AND SENTINEL_CXX_BUILD_STATIC)
    set(LIBRARY_TYPE SHARED STATIC)
elseif(SENTINEL_CXX_BUILD_SHARED)
    set(LIBRARY_TYPE SHARED)
elseif(SENTINEL_CXX_BUILD_STATIC)
    set(LIBRARY_TYPE STATIC)
else()
    message(FATAL_ERROR "At least one of SENTINEL_CXX_BUILD_SHARED or SENTINEL_CXX_BUILD_STATIC must be ON")
endif()

add_library(sentinel-cxx ${LIBRARY_TYPE} ${CXX_SOURCES} ${CXX_HEADERS})

if(SENTINEL_CXX_BUILD_SHARED)
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

# Add compile definitions
if(SENTINEL_CXX_ENABLE_ASYNC)
    target_compile_definitions(sentinel-cxx PUBLIC SENTINEL_CXX_ASYNC_ENABLED)
endif()

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

    # C async example
    if(SENTINEL_CXX_ENABLE_ASYNC)
        add_executable(c_async_example examples/c_async_example.c)
        target_include_directories(c_async_example PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
        target_link_libraries(c_async_example PRIVATE sentinel-cxx)
    endif()

    # C++ example
    add_executable(cpp_example examples/cpp_example.cpp)
    target_link_libraries(cpp_example PRIVATE sentinel-cxx)

    # C++ async example
    if(SENTINEL_CXX_ENABLE_ASYNC)
        add_executable(cpp_async_example examples/cpp_async_example.cpp)
        target_link_libraries(cpp_async_example PRIVATE sentinel-cxx)
    endif()
endif()

# Build tests
if(SENTINEL_CXX_BUILD_TESTS)
    enable_testing()

    # C++ unit tests
    add_executable(cxx_unit_tests tests/cxx_unit_tests.cpp)
    target_link_libraries(cxx_unit_tests PRIVATE sentinel-cxx)
    add_test(NAME cxx_unit_tests COMMAND cxx_unit_tests)

    # C tests
    add_executable(c_unit_tests tests/c_unit_tests.c)
    target_include_directories(c_unit_tests PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
    target_link_libraries(c_unit_tests PRIVATE sentinel-cxx)
    add_test(NAME c_unit_tests COMMAND c_unit_tests)

    # Async tests (if enabled)
    if(SENTINEL_CXX_ENABLE_ASYNC)
        add_executable(async_tests tests/async_tests.cpp)
        target_link_libraries(async_tests PRIVATE sentinel-cxx)
        add_test(NAME async_tests COMMAND async_tests)
    endif()

    # Performance tests
    add_executable(perf_tests tests/perf_tests.cpp)
    target_link_libraries(perf_tests PRIVATE sentinel-cxx)
    add_test(NAME perf_tests COMMAND perf_tests)

    # Integration tests
    add_executable(integration_tests tests/integration_tests.cpp)
    target_link_libraries(integration_tests PRIVATE sentinel-cxx)
    add_test(NAME integration_tests COMMAND integration_tests)
endif()

# Print configuration summary
message(STATUS "Sentinel C++ Configuration:")
message(STATUS "  Library: ${SENTINEL_CXX_LIBRARY}")
message(STATUS "  Include: ${SENTINEL_CXX_INCLUDE_DIR}")
message(STATUS "  Build shared: ${SENTINEL_CXX_BUILD_SHARED}")
message(STATUS "  Build static: ${SENTINEL_CXX_BUILD_STATIC}")
message(STATUS "  Build tests: ${SENTINEL_CXX_BUILD_TESTS}")
message(STATUS "  Build examples: ${SENTINEL_CXX_BUILD_EXAMPLES}")
message(STATUS "  Enable async: ${SENTINEL_CXX_ENABLE_ASYNC}")

# Custom targets
add_custom_target(check
    COMMAND ${CMAKE_CTEST_COMMAND} --output-on-failure
    COMMENT "Running all tests"
)

add_custom_target(sync-api
    COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/../sync_api.py check
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/../..
    COMMENT "Checking API synchronization"
)

add_custom_target(update-bindings
    COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/../sync_api.py update
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/../..
    COMMENT "Updating bindings automatically"
)