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

<!-- docs-test: output -->
```basic
x = 7
PRINT _IIF(x > 5, "big", "small"); _IIF(x < 5, 1, 2)
```

<!-- expected output -->
```text
big 2
```

Only the chosen value is evaluated. Both values are strings, or both are
numbers.

### `_ANDALSO`, `_ORELSE`: short-circuit logic

<!-- docs-test: output -->
```basic
DIM a(1 TO 3) AS INTEGER
i = 4
IF i <= 3 _ANDALSO a(i) = 0 THEN PRINT "in range" ELSE PRINT "index"; i; "skipped"
IF i > 3 _ORELSE a(i) = 0 THEN PRINT "no subscript error"
```

<!-- expected output -->
```text
index 4 skipped
no subscript error
```

The right side runs only when needed. The result is -1 or 0.

### String interpolation

<!-- docs-test: output -->
```basic
name$ = "Ada"
n% = 3
PRINT $"Hello {name$}, {n%} new messages, {{braces}}"
```

<!-- expected output -->
```text
Hello Ada, 3 new messages, {braces}
```

Numbers are written like `LTRIM$(STR$(x))` (the `_TOSTR$` function).
`{{` and `}}` are literal braces. An expression inside `{}` may contain
string literals: `$"Hi {name$ + "!"}"`.

### `FOR EACH`

<!-- docs-test: output -->
```basic
DIM scores(1 TO 4) AS INTEGER
FOR i = 1 TO 4: scores(i) = i * 10: NEXT
FOR EACH s IN scores()
    total = total + s
NEXT s
PRINT total
```

<!-- expected output -->
```text
 100
```

Every element from LBOUND to UBOUND; arrays with more dimensions are walked
with the first index outermost. The loop variable is a copy of the element.

### `ENUM`

<!-- docs-test: output -->
```basic
ENUM Fruit
    Apple
    Banana = 5
    Cherry, Grape
END ENUM
PRINT Apple; Banana; Cherry; Grape
```

<!-- expected output -->
```text
 0  5  6  7
```

Members become integer `CONST`s, counting up from 0 or from the last
`= value`. `ENUM` without a name works too.

### `TRY` / `CATCH` / `FINALLY`

<!-- docs-test: output -->
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

<!-- expected output -->
```text
caught error 53
cleanup
```

