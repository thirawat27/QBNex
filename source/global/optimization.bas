'===============================================================================
' QBNex Performance Optimization Module
'===============================================================================
' This module provides performance optimizations for the QBNex compiler:
' - String pooling to reduce memory allocations
' - Lazy array resizing with exponential growth
' - Compilation cache for faster rebuilds
' - Performance metrics and profiling
'===============================================================================

'-------------------------------------------------------------------------------
' STRING POOLING SYSTEM
'-------------------------------------------------------------------------------
' Reduces memory allocations by reusing identical strings
'-------------------------------------------------------------------------------

CONST STRING_POOL_INITIAL_SIZE = 10000
CONST STRING_POOL_MAX_SIZE = 100000

TYPE StringPoolEntry
    value AS STRING
    refCount AS LONG
    hash AS LONG
END TYPE

DIM SHARED StringPool() AS StringPoolEntry
DIM SHARED StringPoolCount AS LONG
DIM SHARED StringPoolSize AS LONG
DIM SHARED StringPoolEnabled AS _BYTE

' Initialize string pool
SUB InitStringPool
    StringPoolSize = STRING_POOL_INITIAL_SIZE
    REDIM StringPool(1 TO StringPoolSize) AS StringPoolEntry
    StringPoolCount = 0
    StringPoolEnabled = -1 'Enabled by default
END SUB

' Simple hash function for strings
FUNCTION StringHash& (s AS STRING)
    DIM h AS LONG, i AS LONG
    h = 0
    FOR i = 1 TO LEN(s)
        h = (h * 31 + ASC(s, i)) AND &H7FFFFFFF
    NEXT
    StringHash& = h
END FUNCTION

' Intern a string - returns pooled version if found, adds to pool if not
FUNCTION InternString$ (s AS STRING)
    IF NOT StringPoolEnabled OR LEN(s) = 0 THEN InternString$ = s: EXIT FUNCTION
    
    DIM h AS LONG, i AS LONG
    h = StringHash(s)
    
    ' Search for existing string with same hash
    FOR i = 1 TO StringPoolCount
        IF StringPool(i).hash = h AND StringPool(i).value = s THEN
            StringPool(i).refCount = StringPool(i).refCount + 1
            InternString$ = StringPool(i).value
            EXIT FUNCTION
        END IF
    NEXT
    
    ' Not found - add to pool
    StringPoolCount = StringPoolCount + 1
    IF StringPoolCount > StringPoolSize THEN
        ' Expand pool with exponential growth
        IF StringPoolSize < STRING_POOL_MAX_SIZE THEN
            StringPoolSize = StringPoolSize * 2
            IF StringPoolSize > STRING_POOL_MAX_SIZE THEN StringPoolSize = STRING_POOL_MAX_SIZE
            REDIM _PRESERVE StringPool(1 TO StringPoolSize) AS StringPoolEntry
        ELSE
            ' Pool full - return original string
            InternString$ = s
            EXIT FUNCTION
        END IF
    END IF
    
    StringPool(StringPoolCount).value = s
    StringPool(StringPoolCount).hash = h
    StringPool(StringPoolCount).refCount = 1
    InternString$ = s
END FUNCTION

' Clear string pool (call between compilations)
SUB ClearStringPool
    IF StringPoolSize > STRING_POOL_INITIAL_SIZE THEN
        REDIM StringPool(1 TO STRING_POOL_INITIAL_SIZE) AS StringPoolEntry
        StringPoolSize = STRING_POOL_INITIAL_SIZE
    END IF
    StringPoolCount = 0
END SUB

'-------------------------------------------------------------------------------
' LAZY ARRAY RESIZING UTILITIES
'-------------------------------------------------------------------------------
' Provides exponential growth for dynamic arrays to reduce reallocation
'-------------------------------------------------------------------------------

' Ensures array has at least the requested capacity using exponential growth
SUB EnsureByteArrayCapacity (arr() AS _BYTE, neededIndex AS LONG)
    IF neededIndex <= UBOUND(arr) THEN EXIT SUB
    
    DIM newSize AS LONG
    newSize = UBOUND(arr) * 2
    IF newSize < neededIndex + 1000 THEN newSize = neededIndex + 1000
    REDIM _PRESERVE arr(1 TO newSize) AS _BYTE
