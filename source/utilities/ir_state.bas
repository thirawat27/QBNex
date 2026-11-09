' R11 spike: shared state of the QIR emitter (utilities/ir_emit.bas).
' Module-level because QB64 needs DIM SHARED before the SUB/FUNCTION area.
' See docs/rust-port/r11-emitter-design.md.
DIM SHARED IrEmitMode AS _BYTE
REDIM SHARED IrSrc(0) AS STRING
DIM SHARED IrSrcN AS LONG
REDIM SHARED IrTokT(0) AS INTEGER 'token kind: 1 identifier, 2 number, 3 string, 4 punctuation
REDIM SHARED IrTokV(0) AS STRING
DIM SHARED IrTokN AS LONG
DIM SHARED IrP AS LONG
DIM SHARED IrOut AS INTEGER
DIM SHARED IrLine AS LONG
DIM SHARED IrScope AS STRING
DIM SHARED IrUniq AS LONG
REDIM SHARED IrDimName(0) AS STRING, IrDimSfx(0) AS STRING, IrDimScope(0) AS STRING
DIM SHARED IrDimN AS LONG
REDIM SHARED IrSubName(0) AS STRING, IrSubParams(0) AS STRING, IrSubArgc(0) AS LONG
DIM SHARED IrSubN AS LONG
REDIM SHARED IrBlkKind(0) AS STRING, IrBlkId(0) AS LONG, IrBlkPart(0) AS LONG, IrBlkVar(0) AS STRING
DIM SHARED IrBlkN AS LONG
