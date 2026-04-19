FUNCTION allocarray (n2$, elements$, elementsize, udt)
    dimsharedlast = dimshared: dimshared = 0

    IF autoarray = 1 THEN autoarray = 0: autoary = 1 'clear global value & set local value

    f12$ = ""

    'changelog:
    'added 4 to [2] to indicate cmem array where appropriate

    e$ = elements$: n$ = n2$
    IF elementsize = -2147483647 THEN stringarray = 1: elementsize = 8

    IF ASC(e$) = 63 THEN '?
    l$ = "(" + sp2 + ")"
    undefined = -1
    nume = 1
    IF LEN(e$) = 1 THEN GOTO undefinedarray
    undefined = 1
    nume = VAL(RIGHT$(e$, LEN(e$) - 1))
    GOTO undefinedarray
END IF


'work out how many elements there are (critical to later calculations)
nume = 1
n = numelements(e$)
FOR i = 1 TO n
    e2$ = getelement(e$, i)
    IF e2$ = "(" THEN b = b + 1
    IF b = 0 AND e2$ = "," THEN nume = nume + 1
    IF e2$ = ")" THEN b = b - 1
NEXT
IF Debug THEN PRINT #9, "numelements count:"; nume

descstatic = 0
IF arraydesc THEN
    IF id.arrayelements <> nume THEN

        IF id.arrayelements = -1 THEN 'unknown
        IF arrayelementslist(currentid) <> 0 AND nume <> arrayelementslist(currentid) THEN Give_Error "Cannot change the number of elements an array has!": EXIT FUNCTION
        IF nume = 1 THEN id.arrayelements = 1: ids(currentid).arrayelements = 1 'lucky guess!
        arrayelementslist(currentid) = nume
    ELSE
        Give_Error "Cannot change the number of elements an array has!": EXIT FUNCTION
    END IF

END IF
IF id.staticarray THEN descstatic = 1
END IF

l$ = "(" + sp2

cr$ = CHR$(13) + CHR$(10)
sd$ = ""
constdimensions = 1
ei = 4 + nume * 4 - 4
cure = 1
e3$ = "": e3base$ = ""
FOR i = 1 TO n
    e2$ = getelement(e$, i)
    IF e2$ = "(" THEN b = b + 1
    IF (e2$ = "," AND b = 0) OR i = n THEN
        IF i = n THEN e3$ = e3$ + sp + e2$
        e3$ = RIGHT$(e3$, LEN(e3$) - 1)
        IF e3base$ <> "" THEN e3base$ = RIGHT$(e3base$, LEN(e3base$) - 1)
        'PRINT e3base$ + "[TO]" + e3$
        'set the base

        basegiven = 1
        IF e3base$ = "" THEN e3base$ = str2$(optionbase + 0): basegiven = 0
        constequation = 1

        e3base$ = fixoperationorder$(e3base$)
        IF Error_Happened THEN EXIT FUNCTION
        IF basegiven THEN l$ = l$ + tlayout$ + sp + SCase$("To") + sp
        e3base$ = evaluatetotyp$(e3base$, 64&)
        IF Error_Happened THEN EXIT FUNCTION

        IF constequation = 0 THEN constdimensions = 0
        sd$ = sd$ + n$ + "[" + str2(ei) + "]=" + e3base$ + ";" + cr$
        'set the number of indexes
        constequation = 1

        e3$ = fixoperationorder$(e3$)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + tlayout$ + sp2
        IF i = n THEN l$ = l$ + ")" ELSE l$ = l$ + "," + sp
        e3$ = evaluatetotyp$(e3$, 64&)
        IF Error_Happened THEN EXIT FUNCTION

        IF constequation = 0 THEN constdimensions = 0
        ei = ei + 1
        sd$ = sd$ + n$ + "[" + str2(ei) + "]=(" + e3$ + ")-" + n$ + "[" + str2(ei - 1) + "]+1;" + cr$
        ei = ei + 1
        'calc muliplier
        IF cure = 1 THEN
            'set only for the purpose of the calculating correct multipliers
            sd$ = sd$ + n$ + "[" + str2(ei) + "]=1;" + cr$
        ELSE
            sd$ = sd$ + n$ + "[" + str2(ei) + "]=" + n$ + "[" + str2(ei + 4) + "]*" + n$ + "[" + str2(ei + 3) + "];" + cr$
        END IF
        ei = ei + 1
        ei = ei + 1 'skip reserved
        ei = ei - 8
        cure = cure + 1
        e3$ = "": e3base$ = ""
        GOTO aanexte
    END IF
    IF e2$ = ")" THEN b = b - 1
    IF UCASE$(e2$) = "TO" AND b = 0 THEN
        e3base$ = e3$
        e3$ = ""
    ELSE
        e3$ = e3$ + sp + e2$
    END IF
    aanexte:
NEXT
sd$ = LEFT$(sd$, LEN(sd$) - 2)

undefinedarray:

'calc cmem
cmem = 0
IF arraydesc = 0 THEN
    IF cmemlist(idn + 1) THEN cmem = 1
ELSE
    IF cmemlist(arraydesc) THEN cmem = 1
END IF

staticarray = constdimensions
IF subfuncn <> 0 AND dimstatic = 0 THEN staticarray = 0 'arrays in SUBS/FUNCTIONS are DYNAMIC
IF dimstatic = 3 THEN staticarray = 0 'STATIC arrayname() listed arrays keep thier values but are dynamic in memory
IF DynamicMode THEN staticarray = 0
IF redimoption THEN staticarray = 0
IF dimoption = 3 THEN staticarray = 0 'STATIC a(100) arrays are still dynamic

IF arraydesc THEN
    IF staticarray = 1 THEN
        IF descstatic THEN Give_Error "Cannot redefine a static array!": EXIT FUNCTION
        staticarray = 0
    END IF
END IF






bytesperelement$ = str2(elementsize)
IF elementsize < 0 THEN
    elementsize = -elementsize
    bytesperelement$ = str2(elementsize) + "/8+1"
END IF


'Begin creation of array descriptor (if array has not been defined yet)
IF arraydesc = 0 THEN
    PRINT #defdatahandle, "ptrszint *" + n$ + "=NULL;"
    PRINT #13, "if (!" + n$ + "){"
    PRINT #13, n$ + "=(ptrszint*)mem_static_malloc(" + str2(4 * nume + 4 + 1) + "*ptrsz);" '+1 is for the lock
    'create _MEM lock
    PRINT #13, "new_mem_lock();"
    PRINT #13, "mem_lock_tmp->type=4;"
    PRINT #13, "((ptrszint*)" + n$ + ")[" + str2(4 * nume + 4 + 1 - 1) + "]=(ptrszint)mem_lock_tmp;"
END IF

'generate sizestr$ & elesizestr$ (both are used in various places in following code)
sizestr$ = ""
FOR i = 1 TO nume
    IF i <> 1 THEN sizestr$ = sizestr$ + "*"
    sizestr$ = sizestr$ + n$ + "[" + str2(i * 4 - 4 + 5) + "]"
NEXT
elesizestr$ = sizestr$ 'elements in entire array
sizestr$ = sizestr$ + "*" + bytesperelement$ 'bytes in entire array



'------------------STATIC ARRAY CREATION--------------------------------
IF staticarray THEN
    'STATIC memory
    PRINT #13, sd$ 'setup new array dimension ranges
    'Example of sd$ for DIM a(10):
    '__ARRAY_SINGLE_A[4]= 0 ;
    '__ARRAY_SINGLE_A[5]=( 10 )-__ARRAY_SINGLE_A[4]+1;
    '__ARRAY_SINGLE_A[6]=1;
    IF cmem AND stringarray = 0 THEN
        'Note: A string array's pointers are always stored in 64bit memory
        '(static)CONVENTINAL memory
        PRINT #13, n$ + "[0]=(ptrszint)cmem_static_pointer;"
        'alloc mem & check if static memory boundry has oversteped dynamic memory boundry
        PRINT #13, "if ((cmem_static_pointer+=((" + sizestr$ + ")+15)&-16)>cmem_dynamic_base) error(257);"
        '64K check
        PRINT #13, "if ((" + sizestr$ + ")>65536) error(257);"
        'clear array
        PRINT #13, "memset((void*)(" + n$ + "[0]),0," + sizestr$ + ");"
        'set flags
        PRINT #13, n$ + "[2]=1+2+4;" 'init+static+cmem
    ELSE
        '64BIT MEMORY
        PRINT #13, n$ + "[0]=(ptrszint)mem_static_malloc(" + sizestr$ + ");"
        IF stringarray THEN
            'Init string pointers in the array
            PRINT #13, "tmp_long=" + elesizestr$ + ";"
            PRINT #13, "while(tmp_long--){"
            IF cmem THEN
                PRINT #13, "((uint64*)(" + n$ + "[0]))[tmp_long]=(uint64)qbs_new_cmem(0,0);"
            ELSE
                PRINT #13, "((uint64*)(" + n$ + "[0]))[tmp_long]=(uint64)qbs_new(0,0);"
            END IF
            PRINT #13, "}"
        ELSE
            'clear array
            PRINT #13, "memset((void*)(" + n$ + "[0]),0," + sizestr$ + ");"
        END IF
        PRINT #13, n$ + "[2]=1+2;" 'init+static
    END IF

    IF udt > 0 AND udtxvariable(udt) THEN
        PRINT #13, "tmp_long=" + elesizestr$ + ";"
        PRINT #13, "while(tmp_long--){"
        initialise_array_udt_varstrings n$, udt, 0, bytesperelement$, acc$
        PRINT #13, acc$
        PRINT #13, "}"
    END IF

    'Close static array desc
    PRINT #13, "}"
    allocarray = nume + 65536
END IF
'------------------END OF STATIC ARRAY CREATION-------------------------

