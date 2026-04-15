/*
 * QBNex Runtime Common Definitions
 * 
 * This header provides common macros, types, and utilities used across
 * all libqb modules.
 */

#ifndef LIBQB_COMMON_H
#define LIBQB_COMMON_H

#ifdef __cplusplus
extern "C" {
#endif

/* Platform detection */
#if defined(_WIN32) || defined(_WIN64)
    #define QB_PLATFORM_WINDOWS
#elif defined(__linux__)
    #define QB_PLATFORM_LINUX
#elif defined(__APPLE__) && defined(__MACH__)
    #define QB_PLATFORM_MACOS
#else
    #define QB_PLATFORM_UNKNOWN
#endif

/* Architecture detection */
#if defined(_WIN64) || defined(__x86_64__) || defined(__amd64__) || defined(__aarch64__)
    #define QB_ARCH_64BIT
#else
    #define QB_ARCH_32BIT
#endif

/* Compiler detection */
#if defined(_MSC_VER)
    #define QB_COMPILER_MSVC
    #define QB_COMPILER_VERSION _MSC_VER
#elif defined(__GNUC__)
    #define QB_COMPILER_GCC
    #define QB_COMPILER_VERSION (__GNUC__ * 10000 + __GNUC_MINOR__ * 100 + __GNUC_PATCHLEVEL__)
#elif defined(__clang__)
    #define QB_COMPILER_CLANG
    #define QB_COMPILER_VERSION (__clang_major__ * 10000 + __clang_minor__ * 100 + __clang_patchlevel__)
#else
    #define QB_COMPILER_UNKNOWN
#endif

/* Export macros */
#ifdef QB_PLATFORM_WINDOWS
    #ifdef QB_BUILDING_DLL
        #define QB_API __declspec(dllexport)
    #else
        #define QB_API __declspec(dllimport)
    #endif
    #define QB_LOCAL
#else
    #define QB_API __attribute__((visibility("default")))
    #define QB_LOCAL __attribute__((visibility("hidden")))
#endif

/* Deprecation warnings */
#ifdef QB_COMPILER_MSVC
    #define QB_DEPRECATED(msg) __declspec(deprecated(msg))
#elif defined(QB_COMPILER_GCC) || defined(QB_COMPILER_CLANG)
    #define QB_DEPRECATED(msg) __attribute__((deprecated(msg)))
#else
    #define QB_DEPRECATED(msg)
#endif

/* Inline hints */
#ifdef QB_COMPILER_MSVC
    #define QB_INLINE __forceinline
    #define QB_NOINLINE __declspec(noinline)
#elif defined(QB_COMPILER_GCC) || defined(QB_COMPILER_CLANG)
    #define QB_INLINE inline __attribute__((always_inline))
    #define QB_NOINLINE __attribute__((noinline))
#else
    #define QB_INLINE inline
    #define QB_NOINLINE
#endif

/* Likely/Unlikely branch prediction hints */
#if defined(QB_COMPILER_GCC) || defined(QB_COMPILER_CLANG)
    #define QB_LIKELY(x) __builtin_expect(!!(x), 1)
    #define QB_UNLIKELY(x) __builtin_expect(!!(x), 0)
#else
    #define QB_LIKELY(x) (x)
    #define QB_UNLIKELY(x) (x)
#endif

/* Alignment macros */
#ifdef QB_COMPILER_MSVC
    #define QB_ALIGN(x) __declspec(align(x))
#else
    #define QB_ALIGN(x) __attribute__((aligned(x)))
#endif

/* Basic types */
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

typedef int8_t   qb_int8;
typedef uint8_t  qb_uint8;
typedef int16_t  qb_int16;
typedef uint16_t qb_uint16;
typedef int32_t  qb_int32;
typedef uint32_t qb_uint32;
typedef int64_t  qb_int64;
typedef uint64_t qb_uint64;
typedef float    qb_float;
typedef double   qb_double;

/* QB-specific types */
typedef int16_t  qb_integer;  /* QB INTEGER: -32768 to 32767 */
typedef int32_t  qb_long;     /* QB LONG: -2147483648 to 2147483647 */
typedef int64_t  qb_integer64;/* QB INTEGER64 */
typedef uint8_t  qb_byte;     /* QB _BYTE: 0 to 255 */
typedef uint32_t qb_offset;   /* QB _OFFSET */
typedef float    qb_single;   /* QB SINGLE precision float */
typedef double   qb_double;   /* QB DOUBLE precision float */

/* Result codes */
typedef enum {
    QB_OK = 0,
    QB_ERROR_INVALID_PARAM = -1,
    QB_ERROR_OUT_OF_MEMORY = -2,
    QB_ERROR_FILE_NOT_FOUND = -3,
    QB_ERROR_PERMISSION_DENIED = -4,
    QB_ERROR_INVALID_OPERATION = -5,
    QB_ERROR_OVERFLOW = -6,
    QB_ERROR_UNDERFLOW = -7,
    QB_ERROR_DIVIDE_BY_ZERO = -8,
    QB_ERROR_NOT_IMPLEMENTED = -9,
    QB_ERROR_UNKNOWN = -99
} qb_result_t;

/* Version information */
#define QB_VERSION_MAJOR 1
#define QB_VERSION_MINOR 0
#define QB_VERSION_PATCH 0
#define QB_VERSION_STRING "1.0.0"

/* Feature flags for conditional compilation */
#define QB_FEATURE_GRAPHICS    1
#define QB_FEATURE_AUDIO       1
#define QB_FEATURE_NETWORKING  0
#define QB_FEATURE_THREADING   0
#define QB_FEATURE_PRINTER     1
#define QB_FEATURE_GUI         0

/* Debug and trace macros */
#ifdef QB_DEBUG
    #include <stdio.h>
    #define QB_TRACE(fmt, ...) fprintf(stderr, "[TRACE] " fmt "\n", ##__VA_ARGS__)
    #define QB_ASSERT(cond) do { if (!(cond)) { fprintf(stderr, "[ASSERT] Failed: " #cond " at %s:%d\n", __FILE__, __LINE__); __debugbreak(); } } while(0)
#else
    #define QB_TRACE(fmt, ...)
    #define QB_ASSERT(cond)
#endif

/* Memory allocation macros with error checking */
#define QB_MALLOC(size) malloc(size)
#define QB_CALLOC(count, size) calloc(count, size)
#define QB_REALLOC(ptr, size) realloc(ptr, size)
#define QB_FREE(ptr) do { free(ptr); (ptr) = NULL; } while(0)

/* Safe array access */
#define QB_ARRAY_INDEX_CHECK(index, size) \
    (QB_LIKELY((index) >= 0 && (index) < (size)))

/* String handling */
#define QB_STRING_MAX_LENGTH 2147483647  /* 2^31 - 1 */
#define QB_STRING_INITIAL_CAPACITY 256

/* Error handling */
typedef void (*qb_error_handler_t)(int error_code, const char* message, const char* file, int line);
extern qb_error_handler_t qb_current_error_handler;

#define QB_SET_ERROR_HANDLER(handler) (qb_current_error_handler = (handler))
#define QB_RAISE_ERROR(code, msg) do { \
    if (qb_current_error_handler) { \
        qb_current_error_handler((code), (msg), __FILE__, __LINE__); \
    } \
} while(0)

/* Initialization and cleanup */
typedef struct {
    int graphics_enabled;
    int audio_enabled;
    int networking_enabled;
    int memory_pool_size;
    int string_pool_size;
    const char* log_file_path;
} qb_init_params_t;

QB_API qb_result_t qb_initialize(const qb_init_params_t* params);
QB_API void qb_shutdown(void);
QB_API bool qb_is_initialized(void);

/* Memory statistics */
typedef struct {
    size_t total_allocated;
    size_t total_freed;
    size_t current_used;
    size_t peak_used;
    size_t allocation_count;
    size_t free_count;
} qb_memory_stats_t;

QB_API void qb_get_memory_stats(qb_memory_stats_t* stats);
QB_API void qb_reset_memory_stats(void);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* LIBQB_COMMON_H */
