' OpenGL keyword table (SUB _GL).
' gl_scan_header reads compiler\support\opengl\gl_table.txt once at start-up (data converted once
' from QB64's gl.h parsing copy; no C header in the repository): every "const GL_x value" becomes a
' _GL_x constant and every "sub|func glX ..." line a _glX SUB/FUNCTION.
' gl_include_content registers them (id.ccall = 1, internal) when the program declares SUB _GL.
' The QIR emitter lowers a call of one of them like a DECLARE LIBRARY call with library "opengl"
' (IrExtCall$ in ir_emit.bas): "call glX args" plus "meta extfn glX opengl <ret> <argtypes>".
' Pointer arguments and pointer results are _OFFSET values (QIR i64).

FUNCTION gl2qb_type_convert$ (a$, symbol$, typ, ctyp$)
    symbol$ = ""

    'unsigned int
    IF a$ = "GLenum" THEN b$ = "_UNSIGNED LONG": symbol$ = "~&": typ = ULONGTYPE - ISPOINTER: ctyp$ = "uint32"
    IF a$ = "GLbitfield" THEN b$ = "_UNSIGNED LONG": symbol$ = "~&": typ = ULONGTYPE - ISPOINTER: ctyp$ = "uint32"
    IF a$ = "GLuint" THEN b$ = "_UNSIGNED LONG": symbol$ = "~&": typ = ULONGTYPE - ISPOINTER: ctyp$ = "uint32"

    'int
    IF a$ = "GLint" THEN b$ = "LONG": symbol$ = "&": typ = LONGTYPE - ISPOINTER: ctyp$ = "int32"
    IF a$ = "GLsizei" THEN b$ = "LONG": symbol$ = "&": typ = LONGTYPE - ISPOINTER: ctyp$ = "int32"

    'unsigned char
    IF a$ = "GLboolean" THEN b$ = "_UNSIGNED _BYTE": symbol$ = "~%%": typ = UBYTETYPE - ISPOINTER: ctyp$ = "uint8"
    IF a$ = "GLubyte" THEN b$ = "_UNSIGNED _BYTE": symbol$ = "~%%": typ = UBYTETYPE - ISPOINTER: ctyp$ = "uint8"

    'char
    IF a$ = "GLbyte" THEN b$ = "_BYTE": symbol$ = "%%": typ = BYTETYPE - ISPOINTER: ctyp$ = "int8"

    'unsigned short
    IF a$ = "GLushort" THEN b$ = "_UNSIGNED INTEGER": symbol$ = "~%": typ = UINTEGERTYPE - ISPOINTER: ctyp$ = "uint16"

    'short
    IF a$ = "GLshort" THEN b$ = "INTEGER": symbol$ = "%": typ = INTEGERTYPE - ISPOINTER: ctyp$ = "int16"

    'float
    IF a$ = "GLfloat" THEN b$ = "SINGLE": symbol$ = "!": typ = SINGLETYPE - ISPOINTER: ctyp$ = "float"
    IF a$ = "GLclampf" THEN b$ = "SINGLE": symbol$ = "!": typ = SINGLETYPE - ISPOINTER: ctyp$ = "float"

    'double
    IF a$ = "GLdouble" THEN b$ = "DOUBLE": symbol$ = "#": typ = DOUBLETYPE - ISPOINTER: ctyp$ = "double"
    IF a$ = "GLclampd" THEN b$ = "DOUBLE": symbol$ = "#": typ = DOUBLETYPE - ISPOINTER: ctyp$ = "double"

    'void
    IF a$ = "GLvoid" THEN b$ = "_OFFSET": symbol$ = "%&": typ = OFFSETTYPE - ISPOINTER: ctyp$ = "ptrszint"

    IF b$ = "" THEN PRINT "gl_table.txt: unknown type: " + a$: END
    gl2qb_type_convert$ = b$
END FUNCTION

'QBNex type of a gl_table.txt argument/result type (GL type name, or "ptr": an _OFFSET)
FUNCTION gl_table_type& (t$)
    DIM s$, typ AS LONG, ctyp$
    IF t$ = "ptr" THEN gl_table_type& = OFFSETTYPE - ISPOINTER: EXIT FUNCTION
    s$ = gl2qb_type_convert$(t$, s$, typ, ctyp$)
    gl_table_type& = typ
END FUNCTION

'next space-separated word of a$ (removed from a$)
FUNCTION gl_word$ (a$)
    DIM x AS LONG
    a$ = LTRIM$(a$)
    x = INSTR(a$, " ")
    IF x = 0 THEN gl_word$ = a$: a$ = "": EXIT FUNCTION
    gl_word$ = LEFT$(a$, x - 1): a$ = MID$(a$, x + 1)
END FUNCTION