'------------------DYNAMIC ARRAY CREATION-------------------------------
IF staticarray = 0 THEN

    IF undefined = 0 THEN



        'Generate error if array is static
        f12$ = f12$ + CRLF + "if (" + n$ + "[2]&2){" 'static array
        f12$ = f12$ + CRLF + "error(10);" 'cannot redefine a static array!
        f12$ = f12$ + CRLF + "}else{"
        'Note: Array is either undefined or dynamically defined at this point


        'REDIM (not DIM) must be used to redefine an array
        IF redimoption = 0 THEN
            f12$ = f12$ + CRLF + "if (" + n$ + "[2]&1){" 'array is defined
            f12$ = f12$ + CRLF + "if (!error_occurred) error(10);" 'cannot redefine an array without using REDIM!
            f12$ = f12$ + CRLF + "}else{"
        ELSE
            '--------ERASE EXISTING ARRAY IF NECESSARY--------

            'IMPORTANT: If array is not going to be preserved, it should be cleared before
            '           creating the new array for memory considerations

            'refresh lock ID (_MEM)
            f12$ = f12$ + CRLF + "((mem_lock*)((ptrszint*)" + n$ + ")[" + str2(4 * nume + 4 + 1 - 1) + "])->id=(++mem_lock_id);"

            IF redimoption = 2 THEN
                f12$ = f12$ + CRLF + "static int32 preserved_elements;" 'must be put here for scope considerations
            END IF

            'If array is defined, it must be destroyed first
            f12$ = f12$ + CRLF + "if (" + n$ + "[2]&1){" 'array is defined

            IF redimoption = 2 THEN
                f12$ = f12$ + CRLF + "preserved_elements=" + elesizestr$ + ";"
                GOTO skiperase
            END IF

            'Note: pointers to strings must be freed before array can be freed
            IF stringarray THEN
                f12$ = f12$ + CRLF + "tmp_long=" + elesizestr$ + ";"
                f12$ = f12$ + CRLF + "while(tmp_long--) qbs_free((qbs*)((uint64*)(" + n$ + "[0]))[tmp_long]);"
            END IF
            'As must any variable length strings in UDT's
            IF udt > 0 AND udtxvariable(udt) THEN
                f12$ = f12$ + CRLF + "tmp_long=" + elesizestr$ + ";"
                f12$ = f12$ + CRLF + "while(tmp_long--) {"
                free_array_udt_varstrings n$, udt, 0, bytesperelement$, acc$
                f12$ = f12$ + acc$ + "}"
            END IF

            'Free array's memory
            IF stringarray THEN
                'Note: String arrays are never in cmem
                f12$ = f12$ + CRLF + "free((void*)(" + n$ + "[0]));"
            ELSE
                'Note: Array may be in cmem!
                f12$ = f12$ + CRLF + "if (" + n$ + "[2]&4){" 'array is in cmem
                f12$ = f12$ + CRLF + "cmem_dynamic_free((uint8*)(" + n$ + "[0]));"
                f12$ = f12$ + CRLF + "}else{" 'not in cmem
                f12$ = f12$ + CRLF + "free((void*)(" + n$ + "[0]));"
                f12$ = f12$ + CRLF + "}"
            END IF

            skiperase:

            f12$ = f12$ + CRLF + "}" 'array was defined
            IF redimoption = 2 THEN
                f12$ = f12$ + CRLF + "else preserved_elements=0;" 'if array wasn't defined, no elements are preserved
            END IF


            '--------ERASED ARRAY AS NECESSARY--------
        END IF 'redim specified


        '--------CREATE ARRAY & CLEAN-UP CODE--------
        'Overwrite existing array dimension sizes/ranges
        f12$ = f12$ + CRLF + sd$
        IF stringarray OR ((udt > 0) AND udtxvariable(udt)) THEN

            'Note: String and variable-length udt arrays are always created in 64bit memory

            IF redimoption = 2 THEN
                f12$ = f12$ + CRLF + "if (preserved_elements){"

                f12$ = f12$ + CRLF + "static ptrszint tmp_long2;"

                'free any qbs strings which will be lost in the realloc
                f12$ = f12$ + CRLF + "tmp_long2=" + elesizestr$ + ";"
                f12$ = f12$ + CRLF + "if (tmp_long2<preserved_elements){"
                f12$ = f12$ + CRLF + "for(tmp_long=tmp_long2;tmp_long<preserved_elements;tmp_long++) {"
                IF stringarray THEN
                    f12$ = f12$ + CRLF + "qbs_free((qbs*)((uint64*)(" + n$ + "[0]))[tmp_long]);"
                ELSE
                    acc$ = ""
                    free_array_udt_varstrings n$, udt, 0, bytesperelement$, acc$
                    f12$ = f12$ + acc$
                END IF
                f12$ = f12$ + CRLF + "}}"
                'reallocate the array
                f12$ = f12$ + CRLF + n$ + "[0]=(ptrszint)realloc((void*)(" + n$ + "[0]),tmp_long2*" + bytesperelement$ + ");"
                f12$ = f12$ + CRLF + "if (!" + n$ + "[0]) error(257);" 'not enough memory
                f12$ = f12$ + CRLF + "if (preserved_elements<tmp_long2){"
                f12$ = f12$ + CRLF + "for(tmp_long=preserved_elements;tmp_long<tmp_long2;tmp_long++){"
                IF stringarray THEN
                    f12$ = f12$ + CRLF + "if (" + n$ + "[2]&4){" 'array is in cmem
                    f12$ = f12$ + CRLF + "((uint64*)(" + n$ + "[0]))[tmp_long]=(uint64)qbs_new_cmem(0,0);"
                    f12$ = f12$ + CRLF + "}else{" 'not in cmem
                    f12$ = f12$ + CRLF + "((uint64*)(" + n$ + "[0]))[tmp_long]=(uint64)qbs_new(0,0);"
                    f12$ = f12$ + CRLF + "}" 'not in cmem
                ELSE
                    acc$ = ""
                    initialise_array_udt_varstrings n$, udt, 0, bytesperelement$, acc$
                    f12$ = f12$ + acc$
                END IF
                f12$ = f12$ + CRLF + "}"
                f12$ = f12$ + CRLF + "}"

                f12$ = f12$ + CRLF + "}else{"
            END IF

            '1. Create array
            f12$ = f12$ + CRLF + n$ + "[0]=(ptrszint)malloc(" + sizestr$ + ");"
            f12$ = f12$ + CRLF + "if (!" + n$ + "[0]) error(257);" 'not enough memory
            f12$ = f12$ + CRLF + n$ + "[2]|=1;" 'ADD initialized flag
            f12$ = f12$ + CRLF + "tmp_long=" + elesizestr$ + ";"


            'init individual strings
            IF stringarray THEN
                f12$ = f12$ + CRLF + "if (" + n$ + "[2]&4){" 'array is in cmem
                f12$ = f12$ + CRLF + "while(tmp_long--) ((uint64*)(" + n$ + "[0]))[tmp_long]=(uint64)qbs_new_cmem(0,0);"
                f12$ = f12$ + CRLF + "}else{" 'not in cmem
                f12$ = f12$ + CRLF + "while(tmp_long--) ((uint64*)(" + n$ + "[0]))[tmp_long]=(uint64)qbs_new(0,0);"
                f12$ = f12$ + CRLF + "}" 'not in cmem
            ELSE 'initialise udt's
                f12$ = f12$ + CRLF + "while(tmp_long--){"
                acc$ = ""
                initialise_array_udt_varstrings n$, udt, 0, bytesperelement$, acc$
                f12$ = f12$ + acc$ + "}"
            END IF

            IF redimoption = 2 THEN
                f12$ = f12$ + CRLF + "}"
            END IF


            '2. Generate "clean up" code (called when EXITING A SUB/FUNCTION)
            IF arraydesc = 0 THEN 'only add for first declaration of the array
            PRINT #19, "if (" + n$ + "[2]&1){" 'initialized?
            PRINT #19, "tmp_long=" + elesizestr$ + ";"
            IF udt > 0 AND udtxvariable(udt) THEN
                PRINT #19, "while(tmp_long--) {"
                acc$ = ""
                free_array_udt_varstrings n$, udt, 0, bytesperelement$, acc$
                PRINT #19, acc$ + "}"
            ELSE
                PRINT #19, "while(tmp_long--) qbs_free((qbs*)((uint64*)(" + n$ + "[0]))[tmp_long]);"
            END IF
            PRINT #19, "free((void*)(" + n$ + "[0]));"
            PRINT #19, "}"
            'free lock (_MEM)
            PRINT #19, "free_mem_lock( (mem_lock*)((ptrszint*)" + n$ + ")[" + str2(4 * nume + 4 + 1 - 1) + "] );"
        END IF


    ELSE 'not string/var-udt array

        '1. Create array
        f12$ = f12$ + CRLF + "if (" + n$ + "[2]&4){" 'array will be in cmem

        IF redimoption = 2 THEN
            f12$ = f12$ + CRLF + "if (preserved_elements){"

            'reallocation method
            'backup data
            f12$ = f12$ + CRLF + "memcpy(redim_preserve_cmem_buffer,(void*)(" + n$ + "[0]),preserved_elements*" + bytesperelement$ + ");"
            'free old array
            f12$ = f12$ + CRLF + "cmem_dynamic_free((uint8*)(" + n$ + "[0]));"
            f12$ = f12$ + CRLF + "tmp_long=" + elesizestr$ + ";"
            f12$ = f12$ + CRLF + n$ + "[0]=(ptrszint)cmem_dynamic_malloc(tmp_long*" + bytesperelement$ + ");"
            f12$ = f12$ + CRLF + "memcpy((void*)(" + n$ + "[0]),redim_preserve_cmem_buffer,preserved_elements*" + bytesperelement$ + ");"
            f12$ = f12$ + CRLF + "if (preserved_elements<tmp_long) ZeroMemory(((uint8*)(" + n$ + "[0]))+preserved_elements*" + bytesperelement$ + ",(tmp_long*" + bytesperelement$ + ")-(preserved_elements*" + bytesperelement$ + "));"

            f12$ = f12$ + CRLF + "}else{"
        END IF

        'standard cmem method
        f12$ = f12$ + CRLF + n$ + "[0]=(ptrszint)cmem_dynamic_malloc(" + sizestr$ + ");"
        'clear array
        f12$ = f12$ + CRLF + "memset((void*)(" + n$ + "[0]),0," + sizestr$ + ");"

        IF redimoption = 2 THEN
            f12$ = f12$ + CRLF + "}"
        END IF


        f12$ = f12$ + CRLF + "}else{" 'not in cmem

        IF redimoption = 2 THEN
            f12$ = f12$ + CRLF + "if (preserved_elements){"
            'reallocation method
            f12$ = f12$ + CRLF + "tmp_long=" + elesizestr$ + ";"
            f12$ = f12$ + CRLF + n$ + "[0]=(ptrszint)realloc((void*)(" + n$ + "[0]),tmp_long*" + bytesperelement$ + ");"
            f12$ = f12$ + CRLF + "if (!" + n$ + "[0]) error(257);" 'not enough memory
            f12$ = f12$ + CRLF + "if (preserved_elements<tmp_long) ZeroMemory(((uint8*)(" + n$ + "[0]))+preserved_elements*" + bytesperelement$ + ",(tmp_long*" + bytesperelement$ + ")-(preserved_elements*" + bytesperelement$ + "));"

            f12$ = f12$ + CRLF + "}else{"
        END IF
        'standard allocation method
        f12$ = f12$ + CRLF + n$ + "[0]=(ptrszint)calloc(" + sizestr$ + ",1);"
        f12$ = f12$ + CRLF + "if (!" + n$ + "[0]) error(257);" 'not enough memory
        IF redimoption = 2 THEN
            f12$ = f12$ + CRLF + "}"
        END IF

        f12$ = f12$ + CRLF + "}" 'not in cmem
        f12$ = f12$ + CRLF + n$ + "[2]|=1;" 'ADD initialized flag

        '2. Generate "clean up" code (called when EXITING A SUB/FUNCTION)
        IF arraydesc = 0 THEN 'only add for first declaration of the array
        PRINT #19, "if (" + n$ + "[2]&1){" 'initialized?
        PRINT #19, "if (" + n$ + "[2]&4){" 'array is in cmem
        PRINT #19, "cmem_dynamic_free((uint8*)(" + n$ + "[0]));"
        PRINT #19, "}else{"
        PRINT #19, "free((void*)(" + n$ + "[0]));"
        PRINT #19, "}" 'cmem
        PRINT #19, "}" 'init
        'free lock (_MEM)
        PRINT #19, "free_mem_lock( (mem_lock*)((ptrszint*)" + n$ + ")[" + str2(4 * nume + 4 + 1 - 1) + "] );"
    END IF
