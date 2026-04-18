'===============================================================================
' QBNex LLVM Backend Compatibility Module
'===============================================================================
' Stage0-compatible LLVM helper layer. Keeps experimental LLVM APIs available
' without requiring advanced UDT layouts during bootstrap/self-host builds.
'===============================================================================

CONST LLVM_TYPE_VOID = 1
CONST LLVM_TYPE_INT1 = 2
CONST LLVM_TYPE_INT8 = 3
CONST LLVM_TYPE_INT16 = 4
CONST LLVM_TYPE_INT32 = 5
CONST LLVM_TYPE_INT64 = 6
CONST LLVM_TYPE_FLOAT = 7
CONST LLVM_TYPE_DOUBLE = 8
CONST LLVM_TYPE_PTR = 9
CONST LLVM_TYPE_ARRAY = 10
CONST LLVM_TYPE_STRUCT = 11
CONST LLVM_TYPE_FUNCTION = 12

CONST LLVM_INST_ALLOC = 1
CONST LLVM_INST_LOAD = 2
CONST LLVM_INST_STORE = 3
CONST LLVM_INST_ADD = 4
CONST LLVM_INST_SUB = 5
CONST LLVM_INST_MUL = 6
CONST LLVM_INST_DIV = 7
CONST LLVM_INST_REM = 8
CONST LLVM_INST_AND = 9
CONST LLVM_INST_OR = 10
CONST LLVM_INST_XOR = 11
CONST LLVM_INST_SHL = 12
CONST LLVM_INST_SHR = 13
CONST LLVM_INST_CMP = 14
CONST LLVM_INST_BR = 15
CONST LLVM_INST_CALL = 16
CONST LLVM_INST_RET = 17
CONST LLVM_INST_GEP = 18
CONST LLVM_INST_BITCAST = 19
CONST LLVM_INST_TRUNC = 20
CONST LLVM_INST_ZEXT = 21
CONST LLVM_INST_SEXT = 22
CONST LLVM_INST_FPTOI = 23
CONST LLVM_INST_ITOFP = 24
CONST LLVM_INST_PHI = 25

DIM SHARED LLVMBackendEnabled AS _BYTE
DIM SHARED LLVMOptimizationLevel%
DIM SHARED LLVMTargetArchitecture$
DIM SHARED LLVMTargetOS$
DIM SHARED LLVMEmitDebugInfo AS _BYTE
DIM SHARED LLVMUseVectorization AS _BYTE
DIM SHARED LLVMUseLinkTimeOpt AS _BYTE
DIM SHARED LLVMModuleName$
DIM SHARED LLVMTargetTriple$
DIM SHARED LLVMDataLayout$
DIM SHARED LLVMCurrentFunction$
DIM SHARED LLVMRegisterCounter&
DIM SHARED LLVMLabelCounter&

SUB InitLLVMBakend
    LLVMBackendEnabled = 0
    LLVMOptimizationLevel% = 2
    LLVMTargetArchitecture$ = "x86_64"
    IF INSTR(_OS$, "WIN") THEN
        LLVMTargetOS$ = "windows"
    ELSEIF INSTR(_OS$, "LINUX") THEN
        LLVMTargetOS$ = "linux"
    ELSEIF INSTR(_OS$, "MAC") THEN
        LLVMTargetOS$ = "darwin"
    ELSE
        LLVMTargetOS$ = "unknown"
    END IF
    LLVMEmitDebugInfo = 0
    LLVMUseVectorization = 0
    LLVMUseLinkTimeOpt = 0
    LLVMModuleName$ = "qbnex_output"
    LLVMTargetTriple$ = GetTargetTriple$()
    LLVMDataLayout$ = GetDataLayout$()
    LLVMCurrentFunction$ = ""
    LLVMRegisterCounter& = 1
    LLVMLabelCounter& = 1
END SUB

SUB CleanupLLVMBakend
    LLVMCurrentFunction$ = ""
    LLVMRegisterCounter& = 1
    LLVMLabelCounter& = 1
END SUB

