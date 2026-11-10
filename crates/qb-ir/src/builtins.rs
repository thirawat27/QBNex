//! Runtime builtins callable with `rt` (normative signature table).
//!
//! Every QIR executor implements each entry. Adding entries is a compatible
//! change; renaming or changing a signature is not. Names are QBasic names in
//! upper case; a `.<type>` suffix selects the overload for that operand type.
//!
//! Channel arguments (`ch`) are file numbers; `0` is the screen/console.

use crate::Ty::{self, *};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Builtin {
    pub name: &'static str,
    pub args: &'static [Ty],
    pub ret: Option<Ty>,
}

macro_rules! b {
    ($n:literal, [$($a:ident),*], $r:ident) => {
        Builtin { name: $n, args: &[$($a),*], ret: Some($r) }
    };
    ($n:literal, [$($a:ident),*]) => {
        Builtin { name: $n, args: &[$($a),*], ret: None }
    };
}

pub static TABLE: &[Builtin] = &[
    // ---- console / file output (PRINT, WRITE, PRINT USING)
    b!("PRINTS", [I32, Str]),
    b!("PRINTN.i64", [I32, I64]),
    b!("PRINTN.u64", [I32, U64]),
    b!("PRINTN.f32", [I32, F32]),
    b!("PRINTN.f64", [I32, F64]),
    b!("PRINTN.f80", [I32, F80]),
    b!("PRINTZONE", [I32]),
    b!("PRINTNL", [I32]),
    b!("PRINTTAB", [I32, I64]),
    b!("PRINTSPC", [I32, I64]),
    b!("WRITES", [I32, Str]),
    b!("WRITEN.i64", [I32, I64]),
    b!("WRITEN.u64", [I32, U64]),
    b!("WRITEN.f32", [I32, F32]),
    b!("WRITEN.f64", [I32, F64]),
    b!("WRITEN.f80", [I32, F80]),
    b!("WRITESEP", [I32]),
    b!("USINGBEGIN", [I32, Str]),
    b!("USINGS", [Str]),
    b!("USINGN.i64", [I64]),
    b!("USINGN.u64", [U64]),
    b!("USINGN.f32", [F32]),
    b!("USINGN.f64", [F64]),
    b!("USINGN.f80", [F80]),
    b!("USINGEND", []),
    // ---- number <-> string
    b!("STR$.i64", [I64], Str),
    b!("STR$.u64", [U64], Str),
    b!("STR$.f32", [F32], Str),
    b!("STR$.f64", [F64], Str),
    b!("STR$.f80", [F80], Str),
    b!("VAL", [Str], F64),
    // legacy func_val returns long double
    b!("VAL.f80", [Str], F80),
    b!("HEX$", [I64], Str),
    b!("OCT$", [I64], Str),
    b!("BIN$", [I64], Str),
    b!("MKI$", [I16], Str),
    b!("MKL$", [I32], Str),
    b!("MKS$", [F32], Str),
    b!("MKD$", [F64], Str),
    b!("_MK$.i64", [I64], Str),
    b!("CVI", [Str], I16),
    b!("CVL", [Str], I32),
    b!("CVS", [Str], F32),
    b!("CVD", [Str], F64),
    b!("_CV.i64", [Str], I64),
    b!("MKSMBF$", [F32], Str),
    b!("MKDMBF$", [F64], Str),
    b!("CVSMBF", [Str], F32),
    b!("CVDMBF", [Str], F64),
    // ---- strings
    b!("LEN", [Str], I32),
    b!("LEFT$", [Str, I32], Str),
    b!("RIGHT$", [Str, I32], Str),
    b!("MID$", [Str, I32], Str),
    b!("MID$.n", [Str, I32, I32], Str),
    b!("MIDSET", [Str, I32, Str], Str),
    b!("MIDSET.n", [Str, I32, I32, Str], Str),
    b!("CHR$", [I32], Str),
    b!("ASC", [Str], I32),
    b!("ASC.n", [Str, I32], I32),
    b!("INSTR", [Str, Str], I32),
    b!("INSTR.n", [I32, Str, Str], I32),
    b!("_INSTRREV", [Str, Str], I32),
    b!("_INSTRREV.n", [I32, Str, Str], I32),
    b!("UCASE$", [Str], Str),
    b!("LCASE$", [Str], Str),
    b!("LTRIM$", [Str], Str),
    b!("RTRIM$", [Str], Str),
    b!("_TRIM$", [Str], Str),
    b!("SPACE$", [I32], Str),
    b!("STRING$", [I32, I32], Str),
    b!("STRING$.s", [I32, Str], Str),
    b!("LSET", [Str, Str], Str),
    b!("RSET", [Str, Str], Str),
    // ---- math
    b!("ABS.i64", [I64], I64),
    b!("ABS.f32", [F32], F32),
    b!("ABS.f64", [F64], F64),
    b!("ABS.f80", [F80], F80),
    b!("SGN.i64", [I64], I32),
    b!("SGN.f64", [F64], I32),
    b!("SGN.f80", [F80], I32),
    b!("INT.f32", [F32], F32),
    b!("INT.f64", [F64], F64),
    b!("INT.f80", [F80], F80),
    b!("FIX.f32", [F32], F32),
    b!("FIX.f64", [F64], F64),
    b!("FIX.f80", [F80], F80),
    b!("SQR", [F64], F64),
    b!("SIN", [F64], F64),
    b!("COS", [F64], F64),
    b!("TAN", [F64], F64),
    b!("ATN", [F64], F64),
    b!("LOG", [F64], F64),
    b!("EXP", [F64], F64),
    b!("_ROUND.f64", [F64], I64),
    b!("RND", [], F32),
    b!("RND.n", [F64], F32),
    b!("RANDOMIZE", [F64]),
    b!("RANDOMIZEUSING", [F64]),
    b!("TIMER", [], F64),
    b!("TIMER.n", [F64], F64),
    // ---- error state
    b!("ERR", [], I32),
    b!("ERL", [], I32),
    b!("_ERRORLINE", [], I32),
    // numeric line label reached (ERL reports the last one before the error)
    b!("LASTLINE", [I32]),
    // ---- files
    b!("OPEN", [Str, I32, I32, I32]),
    b!("OPENOLD", [Str, I32, Str]),
    b!("CLOSE", [I32]),
    b!("CLOSEALL", []),
    b!("FREEFILE", [], I32),
    b!("EOF", [I32], I32),
    b!("LOF", [I32], I64),
    b!("LOC", [I32], I64),
    b!("SEEK", [I32], I64),
    b!("SEEKSET", [I32, I64]),
    b!("GETREC", [I32, I64, Ptr, I32]),
    b!("PUTREC", [I32, I64, Ptr, I32]),
    b!("GETSTR", [I32, I64, I32], Str),
    b!("PUTSTR", [I32, I64, Str]),
    b!("FIELD", [I32, I32, Ptr]),
    b!("INPUT$", [I32, I32], Str),
    b!("LINEINPUT", [I32], Str),
    b!("INPUTITEM.s", [I32], Str),
    b!("INPUTITEM.n", [I32], F64),
    b!("INPUTPROMPT", [Str, I32]),
    b!("KILL", [Str]),
    b!("NAME", [Str, Str]),
    b!("MKDIR", [Str]),
    b!("RMDIR", [Str]),
    b!("CHDIR", [Str]),
    b!("_FILEEXISTS", [Str], I32),
    b!("_DIREXISTS", [Str], I32),
    // ---- console / system
    b!("CLS", []),
    b!("LOCATE", [I32, I32]),
    b!("COLOR", [I32, I32]),
    b!("POS", [], I32),
    b!("CSRLIN", [], I32),
    b!("INKEY$", [], Str),
    b!("SLEEP", [I32]),
    b!("BEEP", []),
    b!("COMMAND$", [], Str),
    b!("ENVIRON$", [Str], Str),
    b!("ENVIRON$.n", [I32], Str),
    b!("ENVIRON", [Str]),
    b!("SHELL", [Str], I32),
    b!("DATE$", [], Str),
    b!("TIME$", [], Str),
    b!("_DEST", [I32]),
    b!("_DEFLATE$", [Str], Str),
    b!("_INFLATE$", [Str], Str),
    b!("_INFLATE$.n", [Str, I64], Str),
    // ---- _MEM (the Ptr arguments are mem_block addresses unless named otherwise)
    b!("_MEMNEW", [Ptr, I64]),
    b!("_MEMAT", [Ptr, I64, I64]),
    // dst, address, size, type value, element size
    b!("_MEMVAR", [Ptr, Ptr, I64, I64, I64]),
    b!("_MEMEXISTS", [Ptr], I32),
    b!("_MEMFREE", [Ptr]),
    // checked address of `bytes` bytes at offset in a block
    b!("_MEMGETP", [Ptr, I64, I64], Ptr),
    // block, offset, bytes, source address, source bytes
    b!("_MEMFILLP", [Ptr, I64, I64, Ptr, I64]),
    b!("_MEMFILLS", [Ptr, I64, I64, Str]),
    b!("_MEMPUTS", [Ptr, I64, Str]),
    b!("_MEMCOPY", [Ptr, I64, I64, Ptr, I64]),
    // array descriptor (aref) -> address of the first element / total bytes
    b!("_ARRADDR", [Ptr], Ptr),
    b!("_ARRBYTES", [Ptr], I64),
    b!("_STRCMP", [Str, Str], I32),
    b!("_STRICMP", [Str, Str], I32),
    b!("_CWD$", [], Str),
    b!("_STARTDIR$", [], Str),
];

pub fn lookup(name: &str) -> Option<&'static Builtin> {
    TABLE.iter().find(|b| b.name == name)
}
