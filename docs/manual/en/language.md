# Language extensions

[Manual index](../README.md) · [ภาษาไทย](../th/language.md)

QBNex accepts QBasic / QuickBASIC 4.5 programs and the QB64 dialect
(`_` keywords, `$` metacommands, `_UNSIGNED`, `_INTEGER64`, `_FLOAT`,
`_MEM`, `DECLARE LIBRARY`, ...). This page lists what QBNex adds. Each item
has a status (see the [labels](../README.md#status-labels--ป้ายสถานะ)).

Checked against the current compiler: a feature marked Available compiles
today; Planned ones are rejected by the compiler even if the editor already
highlights them.

## Available

### Source files in CP437 or UTF-8

Files saved by QBasic (CP437 bytes, box drawing in strings) compile as they
are; the bytes are kept. UTF-8 files (with or without BOM) work too.

### Standard library modules: `'$IMPORT:'name'`

```basic
'$IMPORT:'json'
'$IMPORT:'ext.http'
```

The line may appear anywhere in the main file. Each module's declarations
are placed before the program and its procedures after it, so a module's
`DIM SHARED`, TYPEs and SUBs work wherever the import line is. See
[Standard library](stdlib.md).

### `_CONTINUE`

Skips to the next iteration of the innermost `FOR`, `DO` or `WHILE` loop.

```basic
FOR i = 1 TO 5
    IF i MOD 2 = 0 THEN _CONTINUE
    PRINT i
NEXT
```

### CLASS (text lowering, with `'$IMPORT:'qbnex'`)

```basic
'$IMPORT:'qbnex'
CLASS Dog
    Name AS STRING * 32
    CONSTRUCTOR (petName AS STRING)
        ME.Name = petName
    END CONSTRUCTOR
END CLASS
```

`CLASS ... END CLASS` with fields, `CONSTRUCTOR`, `METHOD`/`SUB`/`FUNCTION`
members, `EXTENDS` and `IMPLEMENTS` is rewritten into TYPEs and procedures
before compiling. A real CLASS implementation in QIR (virtual dispatch,
`NEW`/`DELETE`, properties) is Planned.

### `DECLARE LIBRARY`, `_FLOAT`, `_MEM`

As in QB64. In the QIR back end external functions are called through
`crates/qb-ffi` (Windows x64, Linux x86-64/AArch64 calling conventions) and
`_FLOAT` is an exact software 80-bit float.

## Planned (port-plan section 9.4)

These are designed and the editor tools (formatter, language server,
highlighting) already know the words. The compiler rejects them today.
All new words start with `_` or appear only in new statement forms, so old
programs keep compiling.

### Control flow

```basic
x = _IIF(score >= 50, "pass", "fail")      ' only the chosen value is evaluated
IF a <> 0 _ANDALSO b / a > 2 THEN ...       ' short-circuit AND / OR (_ORELSE)

FOR EACH item IN names$()
    PRINT item
NEXT
```

`EXIT FOR`/`_CONTINUE` naming the loop to leave or continue.

### Errors: TRY / CATCH / FINALLY

```basic
TRY
    OPEN "data.txt" FOR INPUT AS #1
    LINE INPUT #1, first$
CATCH e
    PRINT "error"; e; _ERRORMESSAGE$
FINALLY
    CLOSE
END TRY

_THROW 5, "bad argument"
```

Built on the existing `ON ERROR` mechanism; `ON ERROR GOTO` keeps working.
(`_ERRORMESSAGE$` itself is already available.)

### Types

```basic
ENUM Direction
    North
    East
    South
    West
END ENUM

DIM primes() AS LONG = {2, 3, 5, 7}
```

Also: variable-length strings and arrays inside TYPE, CONST of every type.

### Procedures

`OPTIONAL` parameters with defaults (`SUB box (w AS LONG, OPTIONAL h AS LONG = 10)`),
opt-in overloading by type, function pointers (`@SubName`, `CALL ptr(...)`),
deep recursion without stack overflow (heap frames in the VM).

### Strings

```basic
name$ = "Ann"
PRINT $"Hello {name$}, you are {age%} years old"   ' interpolation

n& = _SPLIT("a,b,c", ",", parts$())
PRINT _JOIN$(parts$(), " | ")
PRINT _REPLACE$("a-b-c", "-", "+"), _STARTSWITH("QBNex", "QB"), _ENDSWITH(f$, ".bas")
PRINT _FORMAT$(1234.5, "#,##0.00")
PRINT _ULEN(s$), _UMID$(s$, 2, 3), _UCHR$(3647), _UASC("ก")      ' UTF-8 by character
```

Until these land, the [`ext.utf8`](stdlib.md#extutf8) and
[`ext.regex`](stdlib.md#extregex) modules cover the same needs.

### Built-in collections

`DIM l AS _LIST`, `DIM d AS _DICT` with `FOR EACH`. Today:
[`ext.collections`](stdlib.md#extcollections) (handles) or the `collections.*`
modules.

### Library

`_REGEX`, `_RANDOMSEED`, JSON, HTTP(S), hashes, zip, UUID, ISO dates as
built-ins. Today all of these except UUID are available as `ext.*` modules.

### Other syntax listed in the old README

`x += 1` (also `-=`, `*=`, `/=`), `# comment` lines, `IMPORT module` without
the `'$IMPORT:` form, and `FUNCTION name (...) AS STRING` were described in
earlier READMEs; the current compiler rejects them, so treat them as
Planned too.