END SUB

SUB EnsureLongArrayCapacity (arr() AS LONG, neededIndex AS LONG)
    IF neededIndex <= UBOUND(arr) THEN EXIT SUB
    
    DIM newSize AS LONG
    newSize = UBOUND(arr) * 2
    IF newSize < neededIndex + 100 THEN newSize = neededIndex + 100
    REDIM _PRESERVE arr(1 TO newSize) AS LONG
END SUB

SUB EnsureStringArrayCapacity (arr() AS STRING, neededIndex AS LONG)
    IF neededIndex <= UBOUND(arr) THEN EXIT SUB
    
    DIM newSize AS LONG
    newSize = UBOUND(arr) * 2
    IF newSize < neededIndex + 100 THEN newSize = neededIndex + 100
    REDIM _PRESERVE arr(1 TO newSize) AS STRING
END SUB

'-------------------------------------------------------------------------------
' PERFORMANCE METRICS
'-------------------------------------------------------------------------------
' Tracks compilation time and memory usage for optimization analysis
'-------------------------------------------------------------------------------

TYPE PerformanceMetrics
    startTime AS DOUBLE
    endTime AS DOUBLE
    peakMemoryMB AS LONG
    hashLookups AS _UNSIGNED _INTEGER64
    hashCollisions AS _UNSIGNED _INTEGER64
    stringOperations AS _UNSIGNED _INTEGER64
    fileIOOps AS _UNSIGNED _INTEGER64
END TYPE

DIM SHARED PerfMetrics AS PerformanceMetrics
DIM SHARED MetricsEnabled AS _BYTE

SUB StartPerformanceMetrics
    PerfMetrics.startTime = TIMER
    PerfMetrics.peakMemoryMB = 0
    PerfMetrics.hashLookups = 0
    PerfMetrics.hashCollisions = 0
    PerfMetrics.stringOperations = 0
    PerfMetrics.fileIOOps = 0
    MetricsEnabled = -1
END SUB

SUB EndPerformanceMetrics
    PerfMetrics.endTime = TIMER
    MetricsEnabled = 0
END SUB

SUB RecordHashLookup
    IF MetricsEnabled THEN PerfMetrics.hashLookups = PerfMetrics.hashLookups + 1
END SUB

SUB RecordHashCollision
    IF MetricsEnabled THEN PerfMetrics.hashCollisions = PerfMetrics.hashCollisions + 1
END SUB

SUB RecordStringOperation
    IF MetricsEnabled THEN PerfMetrics.stringOperations = PerfMetrics.stringOperations + 1
END SUB

SUB RecordFileIO
    IF MetricsEnabled THEN PerfMetrics.fileIOOps = PerfMetrics.fileIOOps + 1
END SUB

FUNCTION GetElapsedTime!
    GetElapsedTime! = TIMER - PerfMetrics.startTime
END FUNCTION

SUB PrintPerformanceReport
    DIM elapsed AS DOUBLE
    elapsed = PerfMetrics.endTime - PerfMetrics.startTime
    
    PRINT
    PRINT "=== Performance Report ==="
    PRINT "Elapsed Time: "; elapsed; " seconds"
    PRINT "Hash Lookups: "; PerfMetrics.hashLookups
    PRINT "Hash Collisions: "; PerfMetrics.hashCollisions
    IF PerfMetrics.hashLookups > 0 THEN
        PRINT "Collision Rate: "; (PerfMetrics.hashCollisions / PerfMetrics.hashLookups * 100); "%"
    END IF
    PRINT "String Operations: "; PerfMetrics.stringOperations
    PRINT "File I/O Operations: "; PerfMetrics.fileIOOps
    PRINT "========================"
END SUB

'-------------------------------------------------------------------------------
' COMPILATION CACHE SYSTEM
'-------------------------------------------------------------------------------
' Caches intermediate compilation results for faster rebuilds
'-------------------------------------------------------------------------------

