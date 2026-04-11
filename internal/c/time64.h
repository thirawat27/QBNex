// time64.h — updated for C++ (migrated from C)
// Original author: Robert Walker (free source)
//
// Provides 64-bit time_t replacements for Windows builds where the
// system time_t may be limited to 32 bits on older toolchains.
//
// After including this header:
//   - time_t is redefined to t64 (int64_t)
//   - The standard time functions are replaced with 64-bit equivalents
//
// NOTE: Do NOT use %d in printf/scanf for time_t variables.
//       Use a cast to (unsigned long long) and print with %llu.
//
// NOTE: thread_local versions are available — define USE_THREAD_LOCAL_VARIABLES
//       before including this header. Not safe with LoadLibrary-loaded DLLs.
//

#pragma once

#include <ctime>
#include <cstdint>   // int64_t — portable, replaces __int64

// -----------------------------------------------------------------------
// Remove existing definitions so we can override them
// -----------------------------------------------------------------------
#undef time
#undef localtime
#undef mktime
#undef difftime
#undef gmtime
#undef time_t

// -----------------------------------------------------------------------
// 64-bit time type
// -----------------------------------------------------------------------
using t64 = int64_t;   // C++ style; portable replacement for __int64

#define time_t t64

// -----------------------------------------------------------------------
// Redirect standard CRT time functions to 64-bit implementations
// -----------------------------------------------------------------------
#define time      time_64
#define localtime localtime_64
#define mktime    mktime_64
#define difftime  difftime_64
#define gmtime    gmtime_64

// -----------------------------------------------------------------------
// Function declarations
// -----------------------------------------------------------------------
t64        time_64(t64 *pt);
struct tm *localtime_64(t64 *pt);
double     difftime_64(t64 time1, t64 time0);
t64        mktime_64(struct tm *today);
struct tm *gmtime_64(t64 t);