FUNCTION GetTargetTriple$ ()
    DIM triple AS STRING

    triple = LLVMTargetArchitecture$ + "-pc-" + LLVMTargetOS$
    IF LLVMTargetOS$ = "windows" THEN triple = triple + "-msvc"
    GetTargetTriple$ = triple
END FUNCTION

FUNCTION GetDataLayout$ ()
    GetDataLayout$ = "e-m:e-i64:64-f80:128-n8:16:32:64-S128"
END FUNCTION

FUNCTION BasicTypeToLLVM$ (basicType AS STRING)
    DIM upperType AS STRING

    upperType = UCASE$(LTRIM$(RTRIM$(basicType)))
    SELECT CASE upperType
        CASE "_BIT", "BIT"
            BasicTypeToLLVM$ = "i1"
        CASE "_BYTE", "BYTE"
            BasicTypeToLLVM$ = "i8"
        CASE "INTEGER", "_UNSIGNED INTEGER"
            BasicTypeToLLVM$ = "i16"
        CASE "LONG", "_UNSIGNED LONG"
            BasicTypeToLLVM$ = "i32"
        CASE "_INTEGER64", "INTEGER64"
            BasicTypeToLLVM$ = "i64"
        CASE "SINGLE"
            BasicTypeToLLVM$ = "float"
        CASE "DOUBLE"
            BasicTypeToLLVM$ = "double"
        CASE "_FLOAT", "FLOAT"
            BasicTypeToLLVM$ = "fp128"
        CASE "STRING"
            BasicTypeToLLVM$ = "i8*"
        CASE ELSE
            BasicTypeToLLVM$ = "%" + basicType
    END SELECT
END FUNCTION

FUNCTION GetLLVMBasicType% (basicType AS STRING)
    DIM upperType AS STRING

    upperType = UCASE$(LTRIM$(RTRIM$(basicType)))
    SELECT CASE upperType
        CASE "_BIT", "BIT"
            GetLLVMBasicType% = LLVM_TYPE_INT1
        CASE "_BYTE", "BYTE"
            GetLLVMBasicType% = LLVM_TYPE_INT8
        CASE "INTEGER", "_UNSIGNED INTEGER"
            GetLLVMBasicType% = LLVM_TYPE_INT16
        CASE "LONG", "_UNSIGNED LONG"
            GetLLVMBasicType% = LLVM_TYPE_INT32
        CASE "_INTEGER64", "INTEGER64"
            GetLLVMBasicType% = LLVM_TYPE_INT64
        CASE "SINGLE"
            GetLLVMBasicType% = LLVM_TYPE_FLOAT
        CASE "DOUBLE", "_FLOAT", "FLOAT"
            GetLLVMBasicType% = LLVM_TYPE_DOUBLE
        CASE "STRING"
            GetLLVMBasicType% = LLVM_TYPE_PTR
        CASE ELSE
            GetLLVMBasicType% = LLVM_TYPE_STRUCT
    END SELECT
END FUNCTION

FUNCTION NewRegister$ ()
    NewRegister$ = "%" + LTRIM$(STR$(LLVMRegisterCounter&))
    LLVMRegisterCounter& = LLVMRegisterCounter& + 1
END FUNCTION

FUNCTION NewLabel$ ()
    NewLabel$ = "label" + LTRIM$(STR$(LLVMLabelCounter&))
    LLVMLabelCounter& = LLVMLabelCounter& + 1
END FUNCTION

FUNCTION EmitAlloca$ (typeStr AS STRING, alignBytes AS INTEGER)
    DIM reg AS STRING

    reg = NewRegister$()
    EmitAlloca$ = "  " + reg + " = alloca " + typeStr + ", align " + LTRIM$(STR$(alignBytes))
END FUNCTION

FUNCTION EmitLoad$ (typeStr AS STRING, ptrReg AS STRING, alignBytes AS INTEGER)
    DIM reg AS STRING

    reg = NewRegister$()
    EmitLoad$ = "  " + reg + " = load " + typeStr + ", " + typeStr + "* " + ptrReg + ", align " + LTRIM$(STR$(alignBytes))
END FUNCTION