SUB gl_scan_header
    DIM a$, k$, nm$, rt$, t$, glt$, c AS LONG, d AS LONG, h AS LONG

    GL_COMMANDS_LAST = 0
    GL_DEFINES_LAST = 0
    glt$ = getfilepath$(COMMAND$(0)) + "compiler" + pathsep$ + "support" + pathsep$ + "opengl" + pathsep$ + "gl_table.txt"
    IF _FILEEXISTS(glt$) = 0 THEN glt$ = "compiler" + pathsep$ + "support" + pathsep$ + "opengl" + pathsep$ + "gl_table.txt"
    IF _FILEEXISTS(glt$) = 0 THEN EXIT SUB 'no table: SUB _GL programs then fail on the unknown _gl* names

    h = FREEFILE
    OPEN glt$ FOR BINARY AS #h
    DO UNTIL EOF(h)
        LINE INPUT #h, a$
        IF RIGHT$(a$, 1) = CHR$(13) THEN a$ = LEFT$(a$, LEN(a$) - 1)
        k$ = gl_word$(a$)
        SELECT CASE k$
            CASE "const"
                d = d + 1
                GL_DEFINES$(d) = gl_word$(a$)
                GL_DEFINES_VALUE(d) = VAL(gl_word$(a$) + "&&")
            CASE "sub", "func"
                GL_COMMANDS_LAST = GL_COMMANDS_LAST + 1
                c = GL_COMMANDS_LAST
                nm$ = gl_word$(a$)
                rt$ = gl_word$(a$)
                GL_COMMAND_CN$(c) = "_" + nm$
                GL_COMMAND_CALLNAME$(c) = nm$ 'the plain GL entry point name
                GL_COMMAND_RET(c) = 0
                IF k$ = "func" THEN
                    GL_COMMAND_SUBFUNC(c) = 1
                    GL_COMMAND_RET(c) = gl_table_type&(rt$)
                ELSE
                    GL_COMMAND_SUBFUNC(c) = 2
                END IF
                GL_COMMAND_ARGS(c) = 0
                GL_COMMAND_ARG$(c) = ""
                DO
                    t$ = gl_word$(a$)
                    IF t$ = "" THEN EXIT DO
                    GL_COMMAND_ARGS(c) = GL_COMMAND_ARGS(c) + 1
                    MID$(GL_COMMAND_ARG$(c), (GL_COMMAND_ARGS(c) - 1) * 4 + 1, 4) = MKL$(gl_table_type&(t$))
                LOOP
        END SELECT
    LOOP
    CLOSE #h

    GL_DEFINES_LAST = d
END SUB
SUB gl_include_content
    DIM d AS LONG
    DIM i AS LONG
    DIM c AS LONG

    'add constants (_GL_*, _INTEGER64)
    FOR d = 1 TO GL_DEFINES_LAST
        constlast = constlast + 1
        IF constlast > constmax THEN
            constmax = constmax * 2
            REDIM _PRESERVE constname(constmax) AS STRING
            REDIM _PRESERVE constcname(constmax) AS STRING
            REDIM _PRESERVE constnamesymbol(constmax) AS STRING
            REDIM _PRESERVE consttype(constmax) AS LONG
            REDIM _PRESERVE constinteger(constmax) AS _INTEGER64
            REDIM _PRESERVE constuinteger(constmax) AS _UNSIGNED _INTEGER64
            REDIM _PRESERVE constfloat(constmax) AS _FLOAT
            REDIM _PRESERVE conststring(constmax) AS STRING
            REDIM _PRESERVE constsubfunc(constmax) AS LONG
            REDIM _PRESERVE constdefined(constmax) AS LONG
        END IF
        i = constlast
        constname(i) = qbnexprefix$ + GL_DEFINES$(d)
        constcname(i) = qbnexprefix$ + GL_DEFINES$(d)
        constnamesymbol(i) = "&&"
        consttype(i) = INTEGER64TYPE - ISPOINTER
        constinteger(i) = GL_DEFINES_VALUE(d)
        constsubfunc(i) = 0
        constdefined(i) = 1
        HashAdd constcname(i), HASHFLAG_CONSTANT, i
    NEXT

    'add subs/functions (_gl*): internal, ccall; the QIR emitter lowers them as library "opengl" calls
    FOR c = 1 TO GL_COMMANDS_LAST
        reginternalsubfunc = 1
        clearid
        id.ccall = 1
        id.n = RTRIM$(GL_COMMAND_CN$(c))
        IF qbnexprefix_set = 1 THEN id.n = MID$(RTRIM$(GL_COMMAND_CN$(c)), 2)
        id.subfunc = GL_COMMAND_SUBFUNC(c)
        id.callname = RTRIM$(GL_COMMAND_CALLNAME$(c))
        id.args = GL_COMMAND_ARGS(c)
        id.arg = GL_COMMAND_ARG$(c)
        id.ret = GL_COMMAND_RET(c)
        regid
        reginternalsubfunc = 0
    NEXT

    'GLU helper
    reginternalsubfunc = 1
    clearid
    id.ccall = 1
    id.n = qbnexprefix$ + "gluPerspective"
    id.subfunc = 2
    id.callname = "gluPerspective"
    id.args = 4
    id.arg = MKL$(DOUBLETYPE - ISPOINTER) + MKL$(DOUBLETYPE - ISPOINTER) + MKL$(DOUBLETYPE - ISPOINTER) + MKL$(DOUBLETYPE - ISPOINTER)
    regid
    reginternalsubfunc = 0
END SUB