END IF 'not string array

END IF 'undefined=0

'----FINISH ARRAY DESCRIPTOR IF DEFINING FOR THE FIRST TIME----
IF arraydesc = 0 THEN
    'Note: Array is init as undefined (& possibly a cmem flag)
    IF cmem THEN PRINT #13, n$ + "[2]=4;" ELSE PRINT #13, n$ + "[2]=0;"
    'set dimensions as undefined
    FOR i = 1 TO nume
        b = i * 4
        PRINT #13, n$ + "[" + str2(b) + "]=2147483647;" 'base
        PRINT #13, n$ + "[" + str2(b + 1) + "]=0;" 'num. index
        PRINT #13, n$ + "[" + str2(b + 2) + "]=0;" 'multiplier
    NEXT
    IF stringarray THEN
        'set array's data offset to the offset of the offset to nothingstring
        PRINT #13, n$ + "[0]=(ptrszint)&nothingstring;"
    ELSE
        'set array's data offset to "nothing"
        PRINT #13, n$ + "[0]=(ptrszint)nothingvalue;"
    END IF
    PRINT #13, "}" 'close array descriptor
END IF 'arraydesc = 0

IF undefined = 0 THEN

    IF redimoption = 0 THEN f12$ = f12$ + CRLF + "}" 'if REDIM not specified the above is conditional
    f12$ = f12$ + CRLF + "}" 'not static

END IF 'undefined=0

allocarray = nume
IF undefined = -1 THEN allocarray = -1

END IF

IF autoary = 0 THEN
    IF dimoption = 3 THEN 'STATIC a(100) puts creation code in main
    PRINT #13, f12$
ELSE
    PRINT #12, f12$
END IF
END IF

'[8] offset of data
'[8] reserved (could be used to store a bit offset)
'(the following repeats depending on the number of elements)
'[4] base-offset
'[4] number of indexes
'[4] multiplier (the last multiplier doesn't actually exist)
'[4] reserved

dimshared = dimsharedlast

tlayout$ = l$
END FUNCTION

SUB assign (a$, n)
    FOR i = 1 TO n
        c = ASC(getelement$(a$, i))
        IF c = 40 THEN b = b + 1 '(
        IF c = 41 THEN b = b - 1 ')
        IF c = 61 AND b = 0 THEN '=
        IF i = 1 THEN Give_Error "Expected ... =": EXIT SUB
        IF i = n THEN Give_Error "Expected = ...": EXIT SUB

        a2$ = fixoperationorder(getelements$(a$, 1, i - 1))
        IF Error_Happened THEN EXIT SUB
        l$ = tlayout$ + sp + "=" + sp

        'note: evaluating a2$ will fail if it is setting a function's return value without this check (as the function, not the return-variable) will be found by evaluate)
        IF i = 2 THEN 'lhs has only 1 element
        try = findid(a2$)
        IF Error_Happened THEN EXIT SUB
        DO WHILE try
            IF id.t THEN
                IF subfuncn = id.insubfuncn THEN 'avoid global before local
                IF (id.t AND ISUDT) = 0 THEN
                    makeidrefer a2$, typ
                    GOTO assignsimplevariable
                END IF
            END IF
        END IF
        IF try = 2 THEN findanotherid = 1: try = findid(a2$) ELSE try = 0
        IF Error_Happened THEN EXIT SUB
    LOOP
END IF

a2$ = evaluate$(a2$, typ): IF Error_Happened THEN EXIT SUB
assignsimplevariable:
IF (typ AND ISREFERENCE) = 0 THEN Give_Error "Expected variable =": EXIT SUB
setrefer a2$, typ, getelements$(a$, i + 1, n), 0
IF Error_Happened THEN EXIT SUB
tlayout$ = l$ + tlayout$

EXIT SUB

END IF '=,b=0
NEXT
Give_Error "Expected =": EXIT SUB
END SUB

SUB clearid
    id = cleariddata
END SUB