FUNCTION EmitStore$ (typeStr AS STRING, valueReg AS STRING, ptrReg AS STRING, alignBytes AS INTEGER)
    EmitStore$ = "  store " + typeStr + " " + valueReg + ", " + typeStr + "* " + ptrReg + ", align " + LTRIM$(STR$(alignBytes))
END FUNCTION

FUNCTION EmitBinOp$ (op AS STRING, typeStr AS STRING, reg1 AS STRING, reg2 AS STRING)
    DIM reg AS STRING
    DIM llvmOp AS STRING

    reg = NewRegister$()
    llvmOp = LCASE$(LTRIM$(RTRIM$(op)))
    IF llvmOp = "" THEN llvmOp = "add"
    EmitBinOp$ = "  " + reg + " = " + llvmOp + " " + typeStr + " " + reg1 + ", " + reg2
END FUNCTION

FUNCTION EmitCompare$ (cmpOp AS STRING, typeStr AS STRING, reg1 AS STRING, reg2 AS STRING)
    DIM reg AS STRING
    DIM pred AS STRING

    reg = NewRegister$()
    pred = LCASE$(LTRIM$(RTRIM$(cmpOp)))
    IF pred = "" THEN pred = "eq"
    EmitCompare$ = "  " + reg + " = icmp " + pred + " " + typeStr + " " + reg1 + ", " + reg2
END FUNCTION

FUNCTION EmitBranch$ (label AS STRING)
    EmitBranch$ = "  br label %" + label
END FUNCTION

FUNCTION EmitCondBranch$ (condReg AS STRING, trueLabel AS STRING, falseLabel AS STRING)
    EmitCondBranch$ = "  br i1 " + condReg + ", label %" + trueLabel + ", label %" + falseLabel
END FUNCTION

FUNCTION EmitCall$ (retType AS STRING, funcName AS STRING, args AS STRING)
    DIM reg AS STRING

    IF UCASE$(retType) = "VOID" THEN
        EmitCall$ = "  call " + retType + " @" + funcName + "(" + args + ")"
    ELSE
        reg = NewRegister$()
        EmitCall$ = "  " + reg + " = call " + retType + " @" + funcName + "(" + args + ")"
    END IF
END FUNCTION

FUNCTION EmitReturn$ (typeStr AS STRING, valueReg AS STRING)
    IF UCASE$(LTRIM$(RTRIM$(typeStr))) = "VOID" OR LEN(LTRIM$(RTRIM$(valueReg))) = 0 THEN
        EmitReturn$ = "  ret void"
    ELSE
        EmitReturn$ = "  ret " + typeStr + " " + valueReg
    END IF
END FUNCTION

SUB EmitModuleHeader (outputFile AS INTEGER)
    PRINT #outputFile, "; ModuleID = '" + LLVMModuleName$ + "'"
    PRINT #outputFile, "source_filename = """ + LLVMModuleName$ + ".ll"""
    PRINT #outputFile, "target datalayout = """ + LLVMDataLayout$ + """"
    PRINT #outputFile, "target triple = """ + LLVMTargetTriple$ + """"
    PRINT #outputFile,
END SUB

FUNCTION IsLLVMBakendEnabled%
    IsLLVMBakendEnabled% = LLVMBackendEnabled
END FUNCTION

SUB SetLLVMBakendEnabled (enabled AS _BYTE)
    LLVMBackendEnabled = enabled
END SUB

FUNCTION GetLLVMBakendStatus$ ()
    IF LLVMBackendEnabled THEN
        GetLLVMBakendStatus$ = "ENABLED (Compatibility Mode)"
    ELSE
        GetLLVMBakendStatus$ = "DISABLED"
    END IF
END FUNCTION

SUB PrintLLVMEvaluationReport
    PRINT "=== LLVM Backend Evaluation Report ==="
    PRINT "Status: "; GetLLVMBakendStatus$
    PRINT "Target Triple: "; LLVMTargetTriple$
    PRINT "Architecture: "; LLVMTargetArchitecture$
    PRINT "Target OS: "; LLVMTargetOS$
    PRINT "Optimization Level: O"; LLVMOptimizationLevel%
END SUB
