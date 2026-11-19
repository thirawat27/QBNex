' QIR emitter state (utilities/ir_emit.bas). Module-level because QB64
' needs DIM SHARED before the SUB/FUNCTION area. Spec: docs/dev/qir.md
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
'procedure, CHR$(1) + QIR name + CHR$(2) + library symbol + CHR$(2) + library spec + CHR$(1)
DIM SHARED IrLibSpec AS STRING, IrLibOf AS STRING
DIM SHARED IrPendLL AS LONG 'numeric line label waiting for its LASTLINE
DIM SHARED IrQuiet AS _BYTE 'IrPlace$: return "" instead of failing
'per control level (IF/SELECT/FOR...) state, indexed by controllevel
REDIM SHARED IrCtlId(0 TO 1000) AS LONG, IrCtlPart(0 TO 1000) AS LONG, IrCtlVar(0 TO 1000) AS STRING
REDIM SHARED IrArgs(1 TO 100) AS STRING
DIM SHARED IrStmt AS STRING, IrStmtN AS LONG, IrStmtFirst AS STRING
'RUN label / CLEAR targets: count and the main labels (CHR$(10)-separated) of RUNFROM 1..IrRunN
DIM SHARED IrRunN AS LONG, IrRunTo AS STRING
'COMMON variables in order, each name + CHR$(1) + SHARED flag + CHR$(2): CHAIN passes them on
DIM SHARED IrChainVars AS STRING
'included files named by "incline": name + CHR$(2) + index + CHR$(1) per file (after a leading CHR$(1)), their count,
'and the last one looked up
DIM SHARED IrSrcFiles AS STRING, IrSrcFileN AS LONG, IrSrcLastName AS STRING, IrSrcLastIndex AS LONG
'right side of an assignment as setrefer fixed it (fixoperationorder$ result and layout),
'reused by IrAssign while IrSymState$ is unchanged: fixing the same expression twice per
'assignment was a large part of the compile time
DIM SHARED IrRhsSrc AS STRING, IrRhsFixed AS STRING, IrRhsLayout AS STRING, IrRhsState AS STRING
'the same for the arguments of a SUB call: fixed by the call's checks, reused by IrSubCall
'while IrSymState$ is unchanged (IrArgKeep); IrArgCall tells the calls apart
DIM SHARED IrArgCall AS LONG
'the last expressions a statement's checks fixed (IrFixKeep), for IrFixOf$
DIM SHARED IrFixSrc(1 TO 4) AS STRING, IrFixFixed(1 TO 4) AS STRING, IrFixLayout(1 TO 4) AS STRING, IrFixState(1 TO 4) AS STRING, IrFixNext AS LONG
REDIM SHARED IrArgTag(1 TO 16) AS LONG, IrArgSrc(1 TO 16) AS STRING, IrArgFixed(1 TO 16) AS STRING, IrArgLayout(1 TO 16) AS STRING, IrArgState(1 TO 16) AS STRING
'IrAssign asks IrElemPlace$ to hold back the "aaddr" of the element being assigned
'(IrAaddrLater) and emits the held line (IrAaddrHeld) after the right side
DIM SHARED IrAaddrLater AS LONG, IrAaddrHeld AS STRING
