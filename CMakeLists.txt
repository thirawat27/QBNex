cmake_minimum_required(VERSION 3.20)

if(WIN32)
  set(QBNEX_LANGUAGES C CXX RC)
else()
  set(QBNEX_LANGUAGES C CXX)
endif()

project(QBNex LANGUAGES ${QBNEX_LANGUAGES})

option(QBNEX_SELF_HOST "Build final qb/qb.exe with the stage0 compiler" ON)
option(QBNEX_KEEP_STAGE0 "Keep qb-stage0/qb-stage0.exe after qb-selfhost" OFF)

set(QBNEX_ROOT "${CMAKE_CURRENT_SOURCE_DIR}")
set(QBNEX_INTERNAL_C "${QBNEX_ROOT}/internal/c")
set(QBNEX_CRATES "${QBNEX_ROOT}/crates")
set(QBNEX_INTERNAL_TEMP "${QBNEX_ROOT}/internal/temp")
file(GLOB QBNEX_STAGE0_SNAPSHOT_FILES CONFIGURE_DEPENDS "${QBNEX_ROOT}/internal/source/*")

if(WIN32)
  if(NOT CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
    message(
      FATAL_ERROR
      "Windows CMake build currently requires MinGW (g++).\n"
      "Run scripts\\setup_win.cmd once, then configure with a MinGW generator, for example:\n"
      "  set PATH=%CD%\\internal\\c\\c_compiler\\bin;%PATH%\n"
      "  cmake -S . -B build -G \"MinGW Makefiles\""
    )
  endif()

  set(QBNEX_OS_DIR "win")
  # crates/*/os/<dir> and crates/qb-exec/bin/<dir> use full OS names
  set(QBNEX_CRATE_OS_DIR "windows")
  set(QBNEX_LIBQB_SCRIPT "setup_build.bat")
  set(QBNEX_TTF_SCRIPT "setup_build.bat")
elseif(APPLE)
  set(QBNEX_OS_DIR "osx")
  set(QBNEX_CRATE_OS_DIR "macos")
  set(QBNEX_LIBQB_SCRIPT "setup_build.command")
  set(QBNEX_TTF_SCRIPT "setup_build.command")
else()
  set(QBNEX_OS_DIR "lnx")
  set(QBNEX_CRATE_OS_DIR "linux")
  set(QBNEX_LIBQB_SCRIPT "setup_build.sh")
  set(QBNEX_TTF_SCRIPT "setup_build.sh")
endif()

function(qbnex_add_script_output output_file work_dir script_name)
  if(WIN32)
    add_custom_command(
      OUTPUT "${output_file}"
      COMMAND "${CMAKE_COMMAND}" -E rm -f "${output_file}"
      COMMAND cmd /c ".\\${script_name}"
      WORKING_DIRECTORY "${work_dir}"
      DEPENDS "${work_dir}/${script_name}" ${ARGN}
      VERBATIM
      COMMENT "Building ${output_file}"
    )
  else()
    add_custom_command(
      OUTPUT "${output_file}"
      COMMAND "${CMAKE_COMMAND}" -E rm -f "${output_file}"
      COMMAND /bin/sh "./${script_name}"
      WORKING_DIRECTORY "${work_dir}"
      DEPENDS "${work_dir}/${script_name}" ${ARGN}
      VERBATIM
      COMMENT "Building ${output_file}"
    )
  endif()
endfunction()

set(QBNEX_LIBQB_OBJ "${QBNEX_INTERNAL_C}/libqb/os/${QBNEX_OS_DIR}/libqb_setup.o")
set(QBNEX_TTF_OBJ "${QBNEX_INTERNAL_C}/parts/video/font/ttf/os/${QBNEX_OS_DIR}/src.o")
qbnex_add_script_output(
  "${QBNEX_LIBQB_OBJ}"
  "${QBNEX_INTERNAL_C}/libqb/os/${QBNEX_OS_DIR}"
  "${QBNEX_LIBQB_SCRIPT}"
  "${QBNEX_INTERNAL_C}/libqb.cpp"
  "${QBNEX_INTERNAL_C}/libqb.h"
  "${QBNEX_INTERNAL_C}/common.h"
  "${QBNEX_CRATES}/qb-runtime/include/qb_runtime.h"
)
qbnex_add_script_output(
  "${QBNEX_TTF_OBJ}"
  "${QBNEX_INTERNAL_C}/parts/video/font/ttf/os/${QBNEX_OS_DIR}"
  "${QBNEX_TTF_SCRIPT}"
)

# Rust runtime (crates/qb-runtime): static library linked after libqb.
if(WIN32)
  set(QBNEX_QB_RUNTIME_SCRIPT "setup_build.bat")
  set(QBNEX_QB_RUNTIME_NATIVE_LIBS kernel32 ntdll userenv ws2_32 dbghelp
    # R8 window layer (winit, softbuffer, gilrs/xinput)
    user32 gdi32 ole32 oleaut32 shell32 imm32 dwmapi uxtheme comctl32 advapi32 xinput)
elseif(APPLE)
  set(QBNEX_QB_RUNTIME_SCRIPT "setup_build.command")
  set(QBNEX_QB_RUNTIME_NATIVE_LIBS System c m
    "-framework AppKit" "-framework CoreGraphics" "-framework QuartzCore" "-framework Foundation"
    "-framework IOKit" "-framework CoreFoundation" objc)
else()
  set(QBNEX_QB_RUNTIME_SCRIPT "setup_build.sh")
  set(QBNEX_QB_RUNTIME_NATIVE_LIBS pthread dl m rt util udev)
endif()
set(QBNEX_QB_RUNTIME_LIB "${QBNEX_CRATES}/qb-runtime/os/${QBNEX_CRATE_OS_DIR}/libqb_runtime.a")
file(GLOB_RECURSE QBNEX_QB_RUNTIME_SOURCES CONFIGURE_DEPENDS
  "${QBNEX_CRATES}/qb-runtime/src/*.rs"
  "${QBNEX_CRATES}/qb-audio/src/*.rs"
  "${QBNEX_CRATES}/qb-runtime/include/*.h"
)
qbnex_add_script_output(
  "${QBNEX_QB_RUNTIME_LIB}"
  "${QBNEX_CRATES}/qb-runtime/os/${QBNEX_CRATE_OS_DIR}"
  "${QBNEX_QB_RUNTIME_SCRIPT}"
  ${QBNEX_QB_RUNTIME_SOURCES}
  "${QBNEX_CRATES}/qb-runtime/Cargo.toml"
  "${QBNEX_CRATES}/qb-audio/Cargo.toml"
)

# R11 spike: prebuilt QIR runner used by `qb --ir` (crates/qb-exec).
set(QBNEX_QB_EXEC_EXE "${QBNEX_CRATES}/qb-exec/bin/${QBNEX_CRATE_OS_DIR}/qb-exec${CMAKE_EXECUTABLE_SUFFIX}")
file(GLOB_RECURSE QBNEX_QB_EXEC_SOURCES CONFIGURE_DEPENDS "${QBNEX_CRATES}/qb-exec/src/*.rs")
qbnex_add_script_output(
  "${QBNEX_QB_EXEC_EXE}"
  "${QBNEX_CRATES}/qb-exec/os/${QBNEX_CRATE_OS_DIR}"
  "${QBNEX_QB_RUNTIME_SCRIPT}"
  ${QBNEX_QB_EXEC_SOURCES}
  ${QBNEX_QB_RUNTIME_SOURCES}
  "${QBNEX_CRATES}/qb-exec/Cargo.toml"
)
add_custom_target(qb-exec ALL DEPENDS "${QBNEX_QB_EXEC_EXE}")

set(QBNEX_RUNTIME_DEPENDS "${QBNEX_LIBQB_OBJ}" "${QBNEX_TTF_OBJ}" "${QBNEX_QB_RUNTIME_LIB}")

# R8: freeglut/GLEW (internal/c/parts/core) are no longer built; the window is qb-runtime (Rust).

add_custom_target(qbnex_runtime_objects DEPENDS ${QBNEX_RUNTIME_DEPENDS})

add_custom_target(
  qbnex_stage0_sources
  COMMAND "${CMAKE_COMMAND}" -E rm -rf "${QBNEX_INTERNAL_TEMP}"
  COMMAND "${CMAKE_COMMAND}" -E make_directory "${QBNEX_INTERNAL_TEMP}"
  COMMAND "${CMAKE_COMMAND}" -E copy_directory "${QBNEX_ROOT}/internal/source" "${QBNEX_INTERNAL_TEMP}"
  DEPENDS ${QBNEX_STAGE0_SNAPSHOT_FILES}
  COMMENT "Preparing stage0 compiler snapshot"
  VERBATIM
)

add_executable(qb-stage0 "${QBNEX_INTERNAL_C}/qbx.cpp")
add_dependencies(qb-stage0 qbnex_runtime_objects qbnex_stage0_sources)
set_target_properties(qb-stage0 PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${QBNEX_ROOT}")

if(WIN32)
  target_sources(qb-stage0 PRIVATE "${QBNEX_ROOT}/compiler/icon.rc")
  target_compile_options(qb-stage0 PRIVATE -w -Wall -Wfatal-errors)
  target_compile_options(qb-stage0 PRIVATE $<$<COMPILE_LANGUAGE:RC>:-I${QBNEX_ROOT}/internal/source>)
  target_compile_definitions(
    qb-stage0
    PRIVATE
      DEPENDENCY_SOCKETS
      DEPENDENCY_NO_PRINTER
      DEPENDENCY_NO_SCREENIMAGE
  )

  target_link_libraries(
    qb-stage0
    PRIVATE
      "${QBNEX_LIBQB_OBJ}"
      "${QBNEX_QB_RUNTIME_LIB}"
      ${QBNEX_QB_RUNTIME_NATIVE_LIBS}
      "${QBNEX_TTF_OBJ}"
      ws2_32
      winmm
      gdi32
  )
  target_link_options(qb-stage0 PRIVATE -mconsole -s -static-libgcc -static-libstdc++)
elseif(APPLE)
  target_compile_options(qb-stage0 PRIVATE -w)
  target_link_libraries(
    qb-stage0
    PRIVATE
      "${QBNEX_LIBQB_OBJ}"
      "${QBNEX_QB_RUNTIME_LIB}"
      ${QBNEX_QB_RUNTIME_NATIVE_LIBS}
      "${QBNEX_TTF_OBJ}"
      "-framework Cocoa"
  )
else()
  target_compile_options(qb-stage0 PRIVATE -w)
  target_link_libraries(
    qb-stage0
    PRIVATE
      "${QBNEX_LIBQB_OBJ}"
      "${QBNEX_QB_RUNTIME_LIB}"
      ${QBNEX_QB_RUNTIME_NATIVE_LIBS}
      "${QBNEX_TTF_OBJ}"
      pthread
      dl
      rt
  )
  target_link_options(qb-stage0 PRIVATE -no-pie)
endif()

if(WIN32)
  set(QBNEX_FINAL_BIN "${QBNEX_ROOT}/qb.exe")
else()
  set(QBNEX_FINAL_BIN "${QBNEX_ROOT}/qb")
endif()

add_custom_target(
  qb-selfhost
  COMMAND "${CMAKE_COMMAND}" -E rm -rf "${QBNEX_INTERNAL_TEMP}"
  COMMAND "${CMAKE_COMMAND}" -E make_directory "${QBNEX_INTERNAL_TEMP}"
  COMMAND "${CMAKE_COMMAND}" -E copy_directory "${QBNEX_ROOT}/internal/source" "${QBNEX_INTERNAL_TEMP}"
  COMMAND "${CMAKE_COMMAND}" -E copy_directory "${QBNEX_ROOT}/compiler" "${QBNEX_INTERNAL_TEMP}"
  COMMAND $<TARGET_FILE:qb-stage0> "${QBNEX_ROOT}/compiler/qbnex.bas" -o "${QBNEX_FINAL_BIN}"
  DEPENDS qb-stage0
  WORKING_DIRECTORY "${QBNEX_ROOT}"
  COMMENT "Self-hosting final QBNex compiler"
  VERBATIM
)

add_dependencies(qb-selfhost qb-exec)

if(UNIX)
  add_custom_command(TARGET qb-selfhost POST_BUILD COMMAND "${CMAKE_COMMAND}" -E chmod +x "${QBNEX_FINAL_BIN}")
  # Product-name alias: ./qbnex runs the same compiler as ./qb (Windows uses the tracked qbnex.cmd).
  add_custom_command(TARGET qb-selfhost POST_BUILD
    COMMAND "${CMAKE_COMMAND}" -E rm -f "${QBNEX_ROOT}/qbnex"
    COMMAND "${CMAKE_COMMAND}" -E create_symlink qb "${QBNEX_ROOT}/qbnex")
endif()

if(NOT QBNEX_KEEP_STAGE0)
  add_custom_command(TARGET qb-selfhost POST_BUILD COMMAND "${CMAKE_COMMAND}" -E rm -f "$<TARGET_FILE:qb-stage0>")
endif()

if(QBNEX_SELF_HOST)
  add_custom_target(qbnex ALL DEPENDS qb-selfhost)
else()
  add_custom_target(qbnex DEPENDS qb-stage0)
endif()