FUNCTION dim2 (varname$, typ2$, method, elements$)

    'notes: (DO NOT REMOVE THESE IMPORTANT USAGE NOTES)
    '
    '(shared)dimsfarray: Creates an ID only (no C++ code)
    '                    Adds an index/'link' to the sub/function's argument
    '                        ID.sfid=glinkid
    '                        ID.sfarg=glinkarg
    '                    Sets arrayelements=-1 'unknown' (if elements$="?") otherwise val(elements$)
    '                    ***Does not refer to arrayelementslist()***
    '
    '(argument)method: 0 being created by a DIM name AS type
    '                  1 being created by a DIM name+symbol
    '                  or automatically without the use of DIM
    '
    'elements$="?": (see also dimsfarray for that special case)
    '               Checks arrayelementslist() and;
    '               if unknown(=0), creates an ID only
    '               if known, creates a DYNAMIC array's C++ initialization code so it can be used later

    typ$ = typ2$
    dim2 = 1 'success

    IF Debug THEN PRINT #9, "dim2 called", method

    cvarname$ = varname$
    l$ = cvarname$
    varname$ = UCASE$(varname$)

    IF dimsfarray = 1 THEN f = 0 ELSE f = 1

    IF dimstatic <> 0 AND dimshared = 0 THEN
        'name will have include the sub/func name in its scope
        'variable/array will be created in main on startup
        defdatahandle = 18 'change from 13 to 18(global.txt)
        CLOSE #13: OPEN tmpdir$ + "maindata.txt" FOR APPEND AS #13
        CLOSE #19: OPEN tmpdir$ + "mainfree.txt" FOR APPEND AS #19
    END IF


    scope2$ = module$ + "_" + subfunc$ + "_"
    'Note: when REDIMing a SHARED array in dynamic memory scope2$ must be modified

    IF LEN(typ$) = 0 THEN Give_Error "DIM2: No type specified!": EXIT FUNCTION

    'UDT
    'is it a udt?
    FOR i = 1 TO lasttype
        IF typ$ = RTRIM$(udtxname(i)) OR (typ$ = "MEM" AND RTRIM$(udtxname(i)) = "_MEM" AND qbnexprefix_set = 1) THEN
            dim2typepassback$ = RTRIM$(udtxcname(i))
            IF typ$ = "MEM" AND RTRIM$(udtxname(i)) = "_MEM" THEN
                dim2typepassback$ = MID$(RTRIM$(udtxcname(i)), 2)
            END IF

            n$ = "UDT_" + varname$

            'array of UDTs
            IF elements$ <> "" THEN
                arraydesc = 0
                IF f = 1 THEN
                    try = findid(varname$)
                    IF Error_Happened THEN EXIT FUNCTION
                    DO WHILE try
                        IF (id.arraytype) THEN
                            l$ = RTRIM$(id.cn)
                            arraydesc = currentid: scope2$ = scope$
                            EXIT DO
                        END IF
                        IF try = 2 THEN findanotherid = 1: try = findid(varname$) ELSE try = 0
                        IF Error_Happened THEN EXIT FUNCTION
                    LOOP
                END IF
                n$ = scope2$ + "ARRAY_" + n$
                bits = udtxsize(i)
                IF udtxbytealign(i) THEN
                    IF bits MOD 8 THEN bits = bits + 8 - (bits MOD 8)
                END IF

                IF f = 1 THEN

                    IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
                    E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
                END IF
                nume = allocarray(n$, elements$, -bits, i)
                IF Error_Happened THEN EXIT FUNCTION
                l$ = l$ + sp + tlayout$
                IF arraydesc THEN GOTO dim2exitfunc
                clearid

            ELSE
                clearid
                IF elements$ = "?" THEN
                    nume = -1
                    id.linkid = glinkid
                    id.linkarg = glinkarg
                ELSE
                    nume = VAL(elements$)
                END IF
            END IF

            id.arraytype = UDTTYPE + i
            IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
            id.n = cvarname$

            IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

            id.arrayelements = nume
            id.callname = n$
            regid
            vWatchVariable n$, 0
            IF Error_Happened THEN EXIT FUNCTION
            GOTO dim2exitfunc
        END IF

        'not an array of UDTs
        bits = udtxsize(i): bytes = bits \ 8
        IF bits MOD 8 THEN
            bytes = bytes + 1
        END IF
        n$ = scope2$ + n$
        IF f THEN PRINT #defdatahandle, "void *" + n$ + "=NULL;"
        clearid
        id.n = cvarname$
        id.t = UDTTYPE + i
        IF cmemlist(idn + 1) THEN
            id.t = id.t + ISINCONVENTIONALMEMORY
            IF f THEN
                PRINT #13, "if(" + n$ + "==NULL){"
                PRINT #13, "cmem_sp-=" + str2(bytes) + ";"
                PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
                PRINT #13, n$ + "=(void*)(dblock+cmem_sp);"
                PRINT #13, "memset(" + n$ + ",0," + str2(bytes) + ");"
                PRINT #13, "}"
            END IF
        ELSE
            IF f THEN
                PRINT #13, "if(" + n$ + "==NULL){"
                PRINT #13, n$ + "=(void*)mem_static_malloc(" + str2$(bytes) + ");"
                PRINT #13, "memset(" + n$ + ",0," + str2(bytes) + ");"
                IF udtxvariable(i) THEN
                    initialise_udt_varstrings n$, i, 13, 0
                    free_udt_varstrings n$, i, 19, 0
                END IF
                PRINT #13, "}"
            END IF
        END IF
        id.callname = n$
        regid
        vWatchVariable n$, 0
        IF Error_Happened THEN EXIT FUNCTION
        GOTO dim2exitfunc
    END IF
NEXT i
'it isn't a udt

typ$ = symbol2fulltypename$(typ$)
IF Error_Happened THEN EXIT FUNCTION

'check if _UNSIGNED was specified
unsgn = 0
IF LEFT$(typ$, 10) = "_UNSIGNED " OR (LEFT$(typ$, 9) = "UNSIGNED " AND qbnexprefix_set = 1) THEN
    unsgn = 1
    typ$ = MID$(typ$, INSTR(typ$, CHR$(32)) + 1)
    IF LEN(typ$) = 0 THEN Give_Error "Expected more type information after " + qbnexprefix$ + "UNSIGNED!": EXIT FUNCTION
END IF

n$ = "" 'n$ is assumed to be "" after branching into the code for each type

IF LEFT$(typ$, 6) = "STRING" THEN

    IF LEN(typ$) > 6 THEN
        IF LEFT$(typ$, 9) <> "STRING * " THEN Give_Error "Expected STRING * number/constant": EXIT FUNCTION

        c$ = RIGHT$(typ$, LEN(typ$) - 9)

        'constant check 2011
        hashfound = 0
        hashname$ = c$
        hashchkflags = HASHFLAG_CONSTANT
        hashres = HashFindRev(hashname$, hashchkflags, hashresflags, hashresref)
        DO WHILE hashres
            IF constsubfunc(hashresref) = subfuncn OR constsubfunc(hashresref) = 0 THEN
                IF constdefined(hashresref) THEN
                    hashfound = 1
                    EXIT DO
                END IF
            END IF
            IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
        LOOP
        IF hashfound THEN
            i2 = hashresref
            t = consttype(i2)
            IF t AND ISSTRING THEN Give_Error "Expected STRING * numeric-constant": EXIT FUNCTION
            'convert value to general formats
            IF t AND ISFLOAT THEN
                v## = constfloat(i2)
                v&& = v##
                v~&& = v&&
            ELSE
                IF t AND ISUNSIGNED THEN
                    v~&& = constuinteger(i2)
                    v&& = v~&&
                    v## = v&&
                ELSE
                    v&& = constinteger(i2)
                    v## = v&&
                    v~&& = v&&
                END IF
            END IF
            IF v&& < 1 OR v&& > 9999999999 THEN Give_Error "STRING * out-of-range constant": EXIT FUNCTION
            bytes = v&&
            dim2typepassback$ = SCase$("String * ") + constcname(i2)
            GOTO constantlenstr
        END IF

        IF isuinteger(c$) = 0 THEN Give_Error "Number/Constant expected after *": EXIT FUNCTION
        IF LEN(c$) > 10 THEN Give_Error "Too many characters in number after *": EXIT FUNCTION
        bytes = VAL(c$)
        IF bytes = 0 THEN Give_Error "Cannot create a fixed string of length 0": EXIT FUNCTION
        constantlenstr:
        n$ = "STRING" + str2(bytes) + "_" + varname$

        'array of fixed length strings
        IF elements$ <> "" THEN
            arraydesc = 0
            IF f = 1 THEN
                try = findid(varname$ + "$")
                IF Error_Happened THEN EXIT FUNCTION
                DO WHILE try
                    IF (id.arraytype) THEN
                        l$ = RTRIM$(id.cn)
                        arraydesc = currentid: scope2$ = scope$
                        EXIT DO
                    END IF
                    IF try = 2 THEN findanotherid = 1: try = findid(varname$ + "$") ELSE try = 0
                    IF Error_Happened THEN EXIT FUNCTION
                LOOP
            END IF
            n$ = scope2$ + "ARRAY_" + n$

            'nume = allocarray(n$, elements$, bytes)
            'IF arraydesc THEN goto dim2exitfunc 'id already exists!
            'clearid

            IF f = 1 THEN

                IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
                E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
            END IF
            nume = allocarray(n$, elements$, bytes, 0)
            IF Error_Happened THEN EXIT FUNCTION
            l$ = l$ + sp + tlayout$
            IF arraydesc THEN GOTO dim2exitfunc
            clearid

        ELSE
            clearid
            IF elements$ = "?" THEN
                nume = -1
                id.linkid = glinkid
                id.linkarg = glinkarg
            ELSE
                nume = VAL(elements$)
            END IF
        END IF

        id.arraytype = STRINGTYPE + ISFIXEDLENGTH
        IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
        id.n = cvarname$
        IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

        id.arrayelements = nume
        id.callname = n$
        id.tsize = bytes
        IF method = 0 THEN
            id.mayhave = "$" + str2(bytes)
        END IF
        IF method = 1 THEN
            id.musthave = "$" + str2(bytes)
        END IF
        regid
        IF Error_Happened THEN EXIT FUNCTION
        vWatchVariable n$, 0
        GOTO dim2exitfunc
    END IF

    'standard fixed length string
    n$ = scope2$ + n$
    IF f THEN PRINT #defdatahandle, "qbs *" + n$ + "=NULL;"
    IF f THEN PRINT #19, "qbs_free(" + n$ + ");" 'so descriptor can be freed
    clearid
    id.n = cvarname$
    id.t = STRINGTYPE + ISFIXEDLENGTH
    IF cmemlist(idn + 1) THEN
        id.t = id.t + ISINCONVENTIONALMEMORY
        IF f THEN PRINT #13, "if(" + n$ + "==NULL){"
        IF f THEN PRINT #13, "cmem_sp-=" + str2(bytes) + ";"
        IF f THEN PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
        IF f THEN PRINT #13, n$ + "=qbs_new_fixed((uint8*)(dblock+cmem_sp)," + str2(bytes) + ",0);"
        IF f THEN PRINT #13, "memset(" + n$ + "->chr,0," + str2(bytes) + ");"
        IF f THEN PRINT #13, "}"
    ELSE
        IF f THEN PRINT #13, "if(" + n$ + "==NULL){"
        o$ = "(uint8*)mem_static_malloc(" + str2$(bytes) + ")"
        IF f THEN PRINT #13, n$ + "=qbs_new_fixed(" + o$ + "," + str2$(bytes) + ",0);"
        IF f THEN PRINT #13, "memset(" + n$ + "->chr,0," + str2$(bytes) + ");"
        IF f THEN PRINT #13, "}"
    END IF
    id.tsize = bytes
    IF method = 0 THEN
        id.mayhave = "$" + str2(bytes)
    END IF
    IF method = 1 THEN
        id.musthave = "$" + str2(bytes)
    END IF
    id.callname = n$
    regid
    vWatchVariable n$, 0
    IF Error_Happened THEN EXIT FUNCTION
    GOTO dim2exitfunc
END IF

'variable length string processing
n$ = "STRING_" + varname$

