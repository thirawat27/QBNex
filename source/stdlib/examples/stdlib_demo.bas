' ============================================================================
' QBNex Standard Library - Complete Demonstration
' ============================================================================
' This program demonstrates all major features of the QBNex Standard Library
' ============================================================================

'$INCLUDE:'../qbnex_stdlib.bas'

' ============================================================================
' Main Program
' ============================================================================

CLS
PRINT "========================================================================"
PRINT "QBNex Standard Library - Complete Demonstration"
PRINT "========================================================================"
PRINT

' Display library information
QBNex_StdLib_PrintInfo
PRINT
PRINT "Press any key to start demonstrations..."
SLEEP
CLS

' ============================================================================
' 1. Collections Demo
' ============================================================================
PRINT "========================================================================"
PRINT "1. COLLECTIONS DEMONSTRATION"
PRINT "========================================================================"
PRINT

PRINT "--- List Demo ---"
DIM myList AS QBNex_List
List_Init myList
List_Add myList, "Apple"
List_Add myList, "Banana"
List_Add myList, "Cherry"
PRINT "List items:"
DIM i AS LONG
FOR i = 0 TO myList.Count - 1
    PRINT "  ["; i; "] "; List_Get(myList, i)
NEXT i
PRINT "List contains 'Banana': "; List_Contains(myList, "Banana")
PRINT

PRINT "--- Dictionary Demo ---"
DIM myDict AS QBNex_Dict
Dict_Init myDict
Dict_Set myDict, "name", "QBNex"
Dict_Set myDict, "version", "1.0.0"
Dict_Set myDict, "type", "BASIC"
PRINT "Dictionary entries:"
PRINT "  name = "; Dict_Get(myDict, "name")
PRINT "  version = "; Dict_Get(myDict, "version")
PRINT "  type = "; Dict_Get(myDict, "type")
PRINT

PRINT "--- Stack Demo ---"
DIM myStack AS QBNex_Stack
Stack_Init myStack
Stack_Push myStack, "First"
Stack_Push myStack, "Second"
Stack_Push myStack, "Third"
PRINT "Stack pop: "; Stack_Pop(myStack)
PRINT "Stack peek: "; Stack_Peek(myStack)
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 2. String Utilities Demo
' ============================================================================
PRINT "========================================================================"
PRINT "2. STRING UTILITIES DEMONSTRATION"
PRINT "========================================================================"
PRINT

PRINT "--- StringBuilder Demo ---"
DIM sb AS QBNex_StringBuilder
SB_Init sb
SB_AppendLine sb, "Line 1"
SB_AppendLine sb, "Line 2"
SB_Append sb, "Line 3"
PRINT "StringBuilder output:"
PRINT SB_ToString(sb)
PRINT

PRINT "--- Encoding Demo ---"
DIM original AS STRING
DIM encoded AS STRING
original = "Hello, World!"
encoded = Base64Encode(original)
PRINT "Original: "; original
PRINT "Base64: "; encoded
PRINT

encoded = UrlEncode("Hello World & Friends")
PRINT "URL Encoded: "; encoded
PRINT

encoded = HtmlEncode("<script>alert('test')</script>")
PRINT "HTML Encoded: "; encoded
PRINT

PRINT "--- Pattern Matching Demo ---"
PRINT "Glob match 'test.txt' with '*.txt': "; GlobMatch("test.txt", "*.txt")
PRINT "Glob match 'test.doc' with '*.txt': "; GlobMatch("test.doc", "*.txt")
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 3. Math Demo
' ============================================================================
PRINT "========================================================================"
PRINT "3. MATH DEMONSTRATION"
PRINT "========================================================================"
PRINT

PRINT "--- Vector Demo ---"
DIM v1 AS QBNex_Vec3, v2 AS QBNex_Vec3, vResult AS QBNex_Vec3
Vec3_Set v1, 1, 2, 3
Vec3_Set v2, 4, 5, 6
Vec3_Add vResult, v1, v2
PRINT "Vector1: ("; v1.X; ","; v1.Y; ","; v1.Z; ")"
PRINT "Vector2: ("; v2.X; ","; v2.Y; ","; v2.Z; ")"
PRINT "Sum: ("; vResult.X; ","; vResult.Y; ","; vResult.Z; ")"
PRINT "Dot product: "; Vec3_Dot(v1, v2)
PRINT "V1 Length: "; Vec3_Length(v1)
PRINT

