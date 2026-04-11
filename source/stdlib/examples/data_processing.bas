' ============================================================================
' QBNex Standard Library - Data Processing Example
' ============================================================================
' Demonstrates practical data processing using collections and I/O
' ============================================================================

'$INCLUDE:'../qbnex_stdlib.bas'

' ============================================================================
' Data structures
' ============================================================================

TYPE Student
    ID AS LONG
    NAME AS STRING * 50
    Grade AS DOUBLE
    Department AS STRING * 30
END TYPE

' ============================================================================
' Main Program
' ============================================================================

CLS
PRINT "========================================================================"
PRINT "QBNex Standard Library - Data Processing Example"
PRINT "========================================================================"
PRINT

' ============================================================================
' 1. Create sample data using List
' ============================================================================
PRINT "--- Creating Sample Student Data ---"
PRINT

DIM studentNames AS QBNex_List
DIM studentGrades AS QBNex_List
DIM studentDepts AS QBNex_List

List_Init studentNames
List_Init studentGrades
List_Init studentDepts

' Add sample students
List_Add studentNames, "Alice Johnson"
List_Add studentGrades, "95.5"
List_Add studentDepts, "Computer Science"

List_Add studentNames, "Bob Smith"
List_Add studentGrades, "87.3"
List_Add studentDepts, "Mathematics"

List_Add studentNames, "Carol White"
List_Add studentGrades, "92.1"
List_Add studentDepts, "Computer Science"

List_Add studentNames, "David Brown"
List_Add studentGrades, "78.9"
List_Add studentDepts, "Physics"

List_Add studentNames, "Eve Davis"
List_Add studentGrades, "88.7"
List_Add studentDepts, "Mathematics"

PRINT "Created "; studentNames.Count; " student records"
PRINT

' ============================================================================
' 2. Store data in Dictionary for quick lookup
' ============================================================================
PRINT "--- Building Student Database (Dictionary) ---"
PRINT

DIM studentDB AS QBNex_Dict
Dict_Init studentDB

DIM i AS LONG
FOR i = 0 TO studentNames.Count - 1
    DIM KEY AS STRING
    KEY = "student_" + LTRIM$(STR$(i))
    
    ' Store as formatted string
    DIM record AS STRING
    record = List_Get(studentNames, i) + "|" + _
    List_Get(studentGrades, i) + "|" + _
    List_Get(studentDepts, i)
    
    Dict_Set studentDB, KEY, record
NEXT i

PRINT "Database contains "; studentDB.Count; " entries"
PRINT

' ============================================================================
' 3. Calculate statistics
' ============================================================================
PRINT "--- Calculating Grade Statistics ---"
PRINT

DIM grades(1 TO 100) AS DOUBLE
DIM gradeCount AS LONG

gradeCount = 0
FOR i = 0 TO studentGrades.Count - 1
    gradeCount = gradeCount + 1
    grades(gradeCount) = VAL(List_Get(studentGrades, i))
NEXT i

PRINT "Total students: "; gradeCount
PRINT "Average grade: "; INT(Stats_Mean(grades(), 1, gradeCount) * 10) / 10
PRINT "Highest grade: "; Stats_Max(grades(), 1, gradeCount)
PRINT "Lowest grade: "; Stats_Min(grades(), 1, gradeCount)
PRINT "Std deviation: "; INT(Stats_StdDev(grades(), 1, gradeCount) * 10) / 10
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 4. Group by department using multiple dictionaries
' ============================================================================
PRINT "--- Grouping by Department ---"
PRINT

DIM deptCS AS QBNex_List
DIM deptMath AS QBNex_List
DIM deptPhysics AS QBNex_List

List_Init deptCS
List_Init deptMath
List_Init deptPhysics

FOR i = 0 TO studentNames.Count - 1
    DIM dept AS STRING
    dept = List_Get(studentDepts, i)
    
    IF dept = "Computer Science" THEN
        List_Add deptCS, List_Get(studentNames, i)
    ELSEIF dept = "Mathematics" THEN
        List_Add deptMath, List_Get(studentNames, i)
    ELSEIF dept = "Physics" THEN
        List_Add deptPhysics, List_Get(studentNames, i)
    END IF
NEXT i

PRINT "Computer Science ("; deptCS.Count; " students):"
FOR i = 0 TO deptCS.Count - 1
    PRINT "  • "; List_Get(deptCS, i)
NEXT i
PRINT

PRINT "Mathematics ("; deptMath.Count; " students):"
FOR i = 0 TO deptMath.Count - 1
    PRINT "  • "; List_Get(deptMath, i)
