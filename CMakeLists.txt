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
set(QBNEX_INTERNAL_TEMP "${QBNEX_ROOT}/internal/temp")
file(GLOB QBNEX_STAGE0_SNAPSHOT_FILES CONFIGURE_DEPENDS "${QBNEX_ROOT}/internal/source/*")

if(WIN32)
  if(NOT CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
    message(
      FATAL_ERROR
      "Windows CMake build currently requires MinGW (g++).\n"
      "Run setup_win.cmd once, then configure with a MinGW generator, for example:\n"
      "  set PATH=%CD%\\internal\\c\\c_compiler\\bin;%PATH%\n"
      "  cmake -S . -B build -G \"MinGW Makefiles\""
    )
  endif()

  set(QBNEX_OS_DIR "win")
  set(QBNEX_LIBQB_SCRIPT "setup_build.bat")
  set(QBNEX_TTF_SCRIPT "setup_build.bat")
elseif(APPLE)
  set(QBNEX_OS_DIR "osx")
  set(QBNEX_LIBQB_SCRIPT "setup_build.command")
  set(QBNEX_TTF_SCRIPT "setup_build.command")
else()
  set(QBNEX_OS_DIR "lnx")
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
  "${QBNEX_INTERNAL_C}/qbrt/include/qbrt.h"
)
qbnex_add_script_output(
  "${QBNEX_TTF_OBJ}"
  "${QBNEX_INTERNAL_C}/parts/video/font/ttf/os/${QBNEX_OS_DIR}"
  "${QBNEX_TTF_SCRIPT}"
)

# Rust runtime (internal/c/qbrt): static library linked after libqb.
if(WIN32)
  set(QBNEX_QBRT_SCRIPT "setup_build.bat")
  set(QBNEX_QBRT_NATIVE_LIBS kernel32 ntdll userenv ws2_32 dbghelp
    # R8 window layer (winit, softbuffer, gilrs/xinput)
    user32 gdi32 ole32 oleaut32 shell32 imm32 dwmapi uxtheme comctl32 advapi32 xinput)
elseif(APPLE)
  set(QBNEX_QBRT_SCRIPT "setup_build.command")
  set(QBNEX_QBRT_NATIVE_LIBS System c m
    "-framework AppKit" "-framework CoreGraphics" "-framework QuartzCore" "-framework Foundation"
    "-framework IOKit" "-framework CoreFoundation")
else()
  set(QBNEX_QBRT_SCRIPT "setup_build.sh")
  set(QBNEX_QBRT_NATIVE_LIBS pthread dl m rt util udev)
endif()
set(QBNEX_QBRT_LIB "${QBNEX_INTERNAL_C}/qbrt/os/${QBNEX_OS_DIR}/libqbrt.a")
file(GLOB_RECURSE QBNEX_QBRT_SOURCES CONFIGURE_DEPENDS
  "${QBNEX_INTERNAL_C}/qbrt/src/*.rs"
  "${QBNEX_INTERNAL_C}/qbrt/include/*.h"
)
qbnex_add_script_output(
  "${QBNEX_QBRT_LIB}"
  "${QBNEX_INTERNAL_C}/qbrt/os/${QBNEX_OS_DIR}"
  "${QBNEX_QBRT_SCRIPT}"
  ${QBNEX_QBRT_SOURCES}
  "${QBNEX_INTERNAL_C}/qbrt/Cargo.toml"
)

set(QBNEX_RUNTIME_DEPENDS "${QBNEX_LIBQB_OBJ}" "${QBNEX_TTF_OBJ}" "${QBNEX_QBRT_LIB}")

# R8: freeglut/GLEW (internal/c/parts/core) are no longer built; the window is qbrt (Rust).

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
  target_sources(qb-stage0 PRIVATE "${QBNEX_ROOT}/source/icon.rc")
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
      "${QBNEX_QBRT_LIB}"
      ${QBNEX_QBRT_NATIVE_LIBS}
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
      "${QBNEX_QBRT_LIB}"
      ${QBNEX_QBRT_NATIVE_LIBS}
      "${QBNEX_TTF_OBJ}"
      "-framework Cocoa"
  )
else()
  target_compile_options(qb-stage0 PRIVATE -w)
  target_link_libraries(
    qb-stage0
    PRIVATE
      "${QBNEX_LIBQB_OBJ}"
      "${QBNEX_QBRT_LIB}"
      ${QBNEX_QBRT_NATIVE_LIBS}
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
  COMMAND "${CMAKE_COMMAND}" -E copy_directory "${QBNEX_ROOT}/source" "${QBNEX_INTERNAL_TEMP}"
  COMMAND $<TARGET_FILE:qb-stage0> "${QBNEX_ROOT}/source/qbnex.bas" -o "${QBNEX_FINAL_BIN}"
  DEPENDS qb-stage0
  WORKING_DIRECTORY "${QBNEX_ROOT}"
  COMMENT "Self-hosting final QBNex compiler"
  VERBATIM
)

if(UNIX)
  add_custom_command(TARGET qb-selfhost POST_BUILD COMMAND "${CMAKE_COMMAND}" -E chmod +x "${QBNEX_FINAL_BIN}")
endif()

if(NOT QBNEX_KEEP_STAGE0)
  add_custom_command(TARGET qb-selfhost POST_BUILD COMMAND "${CMAKE_COMMAND}" -E rm -f "$<TARGET_FILE:qb-stage0>")
endif()

if(QBNEX_SELF_HOST)
  add_custom_target(qbnex ALL DEPENDS qb-selfhost)
else()
  add_custom_target(qbnex DEPENDS qb-stage0)
endif()