PRINT "--- Statistics Demo ---"
DIM DATA(1 TO 10) AS DOUBLE
FOR i = 1 TO 10
    DATA(i) = i * 1.5
NEXT i
PRINT "Data: 1.5, 3.0, 4.5, 6.0, 7.5, 9.0, 10.5, 12.0, 13.5, 15.0"
PRINT "Sum: "; Stats_Sum(DATA(), 1, 10)
PRINT "Mean: "; Stats_Mean(DATA(), 1, 10)
PRINT "Min: "; Stats_Min(DATA(), 1, 10)
PRINT "Max: "; Stats_Max(DATA(), 1, 10)
PRINT "StdDev: "; Stats_StdDev(DATA(), 1, 10)
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 4. DateTime Demo
' ============================================================================
PRINT "========================================================================"
PRINT "4. DATETIME DEMONSTRATION"
PRINT "========================================================================"
PRINT

PRINT "--- Current DateTime ---"
DIM dt AS QBNex_DateTime
DT_Now dt
PRINT "Current date/time:"
PRINT "  Year: "; dt.Year
PRINT "  Month: "; dt.Month
PRINT "  Day: "; dt.Day
PRINT "  Hour: "; dt.Hour
PRINT "  Minute: "; dt.Minute
PRINT "  Second: "; dt.Second
PRINT "  Day of Week: "; dt.DOW; " (0=Sun)"
PRINT

PRINT "--- DateTime Formatting ---"
PRINT "Formatted: "; DT_Format(dt, "YYYY-MM-DD HH:MI:SS")
PRINT

PRINT "--- DateTime Arithmetic ---"
DIM dt2 AS QBNex_DateTime
dt2 = dt
DT_AddDays dt2, 7
PRINT "After adding 7 days: "; DT_Format(dt2, "YYYY-MM-DD")
PRINT "Day difference: "; DT_DiffDays(dt, dt2)
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 5. I/O Demo
' ============================================================================
PRINT "========================================================================"
PRINT "5. I/O DEMONSTRATION"
PRINT "========================================================================"
PRINT

PRINT "--- Path Manipulation ---"
DIM path1 AS STRING, path2 AS STRING
path1 = "C:\Users\Documents"
path2 = "file.txt"
PRINT "Path 1: "; path1
PRINT "Path 2: "; path2
PRINT "Joined: "; Path_Join(path1, path2)
PRINT "Filename: "; Path_Filename(Path_Join(path1, path2))
PRINT "Extension: "; Path_Extension(Path_Join(path1, path2))
PRINT "Basename: "; Path_Basename(Path_Join(path1, path2))
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 6. System Integration Demo
' ============================================================================
PRINT "========================================================================"
PRINT "6. SYSTEM INTEGRATION DEMONSTRATION"
PRINT "========================================================================"
PRINT

PRINT "--- Environment ---"
PRINT "Platform: "; Env_Platform
PRINT "Is Windows: "; Env_IsWindows
PRINT "Is 64-bit: "; Env_Is64Bit
PRINT "Home directory: "; Env_GetHome
PRINT "Temp directory: "; Env_GetTemp
PRINT

PRINT "--- Optional Demo ---"
DIM opt AS QBNex_Optional
Opt_SetSome opt, "Hello"
PRINT "Optional has value: "; Opt_IsSome(opt)
PRINT "Optional value: "; Opt_Get(opt)
Opt_SetNone opt
PRINT "After SetNone, has value: "; Opt_IsSome(opt)
PRINT "Get or default: "; Opt_GetOrDefault(opt, "Default Value")
PRINT

PRINT "--- Pair Demo ---"
DIM pair AS QBNex_Pair
Pair_Set pair, "key", "value"
PRINT "Pair first: "; Pair_First(pair)
PRINT "Pair second: "; Pair_Second(pair)
PRINT

' ============================================================================
' Cleanup
' ============================================================================
List_Free myList
Dict_Free myDict
Stack_Free myStack
SB_Free sb

PRINT "========================================================================"
PRINT "Demonstration Complete!"
PRINT "========================================================================"
PRINT
PRINT "The QBNex Standard Library provides a complete ecosystem for"
PRINT "high-level programming while maintaining QBasic compatibility."
PRINT
PRINT "Press any key to exit..."
SLEEP