'array of variable length strings
IF elements$ <> "" THEN
    arraydesc = 0
    IF f = 1 THEN
        try = findid(varname$ + "$")
        IF Error_Happened THEN EXIT FUNCTION
        DO WHILE try
            IF (id.arraytype) THEN
                l$ = RTRIM$(id.cn)
                arraydesc = currentid: scope2$ = scope$
                EXIT DO
            END IF
            IF try = 2 THEN findanotherid = 1: try = findid(varname$ + "$") ELSE try = 0
            IF Error_Happened THEN EXIT FUNCTION
        LOOP
    END IF
    n$ = scope2$ + "ARRAY_" + n$

    'nume = allocarray(n$, elements$, -2147483647) '-2147483647=STRING
    'IF arraydesc THEN goto dim2exitfunc 'id already exists!
    'clearid

    IF f = 1 THEN

        IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
        E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
    END IF
    nume = allocarray(n$, elements$, -2147483647, 0)
    IF Error_Happened THEN EXIT FUNCTION
    l$ = l$ + sp + tlayout$
    IF arraydesc THEN GOTO dim2exitfunc
    clearid

ELSE
    clearid
    IF elements$ = "?" THEN
        nume = -1
        id.linkid = glinkid
        id.linkarg = glinkarg
    ELSE
        nume = VAL(elements$)
    END IF
END IF

id.n = cvarname$
id.arraytype = STRINGTYPE
IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

id.arrayelements = nume
id.callname = n$
IF method = 0 THEN
    id.mayhave = "$"
END IF
IF method = 1 THEN
    id.musthave = "$"
END IF
regid
IF Error_Happened THEN EXIT FUNCTION
vWatchVariable n$, 0
GOTO dim2exitfunc
END IF

'standard variable length string
n$ = scope2$ + n$
clearid
id.n = cvarname$
id.t = STRINGTYPE
IF cmemlist(idn + 1) THEN
    IF f THEN PRINT #defdatahandle, "qbs *" + n$ + "=NULL;"
    IF f THEN PRINT #13, "if (!" + n$ + ")" + n$ + "=qbs_new_cmem(0,0);"
    id.t = id.t + ISINCONVENTIONALMEMORY
ELSE
    IF f THEN PRINT #defdatahandle, "qbs *" + n$ + "=NULL;"
    IF f THEN PRINT #13, "if (!" + n$ + ")" + n$ + "=qbs_new(0,0);"
END IF
IF f THEN PRINT #19, "qbs_free(" + n$ + ");"
IF method = 0 THEN
    id.mayhave = "$"
END IF
IF method = 1 THEN
    id.musthave = "$"
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF

