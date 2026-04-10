/**
 * @file qbnex_version.h
 * @brief QBNex Compiler Version Information
 * @copyright Copyright © 2026 thirawat27
 * @license MIT License
 * @version 1.0.0
 */

#ifndef QBNEX_VERSION_H
#define QBNEX_VERSION_H

// Version information
#define QBNEX_VERSION_MAJOR 1
#define QBNEX_VERSION_MINOR 0
#define QBNEX_VERSION_PATCH 0
#define QBNEX_VERSION_STRING "1.0.0"

// Project information
#define QBNEX_PROJECT_NAME "QBNex"
#define QBNEX_OWNER "thirawat27"
#define QBNEX_YEAR "2026"
#define QBNEX_REPOSITORY "https://github.com/thirawat27/QBNex"

// Build information
#define QBNEX_BUILD_TYPE "Release"
#define QBNEX_COMPILER_NAME "QBNex CLI Compiler"
#define QBNEX_DESCRIPTION "Modern BASIC to Native Executable Compiler"

// Feature flags
#define QBNEX_FEATURE_OPENGL 1
#define QBNEX_FEATURE_AUDIO 1
#define QBNEX_FEATURE_NETWORK 1
#define QBNEX_FEATURE_MULTITHREADING 1

// Platform detection
#if defined(_WIN32) || defined(_WIN64)
    #define QBNEX_PLATFORM_WINDOWS 1
    #define QBNEX_PLATFORM_NAME "Windows"
#elif defined(__APPLE__) && defined(__MACH__)
    #define QBNEX_PLATFORM_MACOS 1
    #define QBNEX_PLATFORM_NAME "macOS"
#elif defined(__linux__)
    #define QBNEX_PLATFORM_LINUX 1
    #define QBNEX_PLATFORM_NAME "Linux"
#else
    #define QBNEX_PLATFORM_UNKNOWN 1
    #define QBNEX_PLATFORM_NAME "Unknown"
#endif

// Architecture detection
#if defined(__x86_64__) || defined(_M_X64)
    #define QBNEX_ARCH_X64 1
    #define QBNEX_ARCH_BITS 64
#elif defined(__i386__) || defined(_M_IX86)
    #define QBNEX_ARCH_X86 1
    #define QBNEX_ARCH_BITS 32
#elif defined(__aarch64__) || defined(_M_ARM64)
    #define QBNEX_ARCH_ARM64 1
    #define QBNEX_ARCH_BITS 64
#elif defined(__arm__) || defined(_M_ARM)
    #define QBNEX_ARCH_ARM 1
    #define QBNEX_ARCH_BITS 32
#else
    #define QBNEX_ARCH_UNKNOWN 1
    #define QBNEX_ARCH_BITS 0
#endif

// Compiler detection
#if defined(__GNUC__)
    #define QBNEX_COMPILER_GCC 1
    #define QBNEX_COMPILER_VERSION __VERSION__
#elif defined(_MSC_VER)
    #define QBNEX_COMPILER_MSVC 1
    #define QBNEX_COMPILER_VERSION _MSC_VER
#elif defined(__clang__)
    #define QBNEX_COMPILER_CLANG 1
    #define QBNEX_COMPILER_VERSION __clang_version__
#else
    #define QBNEX_COMPILER_UNKNOWN 1
    #define QBNEX_COMPILER_VERSION "Unknown"
#endif

/**
 * @brief Get full version string with platform info
 * @return Formatted version string
 */
inline const char* qbnex_get_full_version() {
    return QBNEX_PROJECT_NAME " v" QBNEX_VERSION_STRING " (" QBNEX_PLATFORM_NAME " " QBNEX_ARCH_BITS "-bit)";
}

/**
 * @brief Get copyright notice
 * @return Copyright string
 */
inline const char* qbnex_get_copyright() {
    return "Copyright © " QBNEX_YEAR " " QBNEX_OWNER;
}

#endif // QBNEX_VERSION_H
