/* Provide some OS/compiler macros.
    * QBNex_WINDOWS: Is this a Windows system?
    * QBNex_LINUX: Is this a Linux system?
    * QBNex_MACOSX: Is this MacOSX, or MacOS or whatever Apple calls it now?
    * QBNex_UNIX: Is this a Unix-flavoured system?
    *
    * QBNex_BACKSLASH_FILESYSTEM: Does this system use \ for file paths (as opposed to /)?
    * QBNex_MICROSOFT: Are we compiling with Visual Studio?
    * QBNex_GCC: Are we compiling with gcc?
    * QBNex_MINGW: Are we compiling with MinGW, specifically? (Set in addition to QBNex_GCC)
    *
    * QBNex_32: A 32bit system (the default)
    * QBNex_64: A 64bit system (assumes all Macs are 64 bit)
*/
#ifdef WIN32
    #define QBNex_WINDOWS
    #define QBNex_BACKSLASH_FILESYSTEM
    #ifdef _MSC_VER
        //Do we even support non-mingw compilers on Windows?
        #define QBNex_MICROSOFT
        #else
        #define QBNex_GCC
        #define QBNex_MINGW
    #endif
    #elif defined(__APPLE__)
    #define QBNex_MACOSX
    #define QBNex_UNIX
    #define QBNex_GCC
    #elif defined(__linux__)
    #define QBNex_LINUX
    #define QBNex_UNIX
    #define QBNex_GCC
    #else
    #error "Unknown system; refusing to build. Edit os.h if needed"
#endif

#if defined(_WIN64) || defined(__x86_64__) || defined(__ppc64__) || defined(__PPC64__) || defined(QBNex_MACOSX) || defined(__aarch64__)
    #define QBNex_64
    #else
    #define QBNex_32
#endif

#if !defined(i386) && !defined(__x86_64__)
    #define QBNex_NOT_X86
#endif

/* common types (not quite an include guard, but allows an including
    * file to not have these included.
    *
    * Should this be adapted to check for each type before defining?
*/
#ifndef QBNex_OS_H_NO_TYPES
    #ifdef QBNex_WINDOWS
        #define uint64 unsigned __int64
        #define uint32 unsigned __int32
        #define uint16 unsigned __int16
        #define uint8 unsigned __int8
        #define int64 __int64
        #define int32 __int32
        #define int16 __int16
        #define int8 __int8
        #else
        #define int64 int64_t
        #define int32 int32_t
        #define int16 int16_t
        #define int8 int8_t
        #define uint64 uint64_t
        #define uint32 uint32_t
        #define uint16 uint16_t
        #define uint8 uint8_t
    #endif
    
    #ifdef QBNex_64
        #define ptrszint int64
        #define uptrszint uint64
        #define ptrsz 8
        #else
        #define ptrszint int32
        #define uptrszint uint32
        #define ptrsz 4
    #endif
#endif
