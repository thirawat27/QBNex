//! The keyword table: language keywords (hand-written, with a short
//! description) merged with the built-in SUB/FUNCTION registry generated from
//! the compiler (`builtins_data.rs`).

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::builtins_data::REGISTRY;

/// What a keyword is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeywordKind {
    /// Statement or clause word (`IF`, `FOR`, `AS`, `SHARED`, ...).
    Statement,
    /// Built-in statement-style procedure (`CLS`, `LOCATE`, `_DELAY`, ...).
    BuiltinSub,
    /// Built-in function (`LEFT$`, `ABS`, `_RGB32`, ...).
    BuiltinFunction,
    /// Operator word (`AND`, `MOD`, ...).
    Operator,
    /// Type name (`INTEGER`, `_UNSIGNED`, ...).
    Type,
}

/// One keyword table entry.
#[derive(Clone, Debug)]
pub struct KeywordInfo {
    /// Canonical upper-case spelling, suffix included (`LEFT$`).
    pub name: String,
    pub kind: KeywordKind,
    /// Syntax line, when known.
    pub syntax: String,
    /// Short description, when known.
    pub doc: String,
}

/// (name, kind, syntax, description)
static LANGUAGE: &[(&str, KeywordKind, &str, &str)] = {
    use KeywordKind::{BuiltinFunction as F, BuiltinSub as B, Operator as O, Statement as S, Type as T};
    &[
        ("ACCESS", S, "OPEN file$ FOR mode ACCESS {READ|WRITE|READ WRITE} AS #n", "Access clause of OPEN."),
        ("ALIAS", S, "DECLARE SUB name ALIAS \"external\" (...)", "Names the external symbol of a DECLARE."),
        ("AND", O, "a AND b", "Bitwise/logical AND."),
        ("ANY", T, "DECLARE SUB s (x AS ANY)", "Parameter of any type (DECLARE only)."),
        ("APPEND", S, "OPEN file$ FOR APPEND AS #n", "Opens a sequential file for appending."),
        ("AS", S, "DIM name AS type", "Introduces a type or a file number."),
        ("BASE", S, "OPTION BASE {0|1}", "Lower bound of arrays without explicit bounds."),
        ("BINARY", S, "OPEN file$ FOR BINARY AS #n", "Opens a file for byte access."),
        ("BYVAL", S, "SUB s (BYVAL x AS LONG)", "Passes a parameter by value (DECLARE LIBRARY)."),
        ("CALL", S, "CALL name[(arguments)]", "Calls a SUB."),
        ("CASE", S, "CASE expression[, ...] | CASE IS op value | CASE a TO b | CASE ELSE", "Branch of a SELECT CASE block."),
        ("CATCH", S, "TRY ... CATCH [e] ... END TRY", "Error handler part of a TRY block."),
        ("CLASS", S, "CLASS name ... END CLASS", "Class definition (QBNex)."),
        ("COMMON", S, "COMMON [SHARED] variable[, ...]", "Shares variables between chained programs."),
        ("CONST", S, "CONST name = expression[, ...]", "Defines a named constant."),
        ("CUSTOMTYPE", S, "DECLARE CUSTOMTYPE LIBRARY [\"header\"]", "Library declaration with custom types."),
        ("DATA", S, "DATA constant[, ...]", "Values read by READ."),
        ("DECLARE", S, "DECLARE {SUB|FUNCTION} name [(parameters)] | DECLARE [DYNAMIC|CUSTOMTYPE|STATIC] LIBRARY", "Declares a procedure or opens a library declaration block."),
        ("DEFDBL", S, "DEFDBL letter-range", "Default type DOUBLE for names starting with the letters."),
        ("DEFINT", S, "DEFINT letter-range", "Default type INTEGER for names starting with the letters."),
        ("DEFLNG", S, "DEFLNG letter-range", "Default type LONG for names starting with the letters."),
        ("DEFSNG", S, "DEFSNG letter-range", "Default type SINGLE for names starting with the letters."),
        ("DEFSTR", S, "DEFSTR letter-range", "Default type STRING for names starting with the letters."),
        ("DIM", S, "DIM [SHARED] name[(bounds)] [AS type][, ...]", "Declares variables and arrays."),
        ("DO", S, "DO [{WHILE|UNTIL} condition] ... LOOP [{WHILE|UNTIL} condition]", "Loop block."),
        ("DOUBLE", T, "AS DOUBLE", "64-bit floating point type (suffix #)."),
        ("DYNAMIC", S, "DECLARE DYNAMIC LIBRARY \"name\"", "Library loaded at run time."),
        ("ELSE", S, "IF condition THEN ... ELSE ... END IF", "Alternative branch of IF."),
        ("ELSEIF", S, "ELSEIF condition THEN", "Further condition of a block IF."),
        ("ENDIF", S, "ENDIF", "Same as END IF."),
        ("ENUM", S, "ENUM name ... END ENUM", "Enumeration (QBNex)."),
        ("EQV", O, "a EQV b", "Bitwise equivalence."),
        ("ERASE", S, "ERASE array[, ...]", "Clears static arrays or frees dynamic arrays."),
        ("EXIT", S, "EXIT {DO|FOR|WHILE|SELECT|SUB|FUNCTION|DEF}", "Leaves the enclosing block or procedure."),
        ("EXPLICIT", S, "OPTION _EXPLICIT", "Requires variables to be declared."),
        ("EXTENDS", S, "CLASS name EXTENDS base", "Base class (QBNex)."),
        ("FINALLY", S, "TRY ... FINALLY ... END TRY", "Always-run part of a TRY block."),
        ("FN", S, "DEF FNname[(parameters)] = expression", "User function defined with DEF."),
        ("FOR", S, "FOR counter = start TO end [STEP increment] ... NEXT [counter]", "Counting loop; also the mode clause of OPEN."),
        ("FUNCTION", S, "FUNCTION name[(parameters)] [STATIC] ... END FUNCTION", "Defines a function procedure."),
        ("GOSUB", S, "GOSUB {label|line-number}", "Calls a subroutine; RETURN comes back."),
        ("GOTO", S, "GOTO {label|line-number}", "Jumps to a label or line number."),
        ("IF", S, "IF condition THEN statements [ELSE statements] | IF condition THEN ... END IF", "Conditional statement or block."),
        ("IMP", O, "a IMP b", "Bitwise implication."),
        ("IMPLEMENTS", S, "CLASS name IMPLEMENTS interface", "Interface list (QBNex)."),
        ("IMPORT", S, "IMPORT module", "Imports a QBNex module."),
        ("INTEGER", T, "AS INTEGER", "16-bit signed integer type (suffix %)."),
        ("IS", S, "CASE IS op value", "Comparison form of CASE."),
        ("LET", S, "[LET] variable = expression", "Assignment."),
        ("LIBRARY", S, "DECLARE LIBRARY [\"name\"] ... END DECLARE", "Declares external library procedures."),
        ("LONG", T, "AS LONG", "32-bit signed integer type (suffix &)."),
        ("LOOP", S, "LOOP [{WHILE|UNTIL} condition]", "End of a DO loop."),
        ("MOD", O, "a MOD b", "Integer remainder."),
        ("NEXT", S, "NEXT [counter[, ...]]", "End of a FOR loop."),
        ("NOT", O, "NOT a", "Bitwise/logical NOT."),
        ("OFF", S, "event OFF", "Disables event trapping."),
        ("ON", S, "ON ERROR GOTO label | ON expression GOTO list | event ON", "Error/event trapping, computed jumps."),
        ("OR", O, "a OR b", "Bitwise/logical OR."),
        ("OUTPUT", S, "OPEN file$ FOR OUTPUT AS #n", "Opens a sequential file for writing."),
        ("PRESERVE", S, "REDIM _PRESERVE array(bounds)", "Keeps array contents on REDIM."),
        ("RANDOM", S, "OPEN file$ FOR RANDOM AS #n [LEN = reclen]", "Opens a file for record access."),
        ("REDIM", S, "REDIM [_PRESERVE] [SHARED] array(bounds) [AS type]", "Declares or resizes a dynamic array."),
        ("REM", S, "REM comment", "Comment until the end of the line."),
        ("RESTORE", S, "RESTORE [label|line-number]", "Sets where the next READ starts."),
        ("RESUME", S, "RESUME [NEXT|label|line-number]", "Returns from an error handler."),
        ("RETURN", S, "RETURN [label|line-number]", "Returns from a GOSUB."),
        ("SEG", S, "DEF SEG [= segment]", "Sets the segment for PEEK/POKE."),
        ("SELECT", S, "SELECT CASE expression ... END SELECT", "Multi-way branch block."),
        ("SHARED", S, "DIM SHARED name | SHARED name", "Makes variables visible in procedures."),
        ("SINGLE", T, "AS SINGLE", "32-bit floating point type (suffix !)."),
        ("STATIC", S, "STATIC variable | SUB name STATIC", "Keeps local values between calls."),
        ("STEP", S, "FOR ... STEP increment | PSET STEP (x, y)", "Loop increment or relative coordinates."),
        ("STOP", S, "STOP", "Halts the program."),
        ("STRING", T, "AS STRING [* length]", "String type (suffix $)."),
        ("SUB", S, "SUB name[(parameters)] [STATIC] ... END SUB", "Defines a SUB procedure."),
        ("THEN", S, "IF condition THEN", "Ends the condition of IF/ELSEIF."),
        ("TO", S, "FOR i = a TO b | CASE a TO b | DIM a(lo TO hi)", "Range separator."),
        ("TRY", S, "TRY ... CATCH [e] ... [FINALLY ...] END TRY", "Structured error handling (QBNex)."),
        ("TYPE", S, "TYPE name ... END TYPE", "Defines a user type."),
        ("UNTIL", S, "DO UNTIL condition | LOOP UNTIL condition", "Loop while the condition is false."),
        ("USING", S, "PRINT USING format$; expressions", "Formatted output."),
        ("WEND", S, "WEND", "End of a WHILE loop."),
        ("WHILE", S, "WHILE condition ... WEND | DO WHILE condition", "Loop while the condition is true."),
        ("XOR", O, "a XOR b", "Bitwise exclusive OR."),
        ("_ANDALSO", O, "a _ANDALSO b", "Short-circuit AND."),
        ("_BIT", T, "AS _BIT [* n]", "Bit type (suffix `)."),
        ("_BYTE", T, "AS _BYTE", "8-bit signed integer type (suffix %%)."),
        ("_CONTINUE", S, "_CONTINUE", "Skips to the next iteration of the loop."),
        ("_EXPLICIT", S, "OPTION _EXPLICIT", "Requires variables to be declared."),
        ("_FLOAT", T, "AS _FLOAT", "80-bit floating point type (suffix ##)."),
        ("_INTEGER64", T, "AS _INTEGER64", "64-bit signed integer type (suffix &&)."),
        ("_MEM", T, "AS _MEM", "Memory block descriptor type."),
        ("_OFFSET", T, "AS _OFFSET", "Pointer-sized integer type (suffix %&)."),
        ("_ORELSE", O, "a _ORELSE b", "Short-circuit OR."),
        ("_PRESERVE", S, "REDIM _PRESERVE array(bounds)", "Keeps array contents on REDIM."),
        ("_UNSIGNED", T, "AS _UNSIGNED {INTEGER|LONG|_BYTE|_INTEGER64|_OFFSET|_BIT}", "Unsigned variant of an integer type (suffix ~)."),
        // ---- QBNex language extensions ----
        ("EACH", S, "FOR EACH item IN array() ... NEXT", "Loop over every element of an array or collection."),
        ("IN", S, "FOR EACH item IN array()", "Collection of a FOR EACH loop."),
        ("OPTIONAL", S, "SUB s (OPTIONAL x AS LONG = 0)", "Parameter that may be left out (with a default value)."),
        ("_IIF", F, "_IIF(condition, valueIfTrue, valueIfFalse)", "Inline IF: evaluates only the chosen value."),
        ("_THROW", B, "_THROW code%[, message$]", "Raises an error that TRY ... CATCH or ON ERROR can handle."),
        ("_ULEN", F, "_ULEN(text$)", "Length of a UTF-8 string in characters."),
        ("_UMID$", F, "_UMID$(text$, start&[, count&])", "Characters of a UTF-8 string (by character, not byte)."),
        ("_UCHR$", F, "_UCHR$(codePoint&)", "UTF-8 encoding of a Unicode code point."),
        ("_UASC", F, "_UASC(text$[, position&])", "Unicode code point of a character of a UTF-8 string."),
        ("_SPLIT", F, "count& = _SPLIT(text$, separator$, parts$())", "Splits text into an array; returns the number of parts."),
        ("_JOIN$", F, "_JOIN$(parts$(), separator$)", "Joins the elements of a string array."),
        ("_REPLACE$", F, "_REPLACE$(text$, find$, replacement$)", "Replaces every occurrence of find$."),
        ("_STARTSWITH", F, "_STARTSWITH(text$, prefix$)", "-1 when text$ starts with prefix$."),
        ("_ENDSWITH", F, "_ENDSWITH(text$, suffix$)", "-1 when text$ ends with suffix$."),
        ("_FORMAT$", F, "_FORMAT$(value, format$)", "Formats a number or date with a .NET-style format string."),
        ("_LIST", T, "DIM l AS _LIST", "Built-in list collection (FOR EACH works on it)."),
        ("_DICT", T, "DIM d AS _DICT", "Built-in dictionary (hash map) collection."),
        ("_REGEX", F, "_REGEX(pattern$, text$)", "Regular-expression match (pure Rust engine)."),
        ("_RANDOMSEED", B, "_RANDOMSEED [seed&&]", "Seeds the better random generator (no seed: from the system)."),
        // ---- 3D API (docs/dev/3d-api.md) ----
        ("_LOADMODEL", F, "model& = _LOADMODEL(file$)", "Loads a glTF 2.0 model (.gltf/.glb); -1 when it cannot be loaded."),
        ("_MESH", F, "model& = _MESH(px!(), py!(), pz!(), nx!(), ny!(), nz!(), u!(), v!(), idx&())", "Model from vertex arrays (normals may be empty: computed)."),
        ("_MESHCUBE", F, "model& = _MESHCUBE(size!)", "Cube primitive."),
        ("_MESHPLANE", F, "model& = _MESHPLANE(width!, depth!)", "Flat plane on the XZ axes."),
        ("_MESHCOLOR", B, "_MESHCOLOR model&, colour~&", "Base colour of the whole model."),
        ("_MESHTEXTURE", B, "_MESHTEXTURE model&, image&", "Texture from a 32-bit image handle."),
        ("_FREEMESH", B, "_FREEMESH model&", "Frees a model."),
        ("_DRAW3D", B, "_DRAW3D model& [, x!, y!, z!] [, rx!, ry!, rz!] [, sx!, sy!, sz!]", "Queues a model to draw at _DISPLAY (angles in degrees, order Y-X-Z)."),
        ("_CAMERA", B, "_CAMERA x!, y!, z! TO tx!, ty!, tz! [, fov! | , _ORTHO height!]", "Camera position and target (perspective or orthographic)."),
        ("_CAMERARANGE", B, "_CAMERARANGE near!, far!", "Near and far clipping distances (default 0.1, 1000)."),
        ("_ORTHO", S, "_CAMERA ... , _ORTHO height!", "Orthographic projection for _CAMERA."),
        ("_AMBIENT", B, "_AMBIENT colour~& [, intensity!]", "Ambient light."),
        ("_LIGHT", F, "light& = _LIGHT({_DIRECTIONAL|_POINT}, x!, y!, z!, colour~&, intensity! [, range!])", "Creates a directional or point light."),
        ("_DIRECTIONAL", S, "_LIGHT(_DIRECTIONAL, dx!, dy!, dz!, colour~&, intensity!)", "Directional light kind."),
        ("_POINT", S, "_LIGHT(_POINT, x!, y!, z!, colour~&, intensity! [, range!])", "Point light kind."),
        ("_LIGHTON", B, "_LIGHTON light&, {0|-1}", "Turns a light off or on (at most 8 lights are on)."),
        ("_FREELIGHT", B, "_FREELIGHT light&", "Frees a light."),
        ("_CLEAR3D", B, "_CLEAR3D [colour~&]", "Clears the colour and depth of the 3D layer."),
        ("_GPU3D", F, "f% = _GPU3D", "-1 when a GPU can draw 3D; without one the 3D statements do nothing."),
    ]
};