Built on `ON ERROR`: `CATCH e` stores the error number in `e`; without
`CATCH` the error is raised again after `FINALLY`. TRY works inside SUBs
and in `$INCLUDE` files, and its keywords may share a line with other
statements (`TRY: ERROR 6: CATCH e: PRINT e: END TRY`). Integer division by
zero (`\` and `MOD`) is error 11 and can be caught. `ON ERROR GOTO` keeps
working as before: the handler that was set before a `TRY` is in force again
when its body ends, also when the body is left with `EXIT FOR`, `EXIT DO`,
`EXIT WHILE`, `_CONTINUE`, `EXIT SUB`, `EXIT FUNCTION` or a `GOTO` to a label
outside it.

### `OPTIONAL` parameters

<!-- docs-test: output -->
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

<!-- expected output -->
```text
Hello, Ada
Hi, Bob
Hi, Bob
 6  30
```

Trailing arguments may be left out; the default is any expression valid
where the procedure is called.

### `_CONTINUE`

<!-- docs-test: output -->
```basic
FOR i = 1 TO 5
    IF i MOD 2 = 0 THEN _CONTINUE
    PRINT i;
NEXT
```

<!-- expected output -->
```text
 1  3  5
```

Skips to the next iteration of the innermost `FOR`, `DO` or `WHILE` loop.

### `$ASSERTS` and `_ASSERT`

<!-- docs-test: output -->
```basic
$ASSERTS
_ASSERT 1 + 1 = 2, "math works"
PRINT "asserts passed"
```

<!-- expected output -->
```text
asserts passed
```

### Source files in CP437 or UTF-8

Files saved by QBasic (CP437 bytes, box drawing in strings) compile as they
are; the bytes are kept. UTF-8 files (with or without BOM) work too.

### Standard library modules: `'$IMPORT:'name'`

<!-- docs-test: output -->
```basic
'$IMPORT:'url'
PRINT Url_Encode("a b&c")
```

<!-- expected output -->
```text
a%20b%26c
```

The line may appear anywhere in the main file. Each module's declarations
are placed before the program and its procedures after it, so a module's
`DIM SHARED`, TYPEs and SUBs work wherever the import line is. See
[Standard library](stdlib.md).

### Method calls on TYPE values

<!-- docs-test: output -->
```basic
TYPE Point
    x AS SINGLE
    y AS SINGLE
END TYPE

DIM p AS Point
p.Move 3, 4                    ' Point_Move p, 3, 4
PRINT p.x; p.y; p.Length       ' Point_Length(p)
PRINT p.Label$("p")            ' Point_Label$(p, "p")
SYSTEM

SUB Point_Move (pt AS Point, dx AS SINGLE, dy AS SINGLE)
    pt.x = pt.x + dx: pt.y = pt.y + dy
END SUB

FUNCTION Point_Length! (pt AS Point)
    Point_Length! = SQR(pt.x * pt.x + pt.y * pt.y)
END FUNCTION

FUNCTION Point_Label$ (pt AS Point, label AS STRING)
    Point_Label$ = label + "(" + LTRIM$(STR$(pt.x)) + ", " + LTRIM$(STR$(pt.y)) + ")"
END FUNCTION
```

```text
 3  4  5
p(3, 4)
```

For a variable, parameter, array element or field whose type is a TYPE `T`
without a field of that name, `x.Name(args)` calls `T_Name(x, args)`. Without
brackets, a statement `x.Add "a"` calls `T_Add x, "a"` and `x.Clear` calls
`T_Clear x`; inside an expression `x.Count` calls `T_Count(x)`. A real field
always wins, so existing programs are unchanged, and a CLASS method wins over
a procedure of the same name. This is how the library types `List`, `Dict`,
`Set`, `Url` and `Result` are used (`names.Add "Ann"`,
`PRINT names.Join(", ")`); see [Standard library](stdlib.md).

### CLASS (text lowering, with `'$IMPORT:'oop.class'`)

```basic
'$IMPORT:'oop.class'
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
`EXTENDS` it. `oop.class` (and `oop.interface`) hold the runtime the
lowered code uses; `'$IMPORT:'qbnex'` imports them too.

### `DECLARE LIBRARY`, `_FLOAT`, `_MEM`

As in QB64. External functions are called through
`crates/engine/ffi` (Win64, System V x86-64 and AArch64 calling conventions,
including Apple's AArch64 variant, so Windows, Linux and macOS) and
`_FLOAT` is an exact software 80-bit float.

## Planned

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
- Array literals and arrays inside TYPE (variable-length strings inside TYPE
  work already: `nm AS STRING`).
- String helpers `_SPLIT`, `_JOIN$`, `_REPLACE$`, `_STARTSWITH`,
  `_ENDSWITH`, `_FORMAT$` and UTF-8 helpers `_ULEN`, `_UMID$`, `_UCHR$`,
  `_UASC` (today: the [`strings`](stdlib.md#strings),
  [`utf8`](stdlib.md#utf8) and [`regex`](stdlib.md#regex) modules).
- Built-in `_LIST` / `_DICT` (today: the [`collections`](stdlib.md#collections) module).
- `_REGEX`, `_RANDOMSEED` and UUIDs as built-ins.
- CLASS in QIR: virtual dispatch, `NEW`/`DELETE`, properties; function
  pointers (`@SubName`); overloading by type.
- `x += 1` (and `-=`, `*=`, `/=`), `# comment` lines, `IMPORT module`
  without the `'$IMPORT:` form and `FUNCTION name (...) AS type`.
