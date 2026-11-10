' QIR v1 emitter state (utilities/ir_emit.bas). Module-level because QB64
' needs DIM SHARED before the SUB/FUNCTION area. Spec: docs/rust-port/qir-v1.md
DIM SHARED IrEmitMode AS _BYTE
'functions: 1 = main
DIM SHARED IrFnN AS LONG, IrFnCur AS LONG
REDIM SHARED IrFnName(1 TO 16) AS STRING, IrFnRet(1 TO 16) AS STRING
REDIM SHARED IrFnHead(1 TO 16) AS STRING, IrFnBody(1 TO 16) AS STRING
REDIM SHARED IrFnDecls(1 TO 16) AS STRING, IrFnRetVar(1 TO 16) AS STRING
REDIM SHARED IrFnRegs(1 TO 16) AS LONG
'module level text (types, globals, data) and declared names
DIM SHARED IrMod AS STRING, IrModDecls AS STRING, IrUdtDone AS STRING
DIM SHARED IrT AS STRING 'register type of the last value produced by IrEval$
DIM SHARED IrFailMsg AS STRING 'first unsupported construct
DIM SHARED IrUniq AS LONG
DIM SHARED IrFix AS STRING, IrFix2 AS STRING, IrFix3 AS STRING 'fixed expressions captured by hooks
DIM SHARED IrPendLL AS LONG 'numeric line label waiting for its LASTLINE
DIM SHARED IrQuiet AS _BYTE 'IrPlace$: return "" instead of failing
DIM SHARED IrDataN AS LONG
'per control level (IF/SELECT/FOR...) state, indexed by controllevel
REDIM SHARED IrCtlId(0 TO 1000) AS LONG, IrCtlPart(0 TO 1000) AS LONG, IrCtlVar(0 TO 1000) AS STRING
DIM SHARED IrDefSeg AS LONG
REDIM SHARED IrArgs(1 TO 100) AS STRING
DIM SHARED IrStmt AS STRING, IrStmtN AS LONG, IrStmtFirst AS STRING