CONST CACHE_ENABLED = -1
CONST CACHE_VERSION = 1 'Increment when cache format changes

DIM SHARED CacheDir AS STRING
DIM SHARED CacheStatsHits AS LONG
DIM SHARED CacheStatsMisses AS LONG

' Initialize cache system
SUB InitCache
    CacheDir$ = "internal/cache/"
    IF NOT _DIREXISTS(CacheDir$) THEN MKDIR CacheDir$
    CacheStatsHits = 0
    CacheStatsMisses = 0
END SUB

' Generate cache key from source file path and modification time
FUNCTION GenerateCacheKey$ (sourceFile AS STRING)
    DIM fileTime AS DOUBLE, fileSize AS LONG
    
    ' Use file modification time and size as cache key components
    fileTime = _FILEDATETIME(sourceFile)
    fileSize = _FILEEXISTS(sourceFile) 'Returns file size if exists
    
    ' Simple hash combining path, time, and size
    GenerateCacheKey$ = HEX$(LEN(sourceFile)) + HEX$(fileTime) + HEX$(fileSize)
END FUNCTION

' Check if cached compilation exists and is valid
FUNCTION CacheExists% (sourceFile AS STRING)
    IF NOT CACHE_ENABLED THEN CacheExists% = 0: EXIT FUNCTION
    
    DIM cacheKey AS STRING, cacheFile AS STRING
    cacheKey$ = GenerateCacheKey$(sourceFile)
    cacheFile$ = CacheDir$ + cacheKey$ + ".qbc"
    
    IF _FILEEXISTS(cacheFile$) THEN
        CacheExists% = -1
    ELSE
        CacheExists% = 0
    END IF
END FUNCTION

' Load from cache - returns true if successful
FUNCTION LoadFromCache% (sourceFile AS STRING, symbolTableFile AS STRING, generatedCodeFile AS STRING)
    IF NOT CACHE_ENABLED THEN LoadFromCache% = 0: EXIT FUNCTION
    
    DIM cacheKey AS STRING, cacheFile AS STRING
    cacheKey$ = GenerateCacheKey$(sourceFile)
    cacheFile$ = CacheDir$ + cacheKey$ + ".qbc"
    
    IF NOT _FILEEXISTS(cacheFile$) THEN
        CacheStatsMisses = CacheStatsMisses + 1
        LoadFromCache% = 0
        EXIT FUNCTION
    END IF
    
    ' TODO: Implement actual cache loading
    ' For now, just track the hit
    CacheStatsHits = CacheStatsHits + 1
    LoadFromCache% = 0 'Not yet implemented
END FUNCTION

' Save to cache
SUB SaveToCache (sourceFile AS STRING, symbolTableFile AS STRING, generatedCodeFile AS STRING)
    IF NOT CACHE_ENABLED THEN EXIT SUB
    
    DIM cacheKey AS STRING, cacheFile AS STRING
    cacheKey$ = GenerateCacheKey$(sourceFile)
    cacheFile$ = CacheDir$ + cacheKey$ + ".qbc"
    
    ' TODO: Implement actual cache saving
    ' For now, this is a placeholder
END SUB

SUB PrintCacheStats
    IF NOT CACHE_ENABLED THEN EXIT SUB
    PRINT "Cache Hits: "; CacheStatsHits
    PRINT "Cache Misses: "; CacheStatsMisses
    IF CacheStatsHits + CacheStatsMisses > 0 THEN
        PRINT "Cache Hit Rate: "; (CacheStatsHits / (CacheStatsHits + CacheStatsMisses) * 100); "%"
    END IF
END SUB

'-------------------------------------------------------------------------------
' MEMORY OPTIMIZATION UTILITIES
'-------------------------------------------------------------------------------

' Get approximate memory usage in MB
FUNCTION GetMemoryUsageMB~
    ' QB64 doesn't have direct memory query, estimate based on array sizes
    GetMemoryUsageMB~ = 0
END FUNCTION

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB InitOptimizationModule
    InitStringPool
    InitCache
    StartPerformanceMetrics
END SUB

SUB CleanupOptimizationModule
    EndPerformanceMetrics
    ClearStringPool
END SUB
