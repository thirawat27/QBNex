# QBNex Compiler Core Module
# Copyright © 2026 thirawat27
# Version: 1.0.0
# Description: Main compiler entry point and core functionality

'All variables will be of type LONG unless explicitly defined
DEFLNG A-Z

'Dynamic array allocation enabled
'$DYNAMIC

'Console-only mode (no GUI/IDE)
$CONSOLE

$EXEICON:'../assets/QBNex.ico'

$VERSIONINFO:CompanyName=QBNex
$VERSIONINFO:FileDescription=QBNex CLI Compiler
$VERSIONINFO:InternalName=qbnex_compiler
$VERSIONINFO:LegalCopyright=MIT License - Copyright (c) 2026 thirawat27
$VERSIONINFO:LegalTrademarks=QBNex
$VERSIONINFO:OriginalFilename=qb.exe
$VERSIONINFO:ProductName=QBNex
$VERSIONINFO:Comments=QBNex is a modern extended BASIC programming language that retains QB4.5/QBasic compatibility and compiles native binaries for Windows, Linux and macOS. CLI-only version with no IDE component.

'Include core modules
'$INCLUDE:'core/version.bas'
'$INCLUDE:'core/constants.bas'
'$INCLUDE:'core/settings.bas'
'$INCLUDE:'compiler/parser.bas'
'$INCLUDE:'compiler/codegen.bas'
'$INCLUDE:'runtime/libqb_wrapper.bas'

DEFLNG A-Z

DIM SHARED CompilerVersion AS STRING
DIM SHARED CompilerName AS STRING
DIM SHARED BuildYear AS STRING
DIM SHARED Owner AS STRING

CompilerName$ = "QBNex"
CompilerVersion$ = "1.0.0"
BuildYear$ = "2026"
Owner$ = "thirawat27"

'=============================================================================
' Main Entry Point
'=============================================================================
SUB main
    DIM argCount AS INTEGER
    DIM args() AS STRING
    DIM sourceFile AS STRING
    DIM outputFile AS STRING
    DIM compileOnly AS _BYTE
    DIM verbose AS _BYTE
    
    'Parse command line arguments
    argCount = _COMMANDCOUNT
    IF argCount = 0 THEN
        PRINT "QBNex v" + CompilerVersion$ + " - Modern BASIC Compiler"
        PRINT "Copyright © " + BuildYear + " " + Owner$
        PRINT
        PRINT "Usage: qb [options] <source.bas>"
        PRINT
        PRINT "Options:"
        PRINT "  -c          Compile only, do not execute"
        PRINT "  -o <name>   Specify output filename"
        PRINT "  -v          Verbose output"
        PRINT "  --version   Show version information"
        PRINT "  --help      Show this help message"
        PRINT
        EXIT SUB
    END IF
    
    REDIM args(argCount)
    FOR i = 1 TO argCount
        args(i) = _COMMAND$(i)
    NEXT i
    
    'Process arguments
    compileOnly = 0
    verbose = 0
    sourceFile$ = ""
    outputFile$ = ""
    
    FOR i = 1 TO argCount
        SELECT CASE args(i)
            CASE "-c"
                compileOnly = -1
            CASE "-v"
                verbose = -1
            CASE "--version"
                PRINT "QBNex version " + CompilerVersion$
                PRINT "Copyright © " + BuildYear + " " + Owner$
                PRINT "Repository: https://github.com/thirawat27/QBNex"
                EXIT SUB
            CASE "--help"
                'Show help (already shown above)
                PRINT "QBNex v" + CompilerVersion$ + " - Modern BASIC Compiler"
                EXIT SUB
            CASE "-o"
                IF i < argCount THEN
                    i = i + 1
                    outputFile$ = args(i)
                END IF
            CASE ELSE
                'Assume it's the source file
                IF sourceFile$ = "" THEN
                    sourceFile$ = args(i)
                ELSE
                    PRINT "Error: Multiple source files not supported"
                    EXIT SUB
                END IF
        END SELECT
    NEXT i
    
    'Validate source file
    IF sourceFile$ = "" THEN
        PRINT "Error: No source file specified"
        EXIT SUB
    END IF
    
    IF _FILEEXISTS(sourceFile$) = 0 THEN
        PRINT "Error: Source file not found: " + sourceFile$
        EXIT SUB
    END IF
    
    'Begin compilation
    IF verbose THEN
        PRINT "QBNex Compiler v" + CompilerVersion$
        PRINT "Compiling: " + sourceFile$
        PRINT
    END IF
    
    'Call compilation pipeline
    CALL compile_program(sourceFile$, outputFile$, compileOnly, verbose)
    
END SUB

'=============================================================================
' Compilation Pipeline
'=============================================================================
SUB compile_program (src AS STRING, out AS STRING, compileOnly AS _BYTE, verbose AS _BYTE)
    DIM startTime AS SINGLE
    DIM endTime AS SINGLE
    DIM duration AS SINGLE
    
    startTime = TIMER
    
    'Stage 1: Parse source
    IF verbose THEN PRINT "[1/5] Parsing QBasic source..."
    CALL parse_source(src)
    
    'Stage 2: Generate C++ code
    IF verbose THEN PRINT "[2/5] Converting QBasic to C++..."
    CALL generate_cpp_code()
    
    'Stage 3: Compile C++
    IF verbose THEN PRINT "[3/5] Compiling C++ code..."
    CALL compile_cpp()
    
    'Stage 4: Link
    IF verbose THEN PRINT "[4/5] Linking executable..."
    CALL link_executable()
    
    'Stage 5: Finalize
    IF verbose THEN PRINT "[5/5] Finalizing build..."
    CALL finalize_build(out)
    
    endTime = TIMER
    duration = endTime - startTime
    
    PRINT "Compilation completed in " + USING("##.##"; duration) + "s"
    
    'Determine output filename
    IF out$ = "" THEN
        out$ = src$
        out$ = LEFT$(out$, INSTR(out$, ".") - 1)
        IF INSTR(_OS$, "WIN") THEN out$ = out$ + ".exe"
    END IF
    
    PRINT "Output: " + out$
    
    'Run if not compile-only
    IF compileOnly = 0 THEN
        PRINT
        PRINT "Running program..."
        PRINT "─────────────────────────────────────────"
        SHELL out$
    END IF
END SUB

'=============================================================================
' Stub implementations (to be filled with actual compiler logic)
'=============================================================================
SUB parse_source (src AS STRING)
    'TODO: Implement QBasic parser
END SUB

SUB generate_cpp_code
    'TODO: Implement C++ code generation
END SUB

SUB compile_cpp
    'TODO: Implement C++ compilation
END SUB

SUB link_executable
    'TODO: Implement linking
END SUB

SUB finalize_build (out AS STRING)
    'TODO: Implement finalization
END SUB

'Main entry
CALL main