static TABLE: LazyLock<HashMap<String, KeywordInfo>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for &(name, kind, syntax, doc) in LANGUAGE {
        map.insert(
            name.to_owned(),
            KeywordInfo { name: name.to_owned(), kind, syntax: syntax.to_owned(), doc: doc.to_owned() },
        );
    }
    for &(name, is_function, must, syntax) in REGISTRY {
        let key = format!("{name}{must}");
        let kind = if is_function { KeywordKind::BuiltinFunction } else { KeywordKind::BuiltinSub };
        let doc = if is_function { "Built-in function." } else { "Built-in statement." };
        map.entry(key.clone())
            .and_modify(|e: &mut KeywordInfo| {
                if e.syntax.is_empty() {
                    e.syntax = syntax.to_owned();
                }
            })
            .or_insert_with(|| KeywordInfo { name: key, kind, syntax: syntax.to_owned(), doc: doc.to_owned() });
    }
    // Words that have both a statement form and a `$` function form.
    for (word, kind, doc) in [
        ("INPUT", KeywordKind::BuiltinSub, "Reads input from the keyboard or a file."),
        ("ERROR", KeywordKind::BuiltinSub, "Raises an error; also ON ERROR."),
        ("END", KeywordKind::Statement, "Ends the program, or closes a block (END IF, END SUB, ...)."),
    ] {
        map.entry(word.to_owned()).or_insert_with(|| KeywordInfo {
            name: word.to_owned(),
            kind,
            syntax: String::new(),
            doc: doc.to_owned(),
        });
    }
    map
});

