# Language extensions

[Manual index](../README.md) · [ภาษาไทย](../th/language.md)

QBNex accepts QBasic / QuickBASIC 4.5 programs and the QB64 dialect
(`_` keywords, `$` metacommands, `_UNSIGNED`, `_INTEGER64`, `_FLOAT`,
`_MEM`, `DECLARE LIBRARY`, ...). This page lists what QBNex adds. Each item
has a status (see the [labels](../README.md#status-labels--ป้ายสถานะ)).
Existing programs are unaffected: the new words start with `_` or are only
recognised where QBasic would have reported a syntax error.

The examples on this page are compiled and run by `tests/docs/check_docs.ps1`
(the expected output is shown under each one).

## Available

### `_IIF`: inline IF

<!-- docs-test: no-ir _IIF is not in QIR yet -->
```basic
x = 7
PRINT _IIF(x > 5, "big", "small"); _IIF(x < 5, 1, 2)
```

<!-- docs-test: output -->
```text
big 2
```

Only the chosen value is evaluated. Both values are strings, or both are
numbers.

### `_ANDALSO`, `_ORELSE`: short-circuit logic

```basic
DIM a(1 TO 3) AS INTEGER
i = 4
IF i <= 3 _ANDALSO a(i) = 0 THEN PRINT "in range" ELSE PRINT "index"; i; "skipped"
IF i > 3 _ORELSE a(i) = 0 THEN PRINT "no subscript error"
```

<!-- docs-test: output -->
```text
index 4 skipped
no subscript error
```

The right side runs only when needed. The result is -1 or 0.

### String interpolation

<!-- docs-test: no-ir _TOSTR$ (interpolation) is not in QIR yet -->
```basic
name$ = "Ada"
n% = 3
PRINT $"Hello {name$}, {n%} new messages, {{braces}}"
```

<!-- docs-test: output -->
```text
Hello Ada, 3 new messages, {braces}
```

Numbers are written like `LTRIM$(STR$(x))` (the `_TOSTR$` function).
`{{` and `}}` are literal braces. An expression inside `{}` cannot contain
a `"`.

### `FOR EACH`

```basic
DIM scores(1 TO 4) AS INTEGER
FOR i = 1 TO 4: scores(i) = i * 10: NEXT
FOR EACH s IN scores()
    total = total + s
NEXT s
PRINT total
```

<!-- docs-test: output -->
```text
 100
```

One-dimensional arrays, from LBOUND to UBOUND; the loop variable is a copy
of the element.

### `ENUM`

```basic
ENUM Fruit
    Apple
    Banana = 5
    Cherry, Grape
END ENUM
PRINT Apple; Banana; Cherry; Grape
```

<!-- docs-test: output -->
```text
 0  5  6  7
```

Members become integer `CONST`s, counting up from 0 or from the last
`= value`. `ENUM` without a name works too.

### `TRY` / `CATCH` / `FINALLY`

```basic
TRY
    ERROR 53
    PRINT "not reached"
CATCH e
    PRINT "caught error"; e
FINALLY
    PRINT "cleanup"
END TRY
```

<!-- docs-test: output -->
```text
caught error 53
cleanup
```

Built on `ON ERROR`: `CATCH e` stores the error number in `e`; without
`CATCH` the error is raised again after `FINALLY`. TRY works inside SUBs.
Limits: TRY only in the main source file (not in `$INCLUDE` files), each
TRY keyword on its own line. Integer division by zero (`\` and `MOD`) is
error 11 and can be caught. `ON ERROR GOTO` keeps working as before.

### `OPTIONAL` parameters

```basic
DECLARE SUB Greet (who$, OPTIONAL greeting$ = "Hello", OPTIONAL times% = 1)
Greet "Ada"
Greet "Bob", "Hi", 2
PRINT Scale(3); Scale(3, 10)
SYSTEM

SUB Greet (who$, OPTIONAL greeting$ = "Hello", OPTIONAL times% = 1)
    FOR i% = 1 TO times%
        PRINT greeting$; ", "; who$
    NEXT
END SUB

FUNCTION Scale& (v AS LONG, OPTIONAL factor AS LONG = 2)
    Scale& = v * factor
END FUNCTION
```

<!-- docs-test: output -->
```text
Hello, Ada
Hi, Bob
Hi, Bob
 6  30
```

Trailing arguments may be left out; the default is any expression valid
where the procedure is called.

### `_CONTINUE`

```basic
FOR i = 1 TO 5
    IF i MOD 2 = 0 THEN _CONTINUE
    PRINT i;
NEXT
```

<!-- docs-test: output -->
```text
 1  3  5
```

Skips to the next iteration of the innermost `FOR`, `DO` or `WHILE` loop.

### `$ASSERTS` and `_ASSERT`

<!-- docs-test: no-ir _ASSERT is not in QIR yet -->
```basic
$ASSERTS
_ASSERT 1 + 1 = 2, "math works"
PRINT "asserts passed"
```

<!-- docs-test: output -->
```text
asserts passed
```

### Source files in CP437 or UTF-8

Files saved by QBasic (CP437 bytes, box drawing in strings) compile as they
are; the bytes are kept. UTF-8 files (with or without BOM) work too.

### Standard library modules: `'$IMPORT:'name'`

```basic
'$IMPORT:'url'
PRINT UrlEncodeComponent$("a b&c")
```

<!-- docs-test: output -->
```text
a%20b%26c
```

The line may appear anywhere in the main file. Each module's declarations
are placed before the program and its procedures after it, so a module's
`DIM SHARED`, TYPEs and SUBs work wherever the import line is. See
[Standard library](stdlib.md).

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
before compiling. A base class must be declared before a class that
`EXTENDS` it.

### `DECLARE LIBRARY`, `_FLOAT`, `_MEM`

As in QB64. In the QIR back end external functions are called through
`crates/qb-ffi` (Windows x64, Linux x86-64/AArch64 calling conventions) and
`_FLOAT` is an exact software 80-bit float.

## Planned (port-plan section 9.4)

Designed, not in the compiler yet (the editor tools already know some of
the words):

<!-- docs-test: compile-error -->
```basic
_THROW 5, "bad argument"
```

<!-- docs-test: compile-error -->
```basic
DIM primes() AS LONG = {2, 3, 5, 7}
```

<!-- docs-test: compile-error -->
```basic
n& = _SPLIT("a,b,c", ",", parts$())
PRINT _JOIN$(parts$(), " | "), _REPLACE$("a-b", "-", "+"), _ULEN("ก")
```

<!-- docs-test: compile-error -->
```basic
x = 1
x += 2
```

- `_THROW code[, message$]` (today: `ERROR code`).
- Array literals, variable-length strings and arrays inside TYPE.
- String helpers `_SPLIT`, `_JOIN$`, `_REPLACE$`, `_STARTSWITH`,
  `_ENDSWITH`, `_FORMAT$` and UTF-8 helpers `_ULEN`, `_UMID$`, `_UCHR$`,
  `_UASC` (today: the [`ext.utf8`](stdlib.md#extutf8) and
  [`ext.regex`](stdlib.md#extregex) modules).
- Built-in `_LIST` / `_DICT` (today: [`ext.collections`](stdlib.md#extcollections)).
- `_REGEX`, `_RANDOMSEED` and UUIDs as built-ins.
- CLASS in QIR: virtual dispatch, `NEW`/`DELETE`, properties; function
  pointers (`@SubName`); overloading by type.
- `x += 1` (and `-=`, `*=`, `/=`), `# comment` lines, `IMPORT module`
  without the `'$IMPORT:` form and `FUNCTION name (...) AS type`, which
  earlier READMEs described.
