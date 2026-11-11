' QIR v1 emitter state (utilities/ir_emit.bas). Module-level because QB64
' needs DIM SHARED before the SUB/FUNCTION area. Spec: docs/rust-port/qir-v1.md
DIM SHARED IrEmitMode AS _BYTE
IrEmitMode = -1 'QIR is the only backend (cutover): --ir is accepted and changes nothing
'--emit-ir or -o <name>.qir: write the QIR text (for qb-web) instead of an executable
DIM SHARED IrEmitText AS _BYTE
'functions: 1 = main
DIM SHARED IrFnN AS LONG, IrFnCur AS LONG
REDIM SHARED IrFnName(1 TO 16) AS STRING, IrFnRet(1 TO 16) AS STRING
REDIM SHARED IrFnHead(1 TO 16) AS STRING, IrFnBody(1 TO 16) AS STRING
REDIM SHARED IrFnDecls(1 TO 16) AS STRING, IrFnRetVar(1 TO 16) AS STRING
REDIM SHARED IrFnRegs(1 TO 16) AS LONG
'per function: code run when it starts (DIM of its static arrays, hoisted)
REDIM SHARED IrFnPro(1 TO 16) AS STRING
'lines IrE has not yet appended to IrFnBody(IrPendFn): appending every line to
'the whole body copied it each time (quadratic for long functions); IrFlush
DIM SHARED IrPendBody AS STRING, IrPendFn AS LONG
'module level text (types, globals, data) and declared names
DIM SHARED IrMod AS STRING, IrModDecls AS STRING, IrUdtDone AS STRING
DIM SHARED IrT AS STRING 'register type of the last value produced by IrEval$
DIM SHARED IrFailMsg AS STRING 'first unsupported construct
DIM SHARED IrUniq AS LONG
DIM SHARED IrFix AS STRING, IrFix2 AS STRING, IrFix3 AS STRING 'fixed expressions captured by hooks
DIM SHARED IrPendCase$, IrPendEvery AS LONG, IrPendElse AS LONG, IrPendCaseOn AS LONG, IrPendLevel AS LONG 'CASE waiting for finishedline
DIM SHARED IrExtMeta AS STRING 'meta extfn lines (DECLARE LIBRARY)
'DECLARE LIBRARY: spec of the library being declared ("" = none) and, per declared
'procedure, CHR$(1) + QIR name + CHR$(2) + C symbol + CHR$(2) + library spec + CHR$(1)
DIM SHARED IrLibSpec AS STRING, IrLibOf AS STRING
DIM SHARED IrPendLL AS LONG 'numeric line label waiting for its LASTLINE
DIM SHARED IrQuiet AS _BYTE 'IrPlace$: return "" instead of failing
DIM SHARED IrDataN AS LONG
'per control level (IF/SELECT/FOR...) state, indexed by controllevel
REDIM SHARED IrCtlId(0 TO 1000) AS LONG, IrCtlPart(0 TO 1000) AS LONG, IrCtlVar(0 TO 1000) AS STRING
DIM SHARED IrDefSeg AS LONG
REDIM SHARED IrArgs(1 TO 100) AS STRING
DIM SHARED IrStmt AS STRING, IrStmtN AS LONG, IrStmtFirst AS STRING
'RUN label / CLEAR targets: count and the main labels (CHR$(10)-separated) of RUNFROM 1..IrRunN
DIM SHARED IrRunN AS LONG, IrRunTo AS STRING