/// Looks up a word (case-insensitive). `word` includes any type suffix.
pub fn lookup(word: &str) -> Option<&'static KeywordInfo> {
    TABLE.get(&word.to_ascii_uppercase())
}

/// Every keyword, in no particular order.
pub fn all() -> impl Iterator<Item = &'static KeywordInfo> {
    TABLE.values()
}

/// True when `word` (case-insensitive, suffix included) is a keyword.
pub fn is_keyword(word: &str) -> bool {
    lookup(word).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups() {
        assert_eq!(lookup("left$").unwrap().kind, KeywordKind::BuiltinFunction);
        assert!(lookup("left").is_none());
        assert_eq!(lookup("If").unwrap().kind, KeywordKind::Statement);
        assert!(lookup("_rgb32").is_some());
        assert!(lookup("string").is_some());
        assert!(lookup("string$").is_some());
        assert!(lookup("input").is_some());
        assert!(lookup("myvar").is_none());
        assert_eq!(lookup("_iif").unwrap().kind, KeywordKind::BuiltinFunction);
        assert_eq!(lookup("_umid$").unwrap().kind, KeywordKind::BuiltinFunction);
        assert_eq!(lookup("_draw3d").unwrap().kind, KeywordKind::BuiltinSub);
        assert!(lookup("each").is_some() && lookup("_meshcube").is_some());
    }
}
