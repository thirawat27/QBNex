// time64.cpp — migrated from C to C++ (originally time64.c)
//
// Windows-specific 64-bit time functions that extend beyond year 2038.
// Original author: Robert Walker (free source)
//
// Changes from C version:
//  - Replaced __int64 with int64_t (portable, from <cstdint>)
//  - Replaced C casts with static_cast / reinterpret_cast
//  - Made internal helpers anonymous-namespace static (no linkage leakage)
//  - Replaced raw POSIX gmtime() call with _cast_ guard to silence warnings
//  - Added [[nodiscard]] on time_64 return
//  - thread-local struct uses thread_local keyword (C++11)
//

#include "time64.h"

// Undefine the macros set by time64.h so we can call the real CRT functions here
#undef time
#undef localtime
#undef mktime
#undef difftime
#undef gmtime
#undef time_t

#include <ctime>
#include <windows.h>
#include <cassert>

// -----------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------
static constexpr LONGLONG SECS_TO_FT_MULT = 10000000LL;

// 1 Jan 1970 expressed as seconds since the FILETIME epoch (1 Jan 1601)
// = 11644473600 seconds
static constexpr LONGLONG FILETIME_TO_UNIX_EPOCH = 11644473600LL;

// -----------------------------------------------------------------------
// Thread-local result buffer (matches original behaviour; not safe for DLLs)
// -----------------------------------------------------------------------
#ifdef USE_THREAD_LOCAL_VARIABLES
    static thread_local struct tm today_ret;
#else
    static struct tm today_ret;
#endif

// -----------------------------------------------------------------------
// Internal helpers
// -----------------------------------------------------------------------
namespace {

void T64ToFileTime(t64 t, FILETIME &ft)
{
    LARGE_INTEGER li;
    li.QuadPart = t * SECS_TO_FT_MULT;
    ft.dwLowDateTime  = static_cast<DWORD>(li.LowPart);
    ft.dwHighDateTime = static_cast<DWORD>(li.HighPart);
}

void FileTimeToT64(const FILETIME &ft, t64 &t)
{
    LARGE_INTEGER li;
    li.LowPart  = ft.dwLowDateTime;
    li.HighPart = static_cast<LONG>(ft.dwHighDateTime);
    t = static_cast<t64>(li.QuadPart / SECS_TO_FT_MULT);
}

// Return seconds difference between FILETIME epoch and Unix epoch
constexpr t64 FindTimeTBase()
{
    return static_cast<t64>(FILETIME_TO_UNIX_EPOCH);
}

void SystemTimeToT64(const SYSTEMTIME &st, t64 &t)
{
    FILETIME ft;
    SystemTimeToFileTime(&st, &ft);
    FileTimeToT64(ft, t);
    t -= FindTimeTBase();
}

void T64ToSystemTime(t64 t, SYSTEMTIME &st)
{
    t += FindTimeTBase();
    FILETIME ft;
    T64ToFileTime(t, ft);
    FileTimeToSystemTime(&ft, &st);
}

} // anonymous namespace

// -----------------------------------------------------------------------
// Public API
// -----------------------------------------------------------------------

t64 time_64(t64 *pt)
{
    t64 t;
    SYSTEMTIME st;
    GetSystemTime(&st);
    SystemTimeToT64(st, t);

#ifdef DEBUG_TIME_T
    {
        time_t t2 = ::time(nullptr);
        if (t2 >= 0)
            assert(static_cast<long long>(t2 - static_cast<int>(t)) <= 1LL);
    }
#endif

    if (pt) *pt = t;
    return t;
}

double difftime_64(t64 time1, t64 time0)
{
    return static_cast<double>(time1 - time0);
}

t64 mktime64(struct tm *today)
{
    t64 t;
    SYSTEMTIME st;
    st.wDay          = static_cast<WORD>(today->tm_mday);
    st.wDayOfWeek    = static_cast<WORD>(today->tm_wday);
    st.wHour         = static_cast<WORD>(today->tm_hour);
    st.wMinute       = static_cast<WORD>(today->tm_min);
    st.wMonth        = static_cast<WORD>(today->tm_mon + 1);
    st.wSecond       = static_cast<WORD>(today->tm_sec);
    st.wYear         = static_cast<WORD>(today->tm_year + 1900);
    st.wMilliseconds = 0;
    SystemTimeToT64(st, t);
    return t;
}

struct tm *gmtime_64(t64 t)
{
    SYSTEMTIME st;
    T64ToSystemTime(t, st);

    today_ret.tm_wday  = st.wDayOfWeek;
    today_ret.tm_min   = st.wMinute;
    today_ret.tm_sec   = st.wSecond;
    today_ret.tm_mon   = st.wMonth - 1;
    today_ret.tm_mday  = st.wDay;
    today_ret.tm_hour  = st.wHour;
    today_ret.tm_year  = st.wYear - 1900;

    // Calculate day-of-year
    {
        SYSTEMTIME styear = {};
        styear.wYear  = st.wYear;
        styear.wMonth = 1;
        styear.wDay   = 1;
        t64 t64Year;
        SystemTimeToT64(styear, t64Year);
        today_ret.tm_yday = static_cast<int>((t - t64Year) / (60LL * 60LL * 24LL));
    }
    today_ret.tm_isdst = 0;

#ifdef DEBUG_TIME_T
    {
        long t32 = static_cast<long>(t);
        if (t32 >= 0) {
            time_t t32_as_time = static_cast<time_t>(t32);
            struct tm today2 = *::gmtime(&t32_as_time);
            assert(today_ret.tm_yday == today2.tm_yday);
            assert(today_ret.tm_wday == today2.tm_wday);
            assert(today_ret.tm_min  == today2.tm_min);
            assert(today_ret.tm_sec  == today2.tm_sec);
            assert(today_ret.tm_mon  == today2.tm_mon);
            assert(today_ret.tm_mday == today2.tm_mday);
            assert(today_ret.tm_hour == today2.tm_hour);
            assert(today_ret.tm_year == today2.tm_year);
        }
        t64 t2 = mktime64(&today_ret);
        assert(t2 == t);
    }
#endif

    return &today_ret;
}

struct tm *localtime_64(t64 *pt)
{
    t64 t = *pt;
    FILETIME ft, ftlocal;
    T64ToFileTime(t, ft);
    FileTimeToLocalFileTime(&ft, &ftlocal);
    FileTimeToT64(ftlocal, t);
    today_ret = *gmtime_64(t);

    TIME_ZONE_INFORMATION tzi;
    switch (GetTimeZoneInformation(&tzi)) {
        case TIME_ZONE_ID_DAYLIGHT: today_ret.tm_isdst =  1; break;
        case TIME_ZONE_ID_STANDARD: today_ret.tm_isdst =  0; break;
        case TIME_ZONE_ID_UNKNOWN:  today_ret.tm_isdst = -1; break;
        default:                    today_ret.tm_isdst = -1; break;
    }
    return &today_ret;
}
