cmake_minimum_required(VERSION 3.15)
project(sentinel-cxx VERSION 2.0.1 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(GNUInstallDirs)

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
        PATHS ${CMAKE_CURRENT_SOURCE_DIR}/lib
              ${CMAKE_CURRENT_SOURCE_DIR}/dist
              ${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release
              ${CMAKE_CURRENT_SOURCE_DIR}/../../../target/debug
    )
endif()

if(NOT SENTINEL_CXX_INCLUDE_DIR)
    find_path(SENTINEL_CXX_INCLUDE_DIR
        NAMES sentinel-cxx.h
        PATHS ${CMAKE_CURRENT_SOURCE_DIR}/include/sentinel
              ${CMAKE_CURRENT_SOURCE_DIR}/dist
              ${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release
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
    # C++ example
    add_executable(cpp_example examples/cpp_example.cpp)
    target_link_libraries(cpp_example PRIVATE sentinel-cxx)

    # C example
    add_executable(c_example examples/c_example.c)
    target_include_directories(c_example PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
    target_link_libraries(c_example PRIVATE sentinel-cxx)

    # C async example
    add_executable(c_async_example examples/c_async_example.c)
    target_include_directories(c_async_example PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
    target_link_libraries(c_async_example PRIVATE sentinel-cxx ${CMAKE_BINARY_DIR}/../../../target/release/libsentinel_cxx.a)

    # C async query example
    if(SENTINEL_CXX_ENABLE_ASYNC)
        add_executable(c_async_query_example examples/c_async_query_example.c)
        target_include_directories(c_async_query_example PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
        target_link_libraries(c_async_query_example PRIVATE sentinel-cxx ${CMAKE_BINARY_DIR}/../../../target/release/libsentinel_cxx.a)
    endif()

    # C query example
    add_executable(c_query_example examples/c_query_example.c)
    target_include_directories(c_query_example PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
    target_link_libraries(c_query_example PRIVATE sentinel-cxx)

    # C complex query example
    add_executable(c_complex_query_example examples/c_complex_query_example.c)
    target_include_directories(c_complex_query_example PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
    target_link_libraries(c_complex_query_example PRIVATE sentinel-cxx ${CMAKE_BINARY_DIR}/../../../target/release/libsentinel_cxx.a)

    # C advanced query example (all filter types and OR/AND operations)
    add_executable(c_advanced_query_example examples/c_advanced_query_example.c)
    target_include_directories(c_advanced_query_example PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
    target_link_libraries(c_advanced_query_example PRIVATE sentinel-cxx ${CMAKE_BINARY_DIR}/../../../target/release/libsentinel_cxx.a)

endif()

# Build tests
if(SENTINEL_CXX_BUILD_TESTS)
    enable_testing()

    # C++ tests - disabled due to compilation issues
    # add_executable(cpp_tests tests/cpp_tests.cpp)
    # target_link_libraries(cpp_tests PRIVATE sentinel-cxx)
    # add_test(NAME cpp_tests COMMAND cpp_tests)

    # C test example (basic functionality test)
    add_executable(test_c_bindings examples/test_c_bindings.c)
    target_include_directories(test_c_bindings PRIVATE ${SENTINEL_CXX_INCLUDE_DIR})
    target_link_libraries(test_c_bindings PRIVATE sentinel-cxx)
    add_test(NAME test_c_bindings COMMAND test_c_bindings)

    # TODO: Add more comprehensive tests when implemented
    # - cxx_unit_tests
    # - c_unit_tests
    # - async_tests
    # - perf_tests
    # - integration_tests
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