IF LEFT$(typ$, 4) = "_BIT" OR (LEFT$(typ$, 3) = "BIT" AND qbnexprefix_set = 1) THEN
    IF (LEFT$(typ$, 4) = "_BIT" AND LEN(typ$) > 4) OR (LEFT$(typ$, 3) = "BIT" AND LEN(typ$) > 3) THEN
        IF LEFT$(typ$, 7) <> "_BIT * " AND LEFT$(typ$, 6) <> "BIT * " THEN Give_Error "Expected " + qbnexprefix$ + "BIT * number": EXIT FUNCTION
        c$ = MID$(typ$, INSTR(typ$, " * ") + 3)
        IF isuinteger(c$) = 0 THEN Give_Error "Number expected after *": EXIT FUNCTION
        IF LEN(c$) > 2 THEN Give_Error "Cannot create a bit variable of size > 64 bits": EXIT FUNCTION
        bits = VAL(c$)
        IF bits = 0 THEN Give_Error "Cannot create a bit variable of size 0 bits": EXIT FUNCTION
        IF bits > 64 THEN Give_Error "Cannot create a bit variable of size > 64 bits": EXIT FUNCTION
    ELSE
        bits = 1
    END IF
    IF bits <= 32 THEN ct$ = "int32" ELSE ct$ = "int64"
    IF unsgn THEN n$ = "U": ct$ = "u" + ct$
    n$ = n$ + "BIT" + str2(bits) + "_" + varname$

    'array of bit-length variables
    IF elements$ <> "" THEN
        IF bits > 63 THEN Give_Error "Cannot create a bit array of size > 63 bits": EXIT FUNCTION
        arraydesc = 0
        cmps$ = varname$: IF unsgn THEN cmps$ = cmps$ + "~"
        cmps$ = cmps$ + "`" + str2(bits)
        IF f = 1 THEN
            try = findid(cmps$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try
                IF (id.arraytype) THEN
                    l$ = RTRIM$(id.cn)
                    arraydesc = currentid: scope2$ = scope$
                    EXIT DO
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(cmps$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP
        END IF
        n$ = scope2$ + "ARRAY_" + n$

        'nume = allocarray(n$, elements$, -bits) 'passing a negative element size signifies bits not bytes
        'IF arraydesc THEN goto dim2exitfunc 'id already exists!
        'clearid

        IF f = 1 THEN

            IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
            E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
        END IF
        nume = allocarray(n$, elements$, -bits, 0)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + sp + tlayout$
        IF arraydesc THEN GOTO dim2exitfunc
        clearid

    ELSE
        clearid
        IF elements$ = "?" THEN
            nume = -1
            id.linkid = glinkid
            id.linkarg = glinkarg
        ELSE
            nume = VAL(elements$)
        END IF
    END IF

    id.n = cvarname$
    id.arraytype = BITTYPE - 1 + bits
    IF unsgn THEN id.arraytype = id.arraytype + ISUNSIGNED
    IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
    IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

    id.arrayelements = nume
    id.callname = n$
    IF method = 0 THEN
        IF unsgn THEN id.mayhave = "~`" + str2(bits) ELSE id.mayhave = "`" + str2(bits)
    END IF
    IF method = 1 THEN
        IF unsgn THEN id.musthave = "~`" + str2(bits) ELSE id.musthave = "`" + str2(bits)
    END IF
    regid
    IF Error_Happened THEN EXIT FUNCTION
    vWatchVariable n$, 0
    GOTO dim2exitfunc
END IF
'standard bit-length variable
n$ = scope2$ + n$
PRINT #defdatahandle, ct$ + " *" + n$ + "=NULL;"
PRINT #13, "if(" + n$ + "==NULL){"
PRINT #13, "cmem_sp-=4;"
PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
PRINT #13, n$ + "=(" + ct$ + "*)(dblock+cmem_sp);"
PRINT #13, "*" + n$ + "=0;"
PRINT #13, "}"
clearid
id.n = cvarname$
id.t = BITTYPE - 1 + bits + ISINCONVENTIONALMEMORY: IF unsgn THEN id.t = id.t + ISUNSIGNED
IF method = 0 THEN
    IF unsgn THEN id.mayhave = "~`" + str2(bits) ELSE id.mayhave = "`" + str2(bits)
END IF
IF method = 1 THEN
    IF unsgn THEN id.musthave = "~`" + str2(bits) ELSE id.musthave = "`" + str2(bits)
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF

IF typ$ = "_BYTE" OR (typ$ = "BYTE" AND qbnexprefix_set = 1) THEN
    ct$ = "int8"
    IF unsgn THEN n$ = "U": ct$ = "u" + ct$
    n$ = n$ + "BYTE_" + varname$
    IF elements$ <> "" THEN
        arraydesc = 0
        cmps$ = varname$: IF unsgn THEN cmps$ = cmps$ + "~"
        cmps$ = cmps$ + "%%"
        IF f = 1 THEN
            try = findid(cmps$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try
                IF (id.arraytype) THEN
                    l$ = RTRIM$(id.cn)
                    arraydesc = currentid: scope2$ = scope$
                    EXIT DO
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(cmps$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP

        END IF
        n$ = scope2$ + "ARRAY_" + n$

        'nume = allocarray(n$, elements$, 1)
        'IF arraydesc THEN goto dim2exitfunc
        'clearid

        IF f = 1 THEN

            IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
            E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
        END IF
        nume = allocarray(n$, elements$, 1, 0)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + sp + tlayout$
        IF arraydesc THEN GOTO dim2exitfunc
        clearid

    ELSE
        clearid
        IF elements$ = "?" THEN
            nume = -1
            id.linkid = glinkid
            id.linkarg = glinkarg
        ELSE
            nume = VAL(elements$)
        END IF
    END IF

    id.arraytype = BYTETYPE: IF unsgn THEN id.arraytype = id.arraytype + ISUNSIGNED
    IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
    IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

    id.arrayelements = nume
    id.callname = n$
ELSE
    n$ = scope2$ + n$
    clearid
    id.t = BYTETYPE: IF unsgn THEN id.t = id.t + ISUNSIGNED
    IF f = 1 THEN PRINT #defdatahandle, ct$ + " *" + n$ + "=NULL;"
    IF f = 1 THEN PRINT #13, "if(" + n$ + "==NULL){"
    IF cmemlist(idn + 1) THEN
        id.t = id.t + ISINCONVENTIONALMEMORY
        IF f = 1 THEN PRINT #13, "cmem_sp-=1;"
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)(dblock+cmem_sp);"
        IF f = 1 THEN PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
    ELSE
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)mem_static_malloc(1);"
    END IF
    IF f = 1 THEN PRINT #13, "*" + n$ + "=0;"
    IF f = 1 THEN PRINT #13, "}"
END IF
id.n = cvarname$
IF method = 0 THEN
    IF unsgn THEN id.mayhave = "~%%" ELSE id.mayhave = "%%"
END IF
IF method = 1 THEN
    IF unsgn THEN id.musthave = "~%%" ELSE id.musthave = "%%"
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF

IF typ$ = "INTEGER" THEN
    ct$ = "int16"
    IF unsgn THEN n$ = "U": ct$ = "u" + ct$
    n$ = n$ + "INTEGER_" + varname$

    IF elements$ <> "" THEN
        arraydesc = 0
        cmps$ = varname$: IF unsgn THEN cmps$ = cmps$ + "~"
        cmps$ = cmps$ + "%"
        IF f = 1 THEN
            try = findid(cmps$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try
                IF (id.arraytype) THEN
                    l$ = RTRIM$(id.cn)
                    arraydesc = currentid: scope2$ = scope$
                    EXIT DO
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(cmps$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP
        END IF
        n$ = scope2$ + "ARRAY_" + n$

        IF f = 1 THEN

            IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
            E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
        END IF
        nume = allocarray(n$, elements$, 2, 0)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + sp + tlayout$
        IF arraydesc THEN GOTO dim2exitfunc
        clearid

    ELSE
        clearid
        IF elements$ = "?" THEN
            nume = -1
            id.linkid = glinkid
            id.linkarg = glinkarg
        ELSE
            nume = VAL(elements$)
        END IF
    END IF


    id.arraytype = INTEGERTYPE: IF unsgn THEN id.arraytype = id.arraytype + ISUNSIGNED
    IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
    IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

    id.arrayelements = nume
    id.callname = n$
ELSE
    n$ = scope2$ + n$
    clearid
    id.t = INTEGERTYPE: IF unsgn THEN id.t = id.t + ISUNSIGNED
    IF f = 1 THEN PRINT #defdatahandle, ct$ + " *" + n$ + "=NULL;"
    IF f = 1 THEN PRINT #13, "if(" + n$ + "==NULL){"
    IF cmemlist(idn + 1) THEN
        id.t = id.t + ISINCONVENTIONALMEMORY
        IF f = 1 THEN PRINT #13, "cmem_sp-=2;"
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)(dblock+cmem_sp);"
        IF f = 1 THEN PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
    ELSE
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)mem_static_malloc(2);"
    END IF
    IF f = 1 THEN PRINT #13, "*" + n$ + "=0;"
    IF f = 1 THEN PRINT #13, "}"
END IF
id.n = cvarname$
IF method = 0 THEN
    IF unsgn THEN id.mayhave = "~%" ELSE id.mayhave = "%"
END IF
IF method = 1 THEN
    IF unsgn THEN id.musthave = "~%" ELSE id.musthave = "%"
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF








IF typ$ = "_OFFSET" OR (typ$ = "OFFSET" AND qbnexprefix_set = 1) THEN
    ct$ = "ptrszint"
    IF unsgn THEN n$ = "U": ct$ = "u" + ct$
    n$ = n$ + "OFFSET_" + varname$
    IF elements$ <> "" THEN
        arraydesc = 0
        cmps$ = varname$: IF unsgn THEN cmps$ = cmps$ + "~"
        cmps$ = cmps$ + "%&"
        IF f = 1 THEN
            try = findid(cmps$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try
                IF (id.arraytype) THEN
                    l$ = RTRIM$(id.cn)
                    arraydesc = currentid: scope2$ = scope$
                    EXIT DO
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(cmps$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP
        END IF
        n$ = scope2$ + "ARRAY_" + n$

        IF f = 1 THEN

            IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
            E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
        END IF
        nume = allocarray(n$, elements$, OS_BITS \ 8, 0)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + sp + tlayout$
        IF arraydesc THEN GOTO dim2exitfunc
        clearid

    ELSE
        clearid
        IF elements$ = "?" THEN
            nume = -1
            id.linkid = glinkid
            id.linkarg = glinkarg
        ELSE
            nume = VAL(elements$)
        END IF
    END IF

    id.arraytype = OFFSETTYPE: IF unsgn THEN id.arraytype = id.arraytype + ISUNSIGNED
    IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
    IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

    id.arrayelements = nume
    id.callname = n$
ELSE
    n$ = scope2$ + n$
    clearid
    id.t = OFFSETTYPE: IF unsgn THEN id.t = id.t + ISUNSIGNED
    IF f = 1 THEN PRINT #defdatahandle, ct$ + " *" + n$ + "=NULL;"
    IF f = 1 THEN PRINT #13, "if(" + n$ + "==NULL){"
    IF cmemlist(idn + 1) THEN
        id.t = id.t + ISINCONVENTIONALMEMORY
        IF f = 1 THEN PRINT #13, "cmem_sp-=" + str2(OS_BITS \ 8) + ";"
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)(dblock+cmem_sp);"
        IF f = 1 THEN PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
    ELSE
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)mem_static_malloc(" + str2(OS_BITS \ 8) + ");"
    END IF
    IF f = 1 THEN PRINT #13, "*" + n$ + "=0;"
    IF f = 1 THEN PRINT #13, "}"
END IF
id.n = cvarname$
IF method = 0 THEN
    IF unsgn THEN id.mayhave = "~%&" ELSE id.mayhave = "%&"
END IF
IF method = 1 THEN
    IF unsgn THEN id.musthave = "~%&" ELSE id.musthave = "%&"
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF

IF typ$ = "LONG" THEN
    ct$ = "int32"
    IF unsgn THEN n$ = "U": ct$ = "u" + ct$
    n$ = n$ + "LONG_" + varname$
    IF elements$ <> "" THEN
        arraydesc = 0
        cmps$ = varname$: IF unsgn THEN cmps$ = cmps$ + "~"
        cmps$ = cmps$ + "&"
        IF f = 1 THEN
            try = findid(cmps$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try
                IF (id.arraytype) THEN
                    l$ = RTRIM$(id.cn)
                    arraydesc = currentid: scope2$ = scope$
                    EXIT DO
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(cmps$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP
        END IF
        n$ = scope2$ + "ARRAY_" + n$

        'nume = allocarray(n$, elements$, 4)
        'IF arraydesc THEN goto dim2exitfunc
        'clearid

        IF f = 1 THEN

            IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
            E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
        END IF
        nume = allocarray(n$, elements$, 4, 0)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + sp + tlayout$
        IF arraydesc THEN GOTO dim2exitfunc
        clearid

    ELSE
        clearid
        IF elements$ = "?" THEN
            nume = -1
            id.linkid = glinkid
            id.linkarg = glinkarg
        ELSE
            nume = VAL(elements$)
        END IF
    END IF

    id.arraytype = LONGTYPE: IF unsgn THEN id.arraytype = id.arraytype + ISUNSIGNED
    IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
    IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

    id.arrayelements = nume
    id.callname = n$
ELSE
    n$ = scope2$ + n$
    clearid
    id.t = LONGTYPE: IF unsgn THEN id.t = id.t + ISUNSIGNED
    IF f = 1 THEN PRINT #defdatahandle, ct$ + " *" + n$ + "=NULL;"
    IF f = 1 THEN PRINT #13, "if(" + n$ + "==NULL){"
    IF cmemlist(idn + 1) THEN
        id.t = id.t + ISINCONVENTIONALMEMORY
        IF f = 1 THEN PRINT #13, "cmem_sp-=4;"
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)(dblock+cmem_sp);"
        IF f = 1 THEN PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
    ELSE
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)mem_static_malloc(4);"
    END IF
    IF f = 1 THEN PRINT #13, "*" + n$ + "=0;"
    IF f = 1 THEN PRINT #13, "}"
END IF
id.n = cvarname$
IF method = 0 THEN
    IF unsgn THEN id.mayhave = "~&" ELSE id.mayhave = "&"
END IF
IF method = 1 THEN
    IF unsgn THEN id.musthave = "~&" ELSE id.musthave = "&"
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF

IF typ$ = "_INTEGER64" OR (typ$ = "INTEGER64" AND qbnexprefix_set = 1) THEN
    ct$ = "int64"
    IF unsgn THEN n$ = "U": ct$ = "u" + ct$
    n$ = n$ + "INTEGER64_" + varname$
    IF elements$ <> "" THEN
        arraydesc = 0
        cmps$ = varname$: IF unsgn THEN cmps$ = cmps$ + "~"
        cmps$ = cmps$ + "&&"
        IF f = 1 THEN
            try = findid(cmps$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try
                IF (id.arraytype) THEN
                    l$ = RTRIM$(id.cn)
                    arraydesc = currentid: scope2$ = scope$
                    EXIT DO
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(cmps$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP
        END IF
        n$ = scope2$ + "ARRAY_" + n$

        'nume = allocarray(n$, elements$, 8)
        'IF arraydesc THEN goto dim2exitfunc
        'clearid

        IF f = 1 THEN

            IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
            E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
        END IF
        nume = allocarray(n$, elements$, 8, 0)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + sp + tlayout$
        IF arraydesc THEN GOTO dim2exitfunc
        clearid

    ELSE
        clearid
        IF elements$ = "?" THEN
            nume = -1
            id.linkid = glinkid
            id.linkarg = glinkarg
        ELSE
            nume = VAL(elements$)
        END IF
    END IF

    id.arraytype = INTEGER64TYPE: IF unsgn THEN id.arraytype = id.arraytype + ISUNSIGNED
    IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
    IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

    id.arrayelements = nume
    id.callname = n$
ELSE
    n$ = scope2$ + n$
    clearid
    id.t = INTEGER64TYPE: IF unsgn THEN id.t = id.t + ISUNSIGNED
    IF f = 1 THEN PRINT #defdatahandle, ct$ + " *" + n$ + "=NULL;"
    IF f = 1 THEN PRINT #13, "if(" + n$ + "==NULL){"
    IF cmemlist(idn + 1) THEN
        id.t = id.t + ISINCONVENTIONALMEMORY
        IF f = 1 THEN PRINT #13, "cmem_sp-=8;"
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)(dblock+cmem_sp);"
        IF f = 1 THEN PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
    ELSE
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)mem_static_malloc(8);"
    END IF
    IF f = 1 THEN PRINT #13, "*" + n$ + "=0;"
    IF f = 1 THEN PRINT #13, "}"
END IF
id.n = cvarname$
IF method = 0 THEN
    IF unsgn THEN id.mayhave = "~&&" ELSE id.mayhave = "&&"
END IF
IF method = 1 THEN
    IF unsgn THEN id.musthave = "~&&" ELSE id.musthave = "&&"
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF

IF unsgn = 1 THEN Give_Error "Type cannot be unsigned": EXIT FUNCTION

IF typ$ = "SINGLE" THEN
    ct$ = "float"
    n$ = n$ + "SINGLE_" + varname$
    IF elements$ <> "" THEN
        arraydesc = 0
        cmps$ = varname$ + "!"
        IF f = 1 THEN
            try = findid(cmps$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try
                IF (id.arraytype) THEN
                    l$ = RTRIM$(id.cn)
                    arraydesc = currentid: scope2$ = scope$
                    EXIT DO
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(cmps$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP
        END IF
        n$ = scope2$ + "ARRAY_" + n$

        'nume = allocarray(n$, elements$, 4)
        'IF arraydesc THEN goto dim2exitfunc
        'clearid

        IF f = 1 THEN

            IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
            E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
        END IF
        nume = allocarray(n$, elements$, 4, 0)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + sp + tlayout$
        IF arraydesc THEN GOTO dim2exitfunc
        clearid

    ELSE
        clearid
        IF elements$ = "?" THEN
            nume = -1
            id.linkid = glinkid
            id.linkarg = glinkarg
        ELSE
            nume = VAL(elements$)
        END IF
    END IF

    id.arraytype = SINGLETYPE
    IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
    IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

    id.arrayelements = nume
    id.callname = n$
ELSE
    n$ = scope2$ + n$
    clearid
    id.t = SINGLETYPE
    IF f = 1 THEN PRINT #defdatahandle, ct$ + " *" + n$ + "=NULL;"
    IF f = 1 THEN PRINT #13, "if(" + n$ + "==NULL){"
    IF cmemlist(idn + 1) THEN
        id.t = id.t + ISINCONVENTIONALMEMORY
        IF f = 1 THEN PRINT #13, "cmem_sp-=4;"
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)(dblock+cmem_sp);"
        IF f = 1 THEN PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
    ELSE
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)mem_static_malloc(4);"
    END IF
    IF f = 1 THEN PRINT #13, "*" + n$ + "=0;"
    IF f = 1 THEN PRINT #13, "}"
END IF
id.n = cvarname$
IF method = 0 THEN
    id.mayhave = "!"
END IF
IF method = 1 THEN
    id.musthave = "!"
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF

IF typ$ = "DOUBLE" THEN
    ct$ = "double"
    n$ = n$ + "DOUBLE_" + varname$
    IF elements$ <> "" THEN
        arraydesc = 0
        cmps$ = varname$ + "#"
        IF f = 1 THEN
            try = findid(cmps$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try
                IF (id.arraytype) THEN
                    l$ = RTRIM$(id.cn)
                    arraydesc = currentid: scope2$ = scope$
                    EXIT DO
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(cmps$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP
        END IF
        n$ = scope2$ + "ARRAY_" + n$

        'nume = allocarray(n$, elements$, 8)
        'IF arraydesc THEN goto dim2exitfunc
        'clearid

        IF f = 1 THEN

            IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
            E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
        END IF
        nume = allocarray(n$, elements$, 8, 0)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + sp + tlayout$
        IF arraydesc THEN GOTO dim2exitfunc
        clearid

    ELSE
        clearid
        IF elements$ = "?" THEN
            nume = -1
            id.linkid = glinkid
            id.linkarg = glinkarg
        ELSE
            nume = VAL(elements$)
        END IF
    END IF

    id.arraytype = DOUBLETYPE
    IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
    IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

    id.arrayelements = nume
    id.callname = n$
ELSE
    n$ = scope2$ + n$
    clearid
    id.t = DOUBLETYPE
    IF f = 1 THEN PRINT #defdatahandle, ct$ + " *" + n$ + "=NULL;"
    IF f = 1 THEN PRINT #13, "if(" + n$ + "==NULL){"
    IF cmemlist(idn + 1) THEN
        id.t = id.t + ISINCONVENTIONALMEMORY
        IF f = 1 THEN PRINT #13, "cmem_sp-=8;"
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)(dblock+cmem_sp);"
        IF f = 1 THEN PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
    ELSE
        IF f = 1 THEN PRINT #13, n$ + "=(" + ct$ + "*)mem_static_malloc(8);"
    END IF
    IF f = 1 THEN PRINT #13, "*" + n$ + "=0;"
    IF f = 1 THEN PRINT #13, "}"
END IF
id.n = cvarname$
IF method = 0 THEN
    id.mayhave = "#"
END IF
IF method = 1 THEN
    id.musthave = "#"
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF

IF typ$ = "_FLOAT" OR (typ$ = "FLOAT" AND qbnexprefix_set = 1) THEN
    ct$ = "long double"
    n$ = n$ + "FLOAT_" + varname$
    IF elements$ <> "" THEN
        arraydesc = 0
        cmps$ = varname$ + "##"
        IF f = 1 THEN
            try = findid(cmps$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try
                IF (id.arraytype) THEN
                    l$ = RTRIM$(id.cn)
                    arraydesc = currentid: scope2$ = scope$
                    EXIT DO
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(cmps$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP
        END IF
        n$ = scope2$ + "ARRAY_" + n$

        'nume = allocarray(n$, elements$, 32)
        'IF arraydesc THEN goto dim2exitfunc
        'clearid

        IF f = 1 THEN

            IF LEN(elements$) = 1 AND ASC(elements$) = 63 THEN '"?"
            E = arrayelementslist(idn + 1): IF E THEN elements$ = elements$ + str2$(E) 'eg. "?3" for a 3 dimensional array
        END IF
        nume = allocarray(n$, elements$, 32, 0)
        IF Error_Happened THEN EXIT FUNCTION
        l$ = l$ + sp + tlayout$
        IF arraydesc THEN GOTO dim2exitfunc
        clearid

    ELSE
        clearid
        IF elements$ = "?" THEN
            nume = -1
            id.linkid = glinkid
            id.linkarg = glinkarg
        ELSE
            nume = VAL(elements$)
        END IF
    END IF

    id.arraytype = FLOATTYPE
    IF cmemlist(idn + 1) THEN id.arraytype = id.arraytype + ISINCONVENTIONALMEMORY
    IF nume > 65536 THEN nume = nume - 65536: id.staticarray = 1

    id.arrayelements = nume
    id.callname = n$
ELSE
    n$ = scope2$ + n$
    clearid
    id.t = FLOATTYPE
    IF f THEN PRINT #defdatahandle, ct$ + " *" + n$ + "=NULL;"
    IF f THEN PRINT #13, "if(" + n$ + "==NULL){"
    IF cmemlist(idn + 1) THEN
        id.t = id.t + ISINCONVENTIONALMEMORY
        IF f THEN PRINT #13, "cmem_sp-=32;"
        IF f THEN PRINT #13, n$ + "=(" + ct$ + "*)(dblock+cmem_sp);"
        IF f THEN PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
    ELSE
        IF f THEN PRINT #13, n$ + "=(" + ct$ + "*)mem_static_malloc(32);"
    END IF
    IF f THEN PRINT #13, "*" + n$ + "=0;"
    IF f THEN PRINT #13, "}"
END IF
id.n = cvarname$
IF method = 0 THEN
    id.mayhave = "##"
END IF
IF method = 1 THEN
    id.musthave = "##"
END IF
id.callname = n$
regid
vWatchVariable n$, 0
IF Error_Happened THEN EXIT FUNCTION
GOTO dim2exitfunc
END IF

Give_Error "Unknown type": EXIT FUNCTION
dim2exitfunc:

bypassNextVariable = 0

IF dimsfarray THEN
    ids(idn).sfid = glinkid
    ids(idn).sfarg = glinkarg
END IF

'restore STATIC state
IF dimstatic <> 0 AND dimshared = 0 THEN
    defdatahandle = 13
    CLOSE #13: OPEN tmpdir$ + "data" + str2$(subfuncn) + ".txt" FOR APPEND AS #13
    CLOSE #19: OPEN tmpdir$ + "free" + str2$(subfuncn) + ".txt" FOR APPEND AS #19
END IF

tlayout$ = l$

END FUNCTION

FUNCTION variablesize$ (i AS LONG) 'ID or -1 (if ID already 'loaded')
    'Note: assumes whole bytes, no bit offsets/sizes
    IF i <> -1 THEN getid i
    IF Error_Happened THEN EXIT FUNCTION
    'find base size from type
    t = id.t: IF t = 0 THEN t = id.arraytype
    bytes = (t AND 511) \ 8

    IF t AND ISUDT THEN 'correct size for UDTs
    u = t AND 511
    bytes = udtxsize(u) \ 8
END IF

IF t AND ISSTRING THEN 'correct size for strings
IF t AND ISFIXEDLENGTH THEN
    bytes = id.tsize
ELSE
    IF id.arraytype THEN Give_Error "Cannot determine size of variable-length string array": EXIT FUNCTION
    variablesize$ = scope$ + "STRING_" + RTRIM$(id.n) + "->len"
    EXIT FUNCTION
END IF
END IF

IF id.arraytype THEN 'multiply size for arrays
n$ = RTRIM$(id.callname)
s$ = str2(bytes) + "*(" + n$ + "[2]&1)" 'note: multiplying by 0 if array not currently defined (affects dynamic arrays)
arrayelements = id.arrayelements: IF arrayelements = -1 THEN arrayelements = 1 '2009
FOR i2 = 1 TO arrayelements
    s$ = s$ + "*" + n$ + "[" + str2(i2 * 4 - 4 + 5) + "]"
NEXT
variablesize$ = "(" + s$ + ")"
EXIT FUNCTION
END IF

variablesize$ = str2(bytes)
END FUNCTION

SUB regid
    idn = idn + 1

    IF idn > ids_max THEN
        ids_max = ids_max * 2
        REDIM _PRESERVE ids(1 TO ids_max) AS idstruct
        REDIM _PRESERVE cmemlist(1 TO ids_max + 1) AS INTEGER
        REDIM _PRESERVE sfcmemargs(1 TO ids_max + 1) AS STRING * 100
        REDIM _PRESERVE arrayelementslist(1 TO ids_max + 1) AS INTEGER
    END IF

    n$ = RTRIM$(id.n)

    IF reginternalsubfunc = 0 THEN
        IF validname(n$) = 0 THEN Give_Error "Invalid name": EXIT SUB
    END IF

    'register case sensitive name if none given
    IF ASC(id.cn) = 32 THEN
        n$ = RTRIM$(id.n)
        id.n = UCASE$(n$)
        id.cn = n$
    END IF

    id.insubfunc = subfunc
    id.insubfuncn = subfuncn

    'note: cannot be STATIC and SHARED at the same time
    IF dimshared THEN
        id.share = dimshared
    ELSE
        IF dimstatic THEN id.staticscope = 1
    END IF

    ids(idn) = id

    currentid = idn

    'prepare hash flags and check for conflicts
    hashflags = 1

    'sub/function?
    'Note: QBASIC does not allow: Internal type names (INTEGER,LONG,...)
    IF id.subfunc THEN
        ids(currentid).internal_subfunc = reginternalsubfunc
        IF id.subfunc = 1 THEN hashflags = hashflags + HASHFLAG_FUNCTION ELSE hashflags = hashflags + HASHFLAG_SUB
        IF reginternalsubfunc = 0 THEN 'allow internal definition of subs/functions without checks
        hashchkflags = HASHFLAG_RESERVED + HASHFLAG_CONSTANT
        IF id.subfunc = 1 THEN hashchkflags = hashchkflags + HASHFLAG_FUNCTION ELSE hashchkflags = hashchkflags + HASHFLAG_SUB
        hashres = HashFind(n$, hashchkflags, hashresflags, hashresref)
        DO WHILE hashres
            IF hashres THEN
                'Note: Numeric sub/function names like 'mid' do not clash with Internal string sub/function names
                '      like 'MID$' because MID$ always requires a '$'. For user defined string sub/function names
                '      the '$' would be optional so the rule should not be applied there.
                allow = 0
                IF hashresflags AND (HASHFLAG_FUNCTION + HASHFLAG_SUB) THEN
                    IF RTRIM$(ids(hashresref).musthave) = "$" THEN
                        IF INSTR(ids(currentid).mayhave, "$") = 0 THEN allow = 1
                    END IF
                END IF
                IF allow = 0 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
            END IF 'hashres
            IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
        LOOP
    END IF 'reginternalsubfunc = 0
END IF

'variable?
IF id.t THEN
    hashflags = hashflags + HASHFLAG_VARIABLE
    IF reginternalvariable = 0 THEN
        allow = 0
        var_recheck:
        IF ASC(id.musthave) = 32 THEN astype2 = 1 '"AS type" declaration?
        scope2 = subfuncn
        hashchkflags = HASHFLAG_RESERVED + HASHFLAG_SUB + HASHFLAG_FUNCTION + HASHFLAG_CONSTANT + HASHFLAG_VARIABLE
        hashres = HashFind(n$, hashchkflags, hashresflags, hashresref)
        DO WHILE hashres

            'conflict with reserved word?
            IF hashresflags AND HASHFLAG_RESERVED THEN
                musthave$ = RTRIM$(id.musthave)
                IF INSTR(musthave$, "$") THEN
                    'All reserved words can be used as variables in QBASIC if "$" is appended to the variable name!
                    '(allow)
                ELSE
                    Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'Conflicts with reserved word
                END IF
            END IF 'HASHFLAG_RESERVED

            'conflict with sub/function?
            IF hashresflags AND (HASHFLAG_FUNCTION + HASHFLAG_SUB) THEN
                IF ids(hashresref).internal_subfunc = 0 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'QBASIC doesn't allow a variable of the same name as a user-defined sub/func
                IF RTRIM$(id.n) = "WIDTH" AND ids(hashresref).subfunc = 2 THEN GOTO varname_exception
                musthave$ = RTRIM$(id.musthave)
                IF LEN(musthave$) = 0 THEN
                    IF RTRIM$(ids(hashresref).musthave) = "$" THEN
                        'a sub/func requiring "$" can co-exist with implicit numeric variables
                        IF INSTR(id.mayhave, "$") THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
                    ELSE
                        Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'Implicitly defined variables cannot conflict with sub/func names
                    END IF
                END IF 'len(musthave$)=0
                IF INSTR(musthave$, "$") THEN
                    IF RTRIM$(ids(hashresref).musthave) = "$" THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'A sub/function name already exists as a string
                    '(allow)
                ELSE
                    IF RTRIM$(ids(hashresref).musthave) <> "$" THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'A non-"$" sub/func name already exists with this name
                END IF
            END IF 'HASHFLAG_FUNCTION + HASHFLAG_SUB

            'conflict with constant?
            IF hashresflags AND HASHFLAG_CONSTANT THEN
                scope1 = constsubfunc(hashresref)
                IF (scope1 = 0 AND AllowLocalName = 0) OR scope1 = scope2 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
            END IF

            'conflict with variable?
            IF hashresflags AND HASHFLAG_VARIABLE THEN
                astype1 = 0: IF ASC(ids(hashresref).musthave) = 32 THEN astype1 = 1
                scope1 = ids(hashresref).insubfuncn
                IF astype1 = 1 AND astype2 = 1 THEN
                    IF scope1 = scope2 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
                END IF
                'same type?
                IF id.t = ids(hashresref).t THEN
                    IF id.tsize = ids(hashresref).tsize THEN
                        IF scope1 = scope2 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
                    END IF
                END IF
                'will astype'd fixed STRING-variable mask a non-fixed string?
                IF id.t AND ISFIXEDLENGTH THEN
                    IF astype2 = 1 THEN
                        IF ids(hashresref).t AND ISSTRING THEN
                            IF (ids(hashresref).t AND ISFIXEDLENGTH) = 0 THEN
                                IF scope1 = scope2 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
                            END IF
                        END IF
                    END IF
                END IF
            END IF

            varname_exception:
            IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
        LOOP
    END IF 'reginternalvariable
END IF 'variable

'array?
IF id.arraytype THEN
    hashflags = hashflags + HASHFLAG_ARRAY
    allow = 0
    ary_recheck:
    scope2 = subfuncn
    IF ASC(id.musthave) = 32 THEN astype2 = 1 '"AS type" declaration?
    hashchkflags = HASHFLAG_RESERVED + HASHFLAG_SUB + HASHFLAG_FUNCTION + HASHFLAG_ARRAY
    hashres = HashFind(n$, hashchkflags, hashresflags, hashresref)
    DO WHILE hashres

        'conflict with reserved word?
        IF hashresflags AND HASHFLAG_RESERVED THEN
            musthave$ = RTRIM$(id.musthave)
            IF INSTR(musthave$, "$") THEN
                'All reserved words can be used as variables in QBASIC if "$" is appended to the variable name!
                '(allow)
            ELSE
                Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'Conflicts with reserved word
            END IF
        END IF 'HASHFLAG_RESERVED

        'conflict with sub/function?
        IF hashresflags AND (HASHFLAG_FUNCTION + HASHFLAG_SUB) THEN
            IF ids(hashresref).internal_subfunc = 0 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'QBASIC doesn't allow a variable of the same name as a user-defined sub/func
            IF RTRIM$(id.n) = "WIDTH" AND ids(hashresref).subfunc = 2 THEN GOTO arrayname_exception
            musthave$ = RTRIM$(id.musthave)

            IF LEN(musthave$) = 0 THEN
                IF RTRIM$(ids(hashresref).musthave) = "$" THEN
                    'a sub/func requiring "$" can co-exist with implicit numeric variables
                    IF INSTR(id.mayhave, "$") THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
                ELSE
                    Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'Implicitly defined variables cannot conflict with sub/func names
                END IF
            END IF 'len(musthave$)=0
            IF INSTR(musthave$, "$") THEN
                IF RTRIM$(ids(hashresref).musthave) = "$" THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'A sub/function name already exists as a string
                '(allow)
            ELSE
                IF RTRIM$(ids(hashresref).musthave) <> "$" THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB 'A non-"$" sub/func name already exists with this name
            END IF
        END IF 'HASHFLAG_FUNCTION + HASHFLAG_SUB

        'conflict with array?
        IF hashresflags AND HASHFLAG_ARRAY THEN
            astype1 = 0: IF ASC(ids(hashresref).musthave) = 32 THEN astype1 = 1
            scope1 = ids(hashresref).insubfuncn
            IF astype1 = 1 AND astype2 = 1 THEN
                IF scope1 = scope2 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
            END IF
            'same type?
            IF id.arraytype = ids(hashresref).arraytype THEN
                IF id.tsize = ids(hashresref).tsize THEN
                    IF scope1 = scope2 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
                END IF
            END IF
            'will astype'd fixed STRING-variable mask a non-fixed string?
            IF id.arraytype AND ISFIXEDLENGTH THEN
                IF astype2 = 1 THEN
                    IF ids(hashresref).arraytype AND ISSTRING THEN
                        IF (ids(hashresref).arraytype AND ISFIXEDLENGTH) = 0 THEN
                            IF scope1 = scope2 THEN Give_Error "Name already in use (" + n$ + ")": EXIT SUB
                        END IF
                    END IF
                END IF
            END IF
        END IF

        arrayname_exception:
        IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
    LOOP
END IF 'array

'add it to the hash table
HashAdd n$, hashflags, currentid

END SUB