NEXT i
PRINT

PRINT "Physics ("; deptPhysics.Count; " students):"
FOR i = 0 TO deptPhysics.Count - 1
    PRINT "  • "; List_Get(deptPhysics, i)
NEXT i
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 5. Generate report using StringBuilder
' ============================================================================
PRINT "--- Generating Report ---"
PRINT

DIM report AS QBNex_StringBuilder
SB_Init report

SB_AppendLine report, "========================================="
SB_AppendLine report, "STUDENT GRADE REPORT"
SB_AppendLine report, "========================================="
SB_AppendLine report, ""

DIM dt AS QBNex_DateTime
DT_Now dt
SB_AppendLine report, "Generated: " + DT_Format(dt, "YYYY-MM-DD HH:MI:SS")
SB_AppendLine report, ""

SB_AppendLine report, "STUDENT LIST:"
SB_AppendLine report, "-----------------------------------------"

FOR i = 0 TO studentNames.Count - 1
    DIM LINE AS STRING
    LINE = RTRIM$(List_Get(studentNames, i))
    
    ' Pad name to 25 characters
    DO WHILE LEN(LINE) < 25
        LINE = LINE + " "
    LOOP
    
    LINE = LINE + " | Grade: " + List_Get(studentGrades, i)
    LINE = LINE + " | " + List_Get(studentDepts, i)
    
    SB_AppendLine report, LINE
NEXT i

SB_AppendLine report, ""
SB_AppendLine report, "STATISTICS:"
SB_AppendLine report, "-----------------------------------------"
SB_AppendLine report, "Average: " + LTRIM$(STR$(INT(Stats_Mean(grades(), 1, gradeCount) * 10) / 10))
SB_AppendLine report, "Highest: " + LTRIM$(STR$(Stats_Max(grades(), 1, gradeCount)))
SB_AppendLine report, "Lowest:  " + LTRIM$(STR$(Stats_Min(grades(), 1, gradeCount)))
SB_AppendLine report, ""
SB_AppendLine report, "========================================="

PRINT SB_ToString(report)
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 6. Export to CSV
' ============================================================================
PRINT "--- Exporting to CSV ---"
PRINT

DIM csvFile AS STRING
csvFile = "students.csv"

DIM writer AS QBNex_CsvWriter
CSV_WriterInit writer, csvFile

' Write header
CSV_AddField writer, "ID"
CSV_AddField writer, "Name"
CSV_AddField writer, "Grade"
CSV_AddField writer, "Department"
CSV_WriteRow writer

' Write data
FOR i = 0 TO studentNames.Count - 1
    CSV_AddField writer, LTRIM$(STR$(i + 1))
    CSV_AddField writer, List_Get(studentNames, i)
    CSV_AddField writer, List_Get(studentGrades, i)
    CSV_AddField writer, List_Get(studentDepts, i)
    CSV_WriteRow writer
NEXT i

PRINT "Data exported to: "; csvFile
PRINT "Total rows: "; studentNames.Count + 1; " (including header)"
PRINT

' ============================================================================
' 7. Find top performers using Set operations
' ============================================================================
PRINT "--- Finding Top Performers (Grade >= 90) ---"
PRINT

DIM topPerformers AS QBNex_Set
Set_Init topPerformers

FOR i = 0 TO studentNames.Count - 1
    IF VAL(List_Get(studentGrades, i)) >= 90 THEN
        Set_Add topPerformers, List_Get(studentNames, i)
    END IF
NEXT i

PRINT "Top performers ("; Set_Count(topPerformers); " students):"
FOR i = 0 TO topPerformers.List.Count - 1
    PRINT "  ★ "; List_Get(topPerformers.List, i)
NEXT i
PRINT

' ============================================================================
' Cleanup
' ============================================================================
List_Free studentNames
List_Free studentGrades
List_Free studentDepts
Dict_Free studentDB
List_Free deptCS
List_Free deptMath
List_Free deptPhysics
SB_Free report
Set_Free topPerformers

PRINT "========================================================================"
PRINT "Data Processing Example Complete!"
PRINT "========================================================================"
PRINT
PRINT "This example demonstrated:"
PRINT "  • Dynamic data storage with Lists"
PRINT "  • Fast lookup with Dictionary"
PRINT "  • Statistical analysis"
PRINT "  • Data grouping and filtering"
PRINT "  • Report generation with StringBuilder"
PRINT "  • CSV export"
PRINT "  • Set operations for data queries"
PRINT
PRINT "Press any key to exit..."
SLEEP
