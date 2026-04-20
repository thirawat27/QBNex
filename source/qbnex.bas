'All variables will be of type LONG unless explicitly defined
DEFLNG A-Z

'All arrays will be dynamically allocated so they can be REDIM-ed
'$DYNAMIC

'We need console access to support command-line compilation via the -x command line compile option
$CONSOLE

'Initially the "SCREEN" will be hidden, if the -x option is used it will never be created
$SCREENHIDE

$EXEICON:'./qbnex.ico'

$VERSIONINFO:FILEVERSION#=1,0,0,0
$VERSIONINFO:PRODUCTVERSION#=1,0,0,0
$VERSIONINFO:CompanyName=thirawat27
$VERSIONINFO:FileDescription=QBNex CLI Compiler
$VERSIONINFO:FileVersion=1.0.0
$VERSIONINFO:InternalName=qb.exe
$VERSIONINFO:LegalCopyright=Copyright (c) 2026 thirawat27
$VERSIONINFO:LegalTrademarks=QBNex
$VERSIONINFO:OriginalFilename=qb.exe
$VERSIONINFO:ProductName=QBNex
$VERSIONINFO:ProductVersion=1.0.0
$VERSIONINFO:Comments=QBNex IS a modern extended BASIC programming language that retains QB4.5/QBasic compatibility AND compiles native binaries FOR Windows, Linux AND macOS.
$VERSIONINFO:Web=https://github.com/thirawat27/QBNex

' Error handler declarations and bootstrap globals must stay above later implementation includes.
'$INCLUDE:'includes\bootstrap.bas'

DEFLNG A-Z

'-------- Optional layout component (1/2) --------

DIM SHARED OName(1000) AS STRING 'Operation name
DIM SHARED PL(1000) AS INTEGER 'Priority level
DIM SHARED PP_TypeMod(0) AS STRING, PP_ConvertedMod(0) AS STRING 'Prepass Name Conversion variables.
Set_OrderOfOperations

DIM SHARED vWatchOn, vWatchRecompileAttempts, vWatchDesiredState, vWatchErrorCall$
DIM SHARED vWatchNewVariable$, vWatchVariableExclusions$
vWatchErrorCall$ = "if (stop_program) {*__LONG_VWATCH_LINENUMBER=0; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);};if(new_error){bkp_new_error=new_error;new_error=0;*__LONG_VWATCH_LINENUMBER=-1; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);new_error=bkp_new_error;};"
vWatchVariableExclusions$ = "@__LONG_VWATCH_LINENUMBER@__LONG_VWATCH_SUBLEVEL@__LONG_VWATCH_GOTO@@__STRING_VWATCH_SUBNAME@__STRING_VWATCH_CALLSTACK@__ARRAY_BYTE_VWATCH_BREAKPOINTS@__ARRAY_BYTE_VWATCH_SKIPLINES@__STRING_VWATCH_INTERNALSUBNAME@__ARRAY_STRING_VWATCH_STACK@"

DIM SHARED nativeDataTypes$
nativeDataTypes$ = "@_OFFSET@OFFSET@_UNSIGNED _OFFSET@UNSIGNED OFFSET@_BIT@BIT@_UNSIGNED _BIT@UNSIGNED BIT@_BYTE@_UNSIGNED _BYTE@BYTE@UNSIGNED BYTE@INTEGER@_UNSIGNED INTEGER@UNSIGNED INTEGER@LONG@_UNSIGNED LONG@UNSIGNED LONG@_INTEGER64@INTEGER64@_UNSIGNED _INTEGER64@UNSIGNED INTEGER64@SINGLE@DOUBLE@_FLOAT@FLOAT@STRING@"

DIM SHARED qbnexprefix_set_recompileAttempts, qbnexprefix_set_desiredState
DIM SHARED opex_recompileAttempts, opex_desiredState
DIM SHARED opexarray_recompileAttempts, opexarray_desiredState

REDIM EveryCaseSet(100), SelectCaseCounter AS _UNSIGNED LONG
REDIM SelectCaseHasCaseBlock(100)
DIM ExecLevel(255), ExecCounter AS INTEGER
DIM SHARED UserDefineName(0 TO 1000) AS STRING
DIM SHARED UserDefineValue(0 TO 1000) AS STRING
DIM SHARED InValidLine(10000) AS _BYTE
DIM DefineElse(255) AS _BYTE
DIM SHARED UserDefineCount AS INTEGER, UserDefineList$
UserDefineList$ = "@DEFINED@UNDEFINED@WINDOWS@WIN@LINUX@MAC@MACOSX@32BIT@64BIT@VERSION@"
UserDefineName$(0) = "WINDOWS": UserDefineName$(1) = "WIN"
UserDefineName$(2) = "LINUX"
UserDefineName$(3) = "MAC": UserDefineName$(4) = "MACOSX"
UserDefineName$(5) = "32BIT": UserDefineName$(6) = "64BIT"
UserDefineName$(7) = "VERSION"
IF INSTR(_OS$, "WIN") THEN UserDefineValue$(0) = "-1": UserDefineValue$(1) = "-1" ELSE UserDefineValue$(0) = "0": UserDefineValue$(1) = "0"
IF INSTR(_OS$, "LINUX") THEN UserDefineValue$(2) = "-1" ELSE UserDefineValue$(2) = "0"
IF INSTR(_OS$, "MAC") THEN UserDefineValue$(3) = "-1": UserDefineValue$(4) = "-1" ELSE UserDefineValue$(3) = "0": UserDefineValue$(4) = "0"
IF INSTR(_OS$, "32BIT") THEN UserDefineValue$(5) = "-1": UserDefineValue$(6) = "0" ELSE UserDefineValue$(5) = "0": UserDefineValue$(6) = "-1"
UserDefineValue$(7) = Version$

InitCompilerServices
VerifyInternalFolderOrExit

DIM SHARED Include_GDB_Debugging_Info 'set using "options.bin"

DIM SHARED DEPENDENCY_LAST
CONST DEPENDENCY_LOADFONT = 1: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_AUDIO_CONVERSION = 2: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_AUDIO_DECODE = 3: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_AUDIO_OUT = 4: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_GL = 5: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_IMAGE_CODEC = 6: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_CONSOLE_ONLY = 7: DEPENDENCY_LAST = DEPENDENCY_LAST + 1 '=2 if via -g switch, =1 if via metacommand $CONSOLE:ONLY
CONST DEPENDENCY_SOCKETS = 8: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_PRINTER = 9: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_ICON = 10: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_SCREENIMAGE = 11: DEPENDENCY_LAST = DEPENDENCY_LAST + 1
CONST DEPENDENCY_DEVICEINPUT = 12: DEPENDENCY_LAST = DEPENDENCY_LAST + 1 'removes support for gamepad input if not present
CONST DEPENDENCY_ZLIB = 13: DEPENDENCY_LAST = DEPENDENCY_LAST + 1 'ZLIB library linkage, if desired, for compression/decompression.

DIM SHARED DEPENDENCY(1 TO DEPENDENCY_LAST)

DIM SHARED UseGL 'declared SUB _GL (no params)
InitPlatformDefaults

TYPE usedVarList
    AS LONG id, linenumber, includeLevel, includedLine, scope, localIndex
    AS LONG arrayElementSize
    AS _BYTE used, watch, isarray, displayFormat 'displayFormat: 0=DEC;1=HEX;2=BIN;3=OCT
    AS STRING NAME, cname, varType, includedFile, subfunc
    AS STRING watchRange, indexes, elements, elementTypes 'for Arrays and UDTs
    AS STRING elementOffset, storage
END TYPE

DIM SHARED typeDefinitions$
DIM SHARED totalVariablesCreated AS LONG, totalMainVariablesCreated AS LONG
DIM SHARED bypassNextVariable AS _BYTE
DIM SHARED totalWarnings AS LONG, warningListItems AS LONG
DIM SHARED maxLineNumber AS LONG
DIM SHARED ExeIconSet AS LONG, qbnexprefix$, qbnexprefix_set
DIM SHARED VersionInfoSet AS _BYTE

'Variables to handle $VERSIONINFO metacommand:
DIM SHARED viFileVersionNum$, viProductVersionNum$, viCompanyName$
DIM SHARED viFileDescription$, viFileVersion$, viInternalName$
DIM SHARED viLegalCopyright$, viLegalTrademarks$, viOriginalFilename$
DIM SHARED viProductName$, viProductVersion$, viComments$, viWeb$

DIM SHARED NoChecks

DIM SHARED Console
DIM SHARED ScreenHide
DIM SHARED Asserts
DIM SHARED OptMax AS LONG
OptMax = 256
DIM SHARED Opt(1 TO OptMax, 1 TO 10) AS STRING * 256
'   (1,1)="READ"
'   (1,2)="WRITE"
'   (1,3)="READ WRITE"
DIM SHARED OptWords(1 TO OptMax, 1 TO 10) AS INTEGER 'The number of words of each opt () element
'   (1,1)=1 '"READ"
'   (1,2)=1 '"WRITE"
'   (1,3)=2 '"READ WRITE"
DIM SHARED T(1 TO OptMax) AS INTEGER 'The type of the entry
'   t is 0 for ? opts
'   ---------- 0 means ? , 1+ means a symbol or {}block ----------
'   t is 1 for symbol opts
'   t is the number of rhs opt () index enteries for {READ|WRITE|READ WRITE} like opts
DIM SHARED Lev(1 TO OptMax) AS INTEGER 'The indwelling level of each opt () element (the lowest is 0)
DIM SHARED EntryLev(1 TO OptMax) AS INTEGER 'The level required from which this opt () can be validly be entered/checked-for
DIM SHARED DitchLev(1 TO OptMax) AS INTEGER 'The lowest level recorded between the previous Opt and this Opt
DIM SHARED DontPass(1 TO OptMax) AS INTEGER 'Set to 1 or 0, with 1 meaning don't pass
'Determines whether the opt () entry needs to actually be passed to the C++ sub/function
DIM SHARED TempList(1 TO OptMax) AS INTEGER
DIM SHARED PassRule(1 TO OptMax) AS LONG
'0 means no pass rule
'negative values refer to an opt () element
'positive values refer to a flag value
DIM SHARED LevelEntered(OptMax) 'up to 64 levels supported
DIM SHARED separgs(OptMax + 1) AS STRING
DIM SHARED separgslayout(OptMax + 1) AS STRING
DIM SHARED separgs_local(OptMax + 1) AS STRING
DIM SHARED separgslayout_local(OptMax + 1) AS STRING
DIM SHARED E
DIM SHARED ResolveStaticFunctions
DIM SHARED ResolveStaticFunction_File(1 TO 100) AS STRING
DIM SHARED ResolveStaticFunction_Name(1 TO 100) AS STRING
DIM SHARED ResolveStaticFunction_Method(1 TO 100) AS LONG
DIM SHARED Error_Happened AS LONG
DIM SHARED Error_Message AS STRING
DIM SHARED FrontendErrorHandled AS _BYTE
DIM SHARED LastFrontendErrorKey AS STRING

BATCHFILE_EXTENSION = ".bat"
IF os$ = "LNX" THEN BATCHFILE_EXTENSION = ".sh"
IF MacOSX THEN BATCHFILE_EXTENSION = ".command"


DIM inlinedatastr(255) AS STRING
FOR i = 0 TO 255
    inlinedatastr(i) = str2$(i) + ","
NEXT


extension$ = ".exe"
IF os$ = "LNX" THEN extension$ = "" 'no extension under Linux

pathsep$ = "\"
IF os$ = "LNX" THEN pathsep$ = "/"
'note: QBNex handles OS specific path separators automatically except under SHELL calls

ON ERROR GOTO qberror_test

IF os$ = "WIN" THEN tmpdir$ = ".\internal\temp\": tmpdir2$ = "..\\temp\\"
IF os$ = "LNX" THEN tmpdir$ = "./internal/temp/": tmpdir2$ = "../temp/"

IF NOT _DIREXISTS(tmpdir$) THEN MKDIR tmpdir$

DECLARE LIBRARY
FUNCTION getpid& ()
    END DECLARE

    thisinstancepid = getpid&
    IF os$ = "LNX" THEN
        fh = FREEFILE
        OPEN ".\internal\temp\tempfoldersearch.bin" FOR RANDOM AS #fh LEN = LEN(tempfolderindex)
        tempfolderrecords = LOF(fh) / LEN(tempfolderindex)
        i = 1
        IF tempfolderrecords = 0 THEN
            'first run ever?
            PUT #fh, 1, thisinstancepid
        ELSE
            FOR i = 1 TO tempfolderrecords
                'check if any of the temp folders is being used = pid still active
                GET #fh, i, tempfoldersearch

                SHELL _HIDE "ps -p " + STR$(tempfoldersearch) + " > /dev/null 2>&1; echo $? > internal/temp/checkpid.bin"
                fh2 = FREEFILE
                OPEN "internal/temp/checkpid.bin" FOR BINARY AS #fh2
                LINE INPUT #fh2, checkpid$
                CLOSE #fh2
                IF VAL(checkpid$) = 1 THEN
                    'This temp folder was locked by an instance that's no longer active, so
                    'this will be our temp folder
                    PUT #fh, i, thisinstancepid
                    EXIT FOR
                END IF
            NEXT
            IF i > tempfolderrecords THEN
                'All indexes were busy. Let's initiate a new one:
                PUT #fh, i, thisinstancepid
            END IF
        END IF
        CLOSE #fh
        IF i > 1 THEN
            tmpdir$ = "./internal/temp" + str2$(i) + "/": tmpdir2$ = "../temp" + str2$(i) + "/"
            IF _DIREXISTS(tmpdir$) = 0 THEN
                MKDIR tmpdir$
            END IF
        END IF
        OPEN tmpdir$ + "temp.bin" FOR OUTPUT LOCK WRITE AS #26
    ELSE
        ON ERROR GOTO qberror_test
        E = 0
        i = 1
        OPEN tmpdir$ + "temp.bin" FOR OUTPUT LOCK WRITE AS #26
        DO WHILE E
            i = i + 1
            IF i = 1000 THEN PRINT "Unable to locate the 'internal' folder": END 1
            MKDIR ".\internal\temp" + str2$(i)
            IF os$ = "WIN" THEN tmpdir$ = ".\internal\temp" + str2$(i) + "\": tmpdir2$ = "..\\temp" + str2$(i) + "\\"
            IF os$ = "LNX" THEN tmpdir$ = "./internal/temp" + str2$(i) + "/": tmpdir2$ = "../temp" + str2$(i) + "/"
            E = 0
            OPEN tmpdir$ + "temp.bin" FOR OUTPUT LOCK WRITE AS #26
        LOOP
    END IF


    'temp folder established
    tempfolderindex = i
    IF i > 1 THEN
        'create modified version of qbx.cpp
        OPEN ".\internal\c\qbx" + str2$(i) + ".cpp" FOR OUTPUT AS #2
        OPEN ".\internal\c\qbx.cpp" FOR BINARY AS #1
        DO UNTIL EOF(1)
            LINE INPUT #1, a$
            x = INSTR(a$, "..\\temp\\"): IF x THEN a$ = LEFT$(a$, x - 1) + "..\\temp" + str2$(i) + "\\" + RIGHT$(a$, LEN(a$) - (x + 9))
            x = INSTR(a$, "../temp/"): IF x THEN a$ = LEFT$(a$, x - 1) + "../temp" + str2$(i) + "/" + RIGHT$(a$, LEN(a$) - (x + 7))
            PRINT #2, a$
        LOOP
        CLOSE #1, #2
    END IF

    IF Debug THEN OPEN tmpdir$ + "debug.txt" FOR OUTPUT AS #9

    ON ERROR GOTO qberror

    'Appended to generated temp-file names when multiple compiler instances run.
    IF tempfolderindex <> 1 THEN tempfolderindexstr$ = "(" + str2$(tempfolderindex) + ")": tempfolderindexstr2$ = str2$(tempfolderindex)


    DIM SHARED compilerdebuginfo
    DIM SHARED seperateargs_error
    DIM SHARED seperateargs_error_message AS STRING

    DIM SHARED reginternalsubfunc
    DIM SHARED reginternalvariable


    DIM SHARED symboltype_size
    symboltype_size = 0

    DIM SHARED use_global_byte_elements
    use_global_byte_elements = 0

    DIM SHARED optionexplicit AS _BYTE
    DIM SHARED optionexplicitarray AS _BYTE
    DIM SHARED optionexplicit_cmd AS _BYTE
    DIM SHARED errorLineInInclude AS LONG

    '$INCLUDE:'global\compiler_settings.bas'

    CMDLineFile = ParseCMDLineArgs$
    IF CMDLineFile <> "" AND FileHasExtension(CMDLineFile) = 0 THEN
        CMDLineFile = CMDLineFile + ".BAS"
    END IF
    IF CMDLineFile <> "" AND _FILEEXISTS(_STARTDIR$ + "/" + CMDLineFile) THEN
        CMDLineFile = _STARTDIR$ + "/" + CMDLineFile
    END IF

    ConsoleMode = -1
    _DEST _CONSOLE

    'Hash table layout. Keep this near the top-level declarations because the
    'hash arrays are shared across parser, semantic, and build phases.
    TYPE HashListItem
        Flags AS LONG
        Reference AS LONG
        NextItem AS LONG
        PrevItem AS LONG
        LastItem AS LONG 'note: this value is only valid on the first item in the list
        'note: name is stored in a seperate array of strings
    END TYPE
    DIM SHARED HashFind_NextListItem AS LONG
    DIM SHARED HashFind_Reverse AS LONG
    DIM SHARED HashFind_SearchFlags AS LONG
    DIM SHARED HashFind_Name AS STRING
    DIM SHARED HashListSize AS LONG
    DIM SHARED HashListNext AS LONG
    DIM SHARED HashListFreeSize AS LONG
    DIM SHARED HashListFreeLast AS LONG
    'hash lookup tables
    DIM SHARED hash1char(255) AS INTEGER
    DIM SHARED hash2char(65535) AS INTEGER
    FOR x = 1 TO 26
        hash1char(64 + x) = x
        hash1char(96 + x) = x
    NEXT
    hash1char(95) = 27 '_
    hash1char(48) = 28 '0
    hash1char(49) = 29 '1
    hash1char(50) = 30 '2
    hash1char(51) = 31 '3
    hash1char(52) = 23 '4 'note: x, y, z and beginning alphabet letters avoided because of common usage (eg. a2, y3)
    hash1char(53) = 22 '5
    hash1char(54) = 20 '6
    hash1char(55) = 19 '7
    hash1char(56) = 18 '8
    hash1char(57) = 17 '9
    FOR c1 = 0 TO 255
        FOR c2 = 0 TO 255
            hash2char(c1 + c2 * 256) = hash1char(c1) + hash1char(c2) * 32
        NEXT
    NEXT
    'Use a compact hash table and chaining rather than the historical oversized
    'fixed table. This keeps memory usage predictable without changing behavior.
    CONST HASH_TABLE_SIZE = 65536 '2^16 entries for better cache locality
    CONST HASH_TABLE_MASK = 65535 'For fast modulo using AND
    HashListSize = 65536
    HashListNext = 1
    HashListFreeSize = 1024
    HashListFreeLast = 0
    DIM SHARED HashList(1 TO HashListSize) AS HashListItem
    DIM SHARED HashListName(1 TO HashListSize) AS STRING * 256
    DIM SHARED HashListFree(1 TO HashListFreeSize) AS LONG
    DIM SHARED HashTable(0 TO HASH_TABLE_MASK) AS LONG '256KB lookup table with chaining for collisions

    CONST HASHFLAG_LABEL = 2
    CONST HASHFLAG_TYPE = 4
    CONST HASHFLAG_RESERVED = 8
    CONST HASHFLAG_OPERATOR = 16
    CONST HASHFLAG_CUSTOMSYNTAX = 32
    CONST HASHFLAG_SUB = 64
    CONST HASHFLAG_FUNCTION = 128
    CONST HASHFLAG_UDT = 256
    CONST HASHFLAG_UDTELEMENT = 512
    CONST HASHFLAG_CONSTANT = 1024
    CONST HASHFLAG_VARIABLE = 2048
    CONST HASHFLAG_ARRAY = 4096
    CONST HASHFLAG_XELEMENTNAME = 8192
    CONST HASHFLAG_XTYPENAME = 16384

    TYPE Label_Type
        State AS _UNSIGNED _BYTE '0=label referenced, 1=label created
        cn AS STRING * 256
        Scope AS LONG
        Data_Offset AS _INTEGER64 'offset within data
        Data_Referenced AS _UNSIGNED _BYTE 'set to 1 if data is referenced (data_offset will be used to create the data offset variable)
        Error_Line AS LONG 'the line number to reference on errors
        Scope_Restriction AS LONG 'cannot exist inside this scope (post checked)
        SourceLineNumber AS LONG
    END TYPE
    DIM SHARED nLabels, Labels_Ubound
    Labels_Ubound = 100
    DIM SHARED Labels(1 TO Labels_Ubound) AS Label_Type
    DIM SHARED Empty_Label AS Label_Type

    DIM SHARED PossibleSubNameLabels AS STRING 'format: name+sp+name+sp+name <-ucase$'d
    DIM SHARED SubNameLabels AS STRING 'format: name+sp+name+sp+name <-ucase$'d
    DIM SHARED CreatingLabel AS LONG

    DIM SHARED AllowLocalName AS LONG

    DIM SHARED DataOffset

    DIM SHARED prepass


    DIM SHARED autoarray

    DIM SHARED ontimerid, onkeyid, onstrigid

    DIM SHARED revertmaymusthave(1 TO 10000)
    DIM SHARED revertmaymusthaven

    DIM SHARED linecontinuation

    DIM SHARED dim2typepassback AS STRING 'passes back correct case sensitive version of type


    DIM SHARED inclevel
    DIM SHARED incname(100) AS STRING 'must be full path as given
    DIM SHARED inclinenumber(100) AS LONG
    DIM SHARED incerror AS STRING


    DIM SHARED fix046 AS STRING
    fix046$ = "__" + "ASCII" + "_" + "CHR" + "_" + "046" + "__" 'broken up to avoid detection for layout reversion

    DIM SHARED layout AS STRING 'layout text for tooling
    DIM SHARED layoutok AS LONG 'tracks status of entire line

    DIM SHARED layoutcomment AS STRING

    DIM SHARED tlayout AS STRING 'temporary layout string set by supporting functions
    DIM SHARED layoutdone AS LONG 'tracks status of single command


    DIM SHARED fooindwel

    DIM SHARED alphanumeric(255)
    FOR i = 48 TO 57
        alphanumeric(i) = -1
    NEXT
    FOR i = 65 TO 90
        alphanumeric(i) = -1
    NEXT
    FOR i = 97 TO 122
        alphanumeric(i) = -1
    NEXT
    '_ is treated as an alphabet letter
    alphanumeric(95) = -1

    DIM SHARED isalpha(255)
    FOR i = 65 TO 90
        isalpha(i) = -1
    NEXT
    FOR i = 97 TO 122
        isalpha(i) = -1
    NEXT
    '_ is treated as an alphabet letter
    isalpha(95) = -1

    DIM SHARED isnumeric(255)
    FOR i = 48 TO 57
        isnumeric(i) = -1
    NEXT


    DIM SHARED lfsinglechar(255)
    lfsinglechar(40) = 1 '(
    lfsinglechar(41) = 1 ')
    lfsinglechar(42) = 1 '*
    lfsinglechar(43) = 1 '+
    lfsinglechar(45) = 1 '-
    lfsinglechar(47) = 1 '/
    lfsinglechar(60) = 1 '<
    lfsinglechar(61) = 1 '=
    lfsinglechar(62) = 1 '>
    lfsinglechar(92) = 1 '\
    lfsinglechar(94) = 1 '^

    lfsinglechar(44) = 1 ',
    lfsinglechar(46) = 1 '.
    lfsinglechar(58) = 1 ':
    lfsinglechar(59) = 1 ';

    lfsinglechar(35) = 1 '# (file no only)
    lfsinglechar(36) = 1 '$ (metacommand only)
    lfsinglechar(63) = 1 '? (print macro)
    lfsinglechar(95) = 1 '_










    DIM SHARED nextrunlineindex AS LONG

    DIM SHARED lineinput3buffer AS STRING
    DIM SHARED lineinput3index AS LONG
    DIM SHARED classSyntaxQueue AS STRING
    DIM SHARED classSyntaxDeferredQueue AS STRING
    DIM SHARED classSyntaxActive AS LONG
    DIM SHARED classSyntaxTypeOpen AS LONG
    DIM SHARED classSyntaxInMethod AS LONG
    DIM SHARED classSyntaxHelperEmitted AS LONG
    DIM SHARED classSyntaxClassName AS STRING
    DIM SHARED classSyntaxBaseName AS STRING
    DIM SHARED classSyntaxInterfaces AS STRING
    DIM SHARED classSyntaxMethodKind AS STRING
    DIM SHARED classSyntaxMethodAlias AS STRING
    DIM SHARED classSyntaxGeneratedName AS STRING
    DIM SHARED classSyntaxOwnFieldLines AS STRING
    DIM SHARED classSyntaxRegistryCount AS LONG
    DIM SHARED classSyntaxRegistryName(1 TO 256) AS STRING
    DIM SHARED classSyntaxRegistryBase(1 TO 256) AS STRING
    DIM SHARED classSyntaxRegistryOwnFields(1 TO 256) AS STRING
    DIM SHARED classSyntaxRegistryFlatFields(1 TO 256) AS STRING
    DIM SHARED classSyntaxRegistryMethods(1 TO 256) AS STRING
    DIM SHARED classSyntaxScopeDepth AS LONG
    DIM SHARED classSyntaxScopeVars(0 TO 63) AS STRING

    DIM SHARED dimstatic AS LONG

    DIM SHARED staticarraylist AS STRING
    DIM SHARED staticarraylistn AS LONG
    DIM SHARED commonarraylist AS STRING
    DIM SHARED commonarraylistn AS LONG

    'CONST support
    DIM SHARED constmax AS LONG
    constmax = 100
    DIM SHARED constlast AS LONG
    constlast = -1
    DIM SHARED constname(constmax) AS STRING
    DIM SHARED constcname(constmax) AS STRING
    DIM SHARED constnamesymbol(constmax) AS STRING 'optional name symbol
    ' `1 and `no-number must be handled correctly
    'DIM SHARED constlastshared AS LONG 'so any defined inside a sub/function after this index can be "forgotten" when sub/function exits
    'constlastshared = -1
    DIM SHARED consttype(constmax) AS LONG 'variable type number
    'consttype determines storage
    DIM SHARED constinteger(constmax) AS _INTEGER64
    DIM SHARED constuinteger(constmax) AS _UNSIGNED _INTEGER64
    DIM SHARED constfloat(constmax) AS _FLOAT
    DIM SHARED conststring(constmax) AS STRING
    DIM SHARED constsubfunc(constmax) AS LONG
    DIM SHARED constdefined(constmax) AS LONG

    'UDT
    'names
    DIM SHARED lasttype AS LONG
    DIM SHARED lasttypeelement AS LONG

    TYPE idstruct

        n AS STRING * 256 'name
        cn AS STRING * 256 'case sensitive version of n

        arraytype AS LONG 'similar to t
        arrayelements AS INTEGER
        staticarray AS INTEGER 'set for arrays declared in the main module with static elements

        mayhave AS STRING * 8 'mayhave and musthave are exclusive of each other
        musthave AS STRING * 8
        t AS LONG 'type

        tsize AS LONG


        subfunc AS INTEGER 'if function=1, sub=2 (max 100 arguments)
        Dependency AS INTEGER
        internal_subfunc AS INTEGER

        callname AS STRING * 256
        ccall AS INTEGER
        overloaded AS _BYTE
        args AS INTEGER
        minargs AS INTEGER
        arg AS STRING * 400 'similar to t
        argsize AS STRING * 400 'similar to tsize (used for fixed length strings)
        specialformat AS STRING * 256
        secondargmustbe AS STRING * 256
        secondargcantbe AS STRING * 256
        ret AS LONG 'the value it returns if it is a function (again like t)

        insubfunc AS STRING * 256
        insubfuncn AS LONG

        share AS INTEGER
        nele AS STRING * 100
        nelereq AS STRING * 100
        linkid AS LONG
        linkarg AS INTEGER
        staticscope AS INTEGER
        'For variables which are arguments passed to a sub/function
        sfid AS LONG 'id number of variable's parent sub/function
        sfarg AS INTEGER 'argument/parameter # within call (1=first)

        hr_syntax AS STRING
    END TYPE

    DIM SHARED id AS idstruct

    DIM SHARED idn AS LONG
    DIM SHARED ids_max AS LONG
    ids_max = 1024
    DIM SHARED ids(1 TO ids_max) AS idstruct
    DIM SHARED cmemlist(1 TO ids_max + 1) AS INTEGER 'variables that must be in cmem
    DIM SHARED sfcmemargs(1 TO ids_max + 1) AS STRING * 100 's/f arg that must be in cmem
    DIM SHARED arrayelementslist(1 TO ids_max + 1) AS INTEGER 'arrayelementslist (like cmemlist) helps to resolve the number of elements in arrays with an unknown number of elements. Note: arrays with an unknown number of elements have .arrayelements=-1


    'create blank id template for idclear to copy (stops strings being set to chr$(0))
    DIM SHARED cleariddata AS idstruct
    cleariddata.cn = ""
    cleariddata.n = ""
    cleariddata.mayhave = ""
    cleariddata.musthave = ""
    cleariddata.callname = ""
    cleariddata.arg = ""
    cleariddata.argsize = ""
    cleariddata.specialformat = ""
    cleariddata.secondargmustbe = ""
    cleariddata.secondargcantbe = ""
    cleariddata.insubfunc = ""
    cleariddata.nele = ""
    cleariddata.nelereq = ""

    DIM SHARED ISSTRING AS LONG
    DIM SHARED ISFLOAT AS LONG
    DIM SHARED ISUNSIGNED AS LONG
    DIM SHARED ISPOINTER AS LONG
    DIM SHARED ISFIXEDLENGTH AS LONG
    DIM SHARED ISINCONVENTIONALMEMORY AS LONG
    DIM SHARED ISOFFSETINBITS AS LONG
    DIM SHARED ISARRAY AS LONG
    DIM SHARED ISREFERENCE AS LONG
    DIM SHARED ISUDT AS LONG
    DIM SHARED ISOFFSET AS LONG

    DIM SHARED STRINGTYPE AS LONG
    DIM SHARED BITTYPE AS LONG
    DIM SHARED UBITTYPE AS LONG
    DIM SHARED BYTETYPE AS LONG
    DIM SHARED UBYTETYPE AS LONG
    DIM SHARED INTEGERTYPE AS LONG
    DIM SHARED UINTEGERTYPE AS LONG
    DIM SHARED LONGTYPE AS LONG
    DIM SHARED ULONGTYPE AS LONG
    DIM SHARED INTEGER64TYPE AS LONG
    DIM SHARED UINTEGER64TYPE AS LONG
    DIM SHARED SINGLETYPE AS LONG
    DIM SHARED DOUBLETYPE AS LONG
    DIM SHARED FLOATTYPE AS LONG
    DIM SHARED OFFSETTYPE AS LONG
    DIM SHARED UOFFSETTYPE AS LONG
    DIM SHARED UDTTYPE AS LONG

    DIM SHARED gosubid AS LONG
    DIM SHARED redimoption AS INTEGER
    DIM SHARED dimoption AS INTEGER
    DIM SHARED arraydesc AS INTEGER
    DIM SHARED qberrorhappened AS INTEGER
    DIM SHARED qberrorcode AS INTEGER
    DIM SHARED qberrorline AS INTEGER
    'COMMON SHARED defineaz() AS STRING
    'COMMON SHARED defineextaz() AS STRING

    DIM SHARED sourcefile AS STRING 'the full path and filename
    DIM SHARED file AS STRING 'name of the file (without .bas or path)

    'COMMON SHARED separgs() AS STRING

    DIM SHARED constequation AS INTEGER
    DIM SHARED DynamicMode AS INTEGER
    DIM SHARED findidsecondarg AS STRING
    DIM SHARED findanotherid AS INTEGER
    DIM SHARED findidinternal AS LONG
    DIM SHARED currentid AS LONG 'is the index of the last ID accessed
    DIM SHARED linenumber AS LONG, reallinenumber AS LONG, totallinenumber AS LONG, definingtypeerror AS LONG
    DIM SHARED wholeline AS STRING
    DIM SHARED diagnosticSourceLine AS STRING
    DIM SHARED firstLineNumberLabelvWatch AS LONG, lastLineNumberLabelvWatch AS LONG
    DIM SHARED vWatchUsedLabels AS STRING, vWatchUsedSkipLabels AS STRING
    DIM SHARED linefragment AS STRING
    'COMMON SHARED bitmask() AS _INTEGER64
    'COMMON SHARED bitmaskinv() AS _INTEGER64

    DIM SHARED arrayprocessinghappened AS INTEGER
    DIM SHARED stringprocessinghappened AS INTEGER
    DIM SHARED cleanupstringprocessingcall AS STRING
    DIM SHARED inputfunctioncalled AS _BYTE
    DIM SHARED recompile AS INTEGER 'forces recompilation
    'COMMON SHARED cmemlist() AS INTEGER
    DIM SHARED optionbase AS INTEGER

    DIM SHARED addmetastatic AS INTEGER
    DIM SHARED addmetadynamic AS INTEGER
    DIM SHARED addmetainclude AS STRING
    DIM SHARED importedModules AS STRING
    DIM SHARED topLevelRuntimeLines AS STRING
    DIM SHARED topLevelRuntimeCallInjected AS LONG
    DIM SHARED topLevelRuntimeFinalized AS LONG
    DIM SHARED topLevelRuntimeProcDepth AS LONG
    DIM SHARED topLevelRuntimeTypeDepth AS LONG
    DIM SHARED topLevelRuntimeDeclareDepth AS LONG

    DIM SHARED closedmain AS INTEGER
    DIM SHARED module AS STRING

    DIM SHARED subfunc AS STRING
    DIM SHARED subfuncn AS LONG
    DIM SHARED closedsubfunc AS _BYTE
    DIM SHARED subfuncid AS LONG

    DIM SHARED defdatahandle AS INTEGER
    DIM SHARED dimsfarray AS INTEGER
    DIM SHARED dimshared AS INTEGER

    'Allows passing of known elements to recompilation
    DIM SHARED sflistn AS INTEGER
    'COMMON SHARED sfidlist() AS LONG
    'COMMON SHARED sfarglist() AS INTEGER
    'COMMON SHARED sfelelist() AS INTEGER
    DIM SHARED glinkid AS LONG
    DIM SHARED glinkarg AS INTEGER
    DIM SHARED typname2typsize AS LONG
    DIM SHARED uniquenumbern AS LONG

    'CLEAR , , 16384


    DIM SHARED bitmask(1 TO 64) AS _INTEGER64
    DIM SHARED bitmaskinv(1 TO 64) AS _INTEGER64

    DIM SHARED defineextaz(1 TO 27) AS STRING
    DIM SHARED defineaz(1 TO 27) AS STRING '27 is an underscore

    ISSTRING = 1073741824
    ISFLOAT = 536870912
    ISUNSIGNED = 268435456
    ISPOINTER = 134217728
    ISFIXEDLENGTH = 67108864 'only set for strings with pointer flag
    ISINCONVENTIONALMEMORY = 33554432
    ISOFFSETINBITS = 16777216
    ISARRAY = 8388608
    ISREFERENCE = 4194304
    ISUDT = 2097152
    ISOFFSET = 1048576

    STRINGTYPE = ISSTRING + ISPOINTER
    BITTYPE = 1& + ISPOINTER + ISOFFSETINBITS
    UBITTYPE = 1& + ISPOINTER + ISUNSIGNED + ISOFFSETINBITS 'QBNex will also support BIT*n, eg. DIM bitarray[10] AS _UNSIGNED _BIT*10
    BYTETYPE = 8& + ISPOINTER
    UBYTETYPE = 8& + ISPOINTER + ISUNSIGNED
    INTEGERTYPE = 16& + ISPOINTER
    UINTEGERTYPE = 16& + ISPOINTER + ISUNSIGNED
    LONGTYPE = 32& + ISPOINTER
    ULONGTYPE = 32& + ISPOINTER + ISUNSIGNED
    INTEGER64TYPE = 64& + ISPOINTER
    UINTEGER64TYPE = 64& + ISPOINTER + ISUNSIGNED
    SINGLETYPE = 32& + ISFLOAT + ISPOINTER
    DOUBLETYPE = 64& + ISFLOAT + ISPOINTER
    FLOATTYPE = 256& + ISFLOAT + ISPOINTER '8-32 bytes
    OFFSETTYPE = 64& + ISOFFSET + ISPOINTER: IF OS_BITS = 32 THEN OFFSETTYPE = 32& + ISOFFSET + ISPOINTER
    UOFFSETTYPE = 64& + ISOFFSET + ISUNSIGNED + ISPOINTER: IF OS_BITS = 32 THEN UOFFSETTYPE = 32& + ISOFFSET + ISUNSIGNED + ISPOINTER
    UDTTYPE = ISUDT + ISPOINTER






    DIM SHARED statementn AS LONG
    DIM SHARED everycasenewcase AS LONG




    DIM SHARED controllevel AS INTEGER '0=not in a control block
    DIM SHARED controltype(1000) AS INTEGER
    '1=IF (awaiting END IF)
    '2=FOR (awaiting NEXT)
    '3=DO (awaiting LOOP [UNTIL|WHILE param])
    '4=DO WHILE/UNTIL (awaiting LOOP)
    '5=WHILE (awaiting WEND)
    '6=$IF (precompiler)
    '10=SELECT CASE qbs (awaiting END SELECT/CASE)
    '11=SELECT CASE int64 (awaiting END SELECT/CASE)
    '12=SELECT CASE uint64 (awaiting END SELECT/CASE)
    '13=SELECT CASE LONG double (awaiting END SELECT/CASE/CASE ELSE)
    '14=SELECT CASE float ...
    '15=SELECT CASE double
    '16=SELECT CASE int32
    '17=SELECT CASE uint32
    '18=CASE (awaiting END SELECT/CASE/CASE ELSE)
    '19=CASE ELSE (awaiting END SELECT)
    '32=SUB/FUNCTION (awaiting END SUB/FUNCTION)
    DIM controlid(1000) AS LONG
    DIM controlvalue(1000) AS LONG
    DIM controlstate(1000) AS INTEGER
    DIM SHARED controlref(1000) AS LONG 'the line number the control was created on





    ON ERROR GOTO qberror

    i2&& = 1
    FOR i&& = 1 TO 64
        bitmask(i&&) = i2&&
        bitmaskinv(i&&) = NOT i2&&
        i2&& = i2&& + 2 ^ i&&
    NEXT

    DIM id2 AS idstruct

    cleanupstringprocessingcall$ = "qbs_cleanup(qbs_tmp_base,"

    DIM SHARED sfidlist(1000) AS LONG
    DIM SHARED sfarglist(1000) AS INTEGER
    DIM SHARED sfelelist(1000) AS INTEGER















    '----------------ripgl.bas--------------------------------------------------------------------------------
    gl_scan_header
    '----------------ripgl.bas--------------------------------------------------------------------------------







    '-----------------------QBNex COMPILER ONCE ONLY SETUP CODE ENDS HERE---------------------------------------

    noide:
    IF CMDLineFile = "" AND (qbnexversionprinted = 0 OR ConsoleMode = 0) AND NOT QuietMode THEN
        qbnexversionprinted = -1
        PRINT "QBNex Compiler V" + Version$
    END IF

    IF CMDLineFile = "" THEN
        PRINT
        PRINT "Usage: qb <file> [switches]"
        PRINT
        PRINT "Commands:"
        PRINT "  -h, --help              Show help"
        PRINT "  -v, --version           Show compiler version"
        PRINT "  -i, --info, --about     Show project information"
        PRINT "  -g, --examples          Show common CLI examples"
        PRINT
        PRINT "Options:"
        PRINT "  <file>                  Source file to load"
        PRINT "  -c                      Compile the source file (default)"
        PRINT "  -o <output file>        Write output executable to <output file>"
        PRINT "  -x                      Compile and output the result to the"
        PRINT "                             console"
        PRINT "  -w                      Show warnings"
        PRINT "  -q                      Quiet mode (does not inhibit warnings or errors)"
        PRINT "  -m                      Do not colorize compiler output (monochrome mode)"
        PRINT "  -d, --verbose-errors    Legacy alias (detailed diagnostics are default)"
        PRINT "  -k, --compact-errors    Use compact diagnostics (hide extra detail notes)"
        PRINT "  -e                      Enable OPTION _EXPLICIT, making variable declaration"
        PRINT "                             mandatory (per-compilation; doesn't affect the"
        PRINT "                             source file or global settings)"
        PRINT "  -s[:switch=true/false]  View/edit compiler settings"
        PRINT "  -p                      Purge all pre-compiled content first"
        PRINT "  -z                      Generate C code without compiling to executable"
        SYSTEM 1
    ELSE
        f$ = CMDLineFile
    END IF

    f$ = LTRIM$(RTRIM$(f$))

    IF FileHasExtension(f$) = 0 THEN f$ = f$ + ".bas"

    sourcefile$ = f$
    CMDLineFile = sourcefile$
    SetCurrentFile sourcefile$
    'derive name from sourcefile
    f$ = RemoveFileExtension$(f$)

    path.exe$ = ""
    currentdir$ = _CWD$
    path.source$ = getfilepath$(sourcefile$)
    IF LEN(path.source$) THEN
        IF _DIREXISTS(path.source$) = 0 THEN
            PRINT
            PRINT "Cannot locate source file: " + sourcefile$
            IF ConsoleMode THEN SYSTEM 1
            END 1
        END IF
        CHDIR path.source$
        path.source$ = _CWD$
        IF RIGHT$(path.source$, 1) <> pathsep$ THEN path.source$ = path.source$ + pathsep$
        CHDIR currentdir$
    END IF
    IF SaveExeWithSource THEN path.exe$ = path.source$
    IF path.exe$ = "" THEN
        IF INSTR(_OS$, "WIN") THEN path.exe$ = "..\..\" ELSE path.exe$ = "../../"
    END IF
    pendingOutputBinary$ = path.exe$ + f$ + extension$

    FOR x = LEN(f$) TO 1 STEP -1
        a$ = MID$(f$, x, 1)
        IF a$ = "/" OR a$ = "\" THEN
            f$ = RIGHT$(f$, LEN(f$) - x)
            EXIT FOR
        END IF
    NEXT
    file$ = f$
    PreparePendingOutputBinary file$

    'if cmemlist(currentid+1)<>0 before calling regid the variable
    'MUST be defined in cmem!

    fullrecompile:

    BU_DEPENDENCY_CONSOLE_ONLY = DEPENDENCY(DEPENDENCY_CONSOLE_ONLY)
    FOR i = 1 TO UBOUND(DEPENDENCY): DEPENDENCY(i) = 0: NEXT
        DEPENDENCY(DEPENDENCY_CONSOLE_ONLY) = BU_DEPENDENCY_CONSOLE_ONLY AND 2 'Restore -g switch if used

        Error_Happened = 0
        FrontendErrorHandled = 0
        LastFrontendErrorKey = ""

        FOR closeall = 1 TO 255: CLOSE closeall: NEXT

            OPEN tmpdir$ + "temp.bin" FOR OUTPUT LOCK WRITE AS #26 'relock

            fh = FREEFILE: OPEN tmpdir$ + "dyninfo.txt" FOR OUTPUT AS #fh: CLOSE #fh

            IF Debug THEN CLOSE #9: OPEN tmpdir$ + "debug.txt" FOR OUTPUT AS #9

            FOR i = 1 TO ids_max + 1
                arrayelementslist(i) = 0
                cmemlist(i) = 0
                sfcmemargs(i) = ""
            NEXT

            'erase cmemlist
            'erase sfcmemargs

            lastunresolved = -1 'first pass
            sflistn = -1 'no entries

            SubNameLabels = sp 'QBNex will perform a repass to resolve sub names used as labels

            vWatchDesiredState = 0
            vWatchRecompileAttempts = 0

            qbnexprefix_set_desiredState = 0
            qbnexprefix_set_recompileAttempts = 0

            opex_desiredState = 0
            opex_recompileAttempts = 0

            opexarray_desiredState = 0
            opexarray_recompileAttempts = 0

            recompile:
            vWatchOn = vWatchDesiredState
            vWatchVariable "", -1 'reset internal variables list

            qbnexprefix_set = qbnexprefix_set_desiredState
            qbnexprefix$ = "_"

            optionexplicit = opex_desiredState
            IF optionexplicit_cmd = -1 THEN optionexplicit = -1
            optionexplicitarray = opexarray_desiredState

            lastLineReturn = 0
            lastLine = 0
            firstLine = 1

            Resize = 0
            Resize_Scale = 0

            UseGL = 0

            Error_Happened = 0
            FrontendErrorHandled = 0
            LastFrontendErrorKey = ""

            HashClear 'clear the hash table

            'add reserved words to hashtable

            f = HASHFLAG_TYPE + HASHFLAG_RESERVED
            HashAdd "_UNSIGNED", f, 0
            HashAdd "_BIT", f, 0
            HashAdd "_BYTE", f, 0
            HashAdd "INTEGER", f, 0
            HashAdd "LONG", f, 0
            HashAdd "_INTEGER64", f, 0
            HashAdd "_OFFSET", f, 0
            HashAdd "SINGLE", f, 0
            HashAdd "DOUBLE", f, 0
            HashAdd "_FLOAT", f, 0
            HashAdd "STRING", f, 0
            HashAdd "ANY", f, 0

            f = HASHFLAG_OPERATOR + HASHFLAG_RESERVED
            HashAdd "NOT", f, 0
            HashAdd "IMP", f, 0
            HashAdd "EQV", f, 0
            HashAdd "AND", f, 0
            HashAdd "OR", f, 0
            HashAdd "XOR", f, 0
            HashAdd "MOD", f, 0

            f = HASHFLAG_RESERVED + HASHFLAG_CUSTOMSYNTAX
            HashAdd "LIST", f, 0
            HashAdd "BASE", f, 0
            HashAdd "_EXPLICIT", f, 0
            HashAdd "AS", f, 0
            HashAdd "IS", f, 0
            HashAdd "OFF", f, 0
            HashAdd "ON", f, 0
            HashAdd "STOP", f, 0
            HashAdd "TO", f, 0
            HashAdd "USING", f, 0
            'PUT(graphics) statement:
            HashAdd "PRESET", f, 0
            HashAdd "PSET", f, 0
            'OPEN statement:
            HashAdd "FOR", f, 0
            HashAdd "OUTPUT", f, 0
            HashAdd "RANDOM", f, 0
            HashAdd "BINARY", f, 0
            HashAdd "APPEND", f, 0
            HashAdd "SHARED", f, 0
            HashAdd "ACCESS", f, 0
            HashAdd "LOCK", f, 0
            HashAdd "READ", f, 0
            HashAdd "WRITE", f, 0
            'LINE statement:
            HashAdd "STEP", f, 0
            'WIDTH statement:
            HashAdd "LPRINT", f, 0
            'VIEW statement:
            HashAdd "PRINT", f, 0

            f = HASHFLAG_RESERVED + HASHFLAG_XELEMENTNAME + HASHFLAG_XTYPENAME
            'A
            'B
            'C
            HashAdd "COMMON", f, 0
            HashAdd "CALL", f, 0
            HashAdd "CASE", f - HASHFLAG_XELEMENTNAME, 0
            HashAdd "COM", f, 0 '(ON...)
            HashAdd "CONST", f, 0
            'D
            HashAdd "DATA", f, 0
            HashAdd "DECLARE", f, 0
            HashAdd "DEF", f, 0
            HashAdd "DEFDBL", f, 0
            HashAdd "DEFINT", f, 0
            HashAdd "DEFLNG", f, 0
            HashAdd "DEFSNG", f, 0
            HashAdd "DEFSTR", f, 0
            HashAdd "DIM", f, 0
            HashAdd "DO", f - HASHFLAG_XELEMENTNAME, 0
            'E
            HashAdd "ERROR", f - HASHFLAG_XELEMENTNAME, 0 '(ON ...)
            HashAdd "ELSE", f, 0
            HashAdd "ELSEIF", f, 0
            HashAdd "ENDIF", f, 0
            HashAdd "EXIT", f - HASHFLAG_XELEMENTNAME, 0
            'F
            HashAdd "FIELD", f - HASHFLAG_XELEMENTNAME, 0
            HashAdd "FUNCTION", f, 0
            'G
            HashAdd "GOSUB", f, 0
            HashAdd "GOTO", f, 0
            'H
            'I
            HashAdd "INPUT", f - HASHFLAG_XELEMENTNAME - HASHFLAG_XTYPENAME, 0 '(INPUT$ function exists, so conflicts if allowed as custom syntax)
            HashAdd "IF", f, 0
            'K
            HashAdd "KEY", f - HASHFLAG_XELEMENTNAME - HASHFLAG_XTYPENAME, 0 '(ON...)
            'L
            HashAdd "LET", f - HASHFLAG_XELEMENTNAME, 0
            HashAdd "LOOP", f - HASHFLAG_XELEMENTNAME, 0
            HashAdd "LEN", f - HASHFLAG_XELEMENTNAME, 0 '(LEN function exists, so conflicts if allowed as custom syntax)
            'M
            'N
            HashAdd "NEXT", f - HASHFLAG_XELEMENTNAME, 0
            'O
            'P
            HashAdd "PLAY", f - HASHFLAG_XELEMENTNAME - HASHFLAG_XTYPENAME, 0 '(ON...)
            HashAdd "PEN", f - HASHFLAG_XELEMENTNAME - HASHFLAG_XTYPENAME, 0 '(ON...)
            'Q
            'R
            HashAdd "REDIM", f, 0
            HashAdd "REM", f, 0
            HashAdd "RESTORE", f - HASHFLAG_XELEMENTNAME, 0
            HashAdd "RESUME", f - HASHFLAG_XELEMENTNAME, 0
            HashAdd "RETURN", f - HASHFLAG_XELEMENTNAME, 0
            HashAdd "RUN", f - HASHFLAG_XELEMENTNAME, 0
            'S
            HashAdd "STATIC", f, 0
            HashAdd "STRIG", f, 0 '(ON...)
            HashAdd "SEG", f, 0
            HashAdd "SELECT", f - HASHFLAG_XELEMENTNAME - HASHFLAG_XTYPENAME, 0
            HashAdd "SUB", f, 0
            HashAdd "SCREEN", f - HASHFLAG_XELEMENTNAME - HASHFLAG_XTYPENAME, 0
            'T
            HashAdd "THEN", f, 0
            HashAdd "TIMER", f - HASHFLAG_XELEMENTNAME - HASHFLAG_XTYPENAME, 0 '(ON...)
            HashAdd "TYPE", f - HASHFLAG_XELEMENTNAME, 0
            'U
            HashAdd "UNTIL", f, 0
            HashAdd "UEVENT", f, 0
            'V
            'W
            HashAdd "WEND", f, 0
            HashAdd "WHILE", f, 0
            'X
            'Y
            'Z







            'clear/init variables
            Console = 0
            ScreenHide = 0
            Asserts = 0
            ResolveStaticFunctions = 0
            dynamiclibrary = 0
            dimsfarray = 0
            dimstatic = 0
            AllowLocalName = 0
            PossibleSubNameLabels = sp 'QBNex will perform a repass to resolve sub names used as labels
            use_global_byte_elements = 0
            dimshared = 0: dimmethod = 0: dimoption = 0: redimoption = 0: commonoption = 0
            mylib$ = "": mylibopt$ = ""
            declaringlibrary = 0
            nLabels = 0
            dynscope = 0
            elsefollowup = 0
            ontimerid = 0: onkeyid = 0: onstrigid = 0
            commonarraylist = "": commonarraylistn = 0
            staticarraylist = "": staticarraylistn = 0
            fooindwel = 0
            layout = ""
            layoutok = 0
            NoChecks = 0
            inclevel = 0
            errorLineInInclude = 0
            addmetainclude$ = ""
            importedModules$ = "@"
            classSyntaxQueue$ = ""
            classSyntaxDeferredQueue$ = ""
            topLevelRuntimeLines = ""
            topLevelRuntimeCallInjected = 0
            topLevelRuntimeFinalized = 0
            topLevelRuntimeProcDepth = 0
            topLevelRuntimeTypeDepth = 0
            topLevelRuntimeDeclareDepth = 0
            classSyntaxActive = 0
            classSyntaxTypeOpen = 0
            classSyntaxInMethod = 0
            classSyntaxHelperEmitted = 0
            classSyntaxClassName$ = ""
            classSyntaxBaseName$ = ""
            classSyntaxInterfaces$ = ""
            classSyntaxMethodKind$ = ""
            classSyntaxMethodAlias$ = ""
            classSyntaxGeneratedName$ = ""
            classSyntaxOwnFieldLines$ = ""
            classSyntaxRegistryCount = 0
            classSyntaxScopeDepth = 0
            ClassSyntax_ClearRegistry
            nextrunlineindex = 1
            lasttype = 0
            lasttypeelement = 0
            DIM SHARED udtxname(1000) AS STRING * 256
            DIM SHARED udtxcname(1000) AS STRING * 256
            DIM SHARED udtxsize(1000) AS LONG
            DIM SHARED udtxbytealign(1000) AS INTEGER 'first element MUST be on a byte alignment & size is a multiple of 8
            DIM SHARED udtxnext(1000) AS LONG
            DIM SHARED udtxvariable(1000) AS INTEGER 'true if the udt contains variable length elements
            'elements
            DIM SHARED udtename(1000) AS STRING * 256
            DIM SHARED udtecname(1000) AS STRING * 256
            DIM SHARED udtebytealign(1000) AS INTEGER
            DIM SHARED udtesize(1000) AS LONG
            DIM SHARED udtetype(1000) AS LONG
            DIM SHARED udtetypesize(1000) AS LONG
            DIM SHARED udtearrayelements(1000) AS LONG
            DIM SHARED udtenext(1000) AS LONG
            definingtype = 0
            definingtypeerror = 0
            constlast = -1
            'constlastshared = -1
            defdatahandle = 18
            closedmain = 0
            addmetastatic = 0
            addmetadynamic = 0
            DynamicMode = 0
            optionbase = 0
            ExeIconSet = 0
            VersionInfoSet = 0
            viFileVersionNum$ = "": viProductVersionNum$ = "": viCompanyName$ = ""
            viFileDescription$ = "": viFileVersion$ = "": viInternalName$ = ""
            viLegalCopyright$ = "": viLegalTrademarks$ = "": viOriginalFilename$ = ""
            viProductName$ = "": viProductVersion$ = "": viComments$ = "": viWeb$ = ""
            DataOffset = 0
            statementn = 0
            everycasenewcase = 0
            qberrorhappened = 0: qberrorcode = 0: qberrorline = 0
            FOR i = 1 TO 27: defineaz(i) = "SINGLE": defineextaz(i) = "!": NEXT
                controllevel = 0
                findidsecondarg$ = "": findanotherid = 0: findidinternal = 0: currentid = 0
                linenumber = 0
                wholeline$ = ""
                diagnosticSourceLine = ""
                linefragment$ = ""
                idn = 0
                arrayprocessinghappened = 0
                stringprocessinghappened = 0
                inputfunctioncalled = 0
                subfuncn = 0
                closedsubfunc = 0
                subfunc = ""
                SelectCaseCounter = 0
                ExecCounter = 0
                UserDefineCount = 7
                totalVariablesCreated = 0
                typeDefinitions$ = ""
                totalMainVariablesCreated = 0
                DIM SHARED usedVariableList(1000) AS usedVarList
                totalWarnings = 0
                warningListItems = 0
                vWatchUsedLabels = SPACE$(1000)
                vWatchUsedSkipLabels = SPACE$(1000)
                firstLineNumberLabelvWatch = 0
                DIM SHARED warning$(1000)
                DIM SHARED warningLines(1000) AS LONG
                DIM SHARED warningIncLines(1000) AS LONG
                DIM SHARED warningIncFiles(1000) AS STRING
                maxLineNumber = 0
                uniquenumbern = 0


                ''create a type for storing memory blocks
                ''UDT
                ''names
                'DIM SHARED lasttype AS LONG
                'DIM SHARED udtxname(1000) AS STRING * 256
                'DIM SHARED udtxcname(1000) AS STRING * 256
                'DIM SHARED udtxsize(1000) AS LONG
                'DIM SHARED udtxbytealign(1000) AS INTEGER 'first element MUST be on a byte alignment & size is a multiple of 8
                'DIM SHARED udtxnext(1000) AS LONG
                ''elements
                'DIM SHARED lasttypeelement AS LONG
                'DIM SHARED udtename(1000) AS STRING * 256
                'DIM SHARED udtecname(1000) AS STRING * 256
                'DIM SHARED udtebytealign(1000) AS INTEGER
                'DIM SHARED udtesize(1000) AS LONG
                'DIM SHARED udtetype(1000) AS LONG
                'DIM SHARED udtetypesize(1000) AS LONG
                'DIM SHARED udtearrayelements(1000) AS LONG
                'DIM SHARED udtenext(1000) AS LONG

                'import _MEM type
                ptrsz = OS_BITS \ 8

                lasttype = lasttype + 1: i = lasttype
                udtxname(i) = "_MEM"
                udtxcname(i) = "_MEM"
                udtxsize(i) = ((ptrsz) * 5 + (4) * 2 + (8) * 1) * 8
                udtxbytealign(i) = 1
                lasttypeelement = lasttypeelement + 1: i2 = lasttypeelement
                udtename(i2) = "OFFSET"
                udtecname(i2) = "OFFSET"
                udtebytealign(i2) = 1
                udtetype(i2) = OFFSETTYPE: udtesize(i2) = ptrsz * 8
                udtetypesize(i2) = 0 'tsize
                udtxnext(i) = i2
                i3 = i2
                lasttypeelement = lasttypeelement + 1: i2 = lasttypeelement
                udtename(i2) = "SIZE"
                udtecname(i2) = "SIZE"
                udtebytealign(i2) = 1
                udtetype(i2) = OFFSETTYPE: udtesize(i2) = ptrsz * 8
                udtetypesize(i2) = 0 'tsize
                udtenext(i3) = i2
                i3 = i2
                lasttypeelement = lasttypeelement + 1: i2 = lasttypeelement
                udtename(i2) = "$_LOCK_ID"
                udtecname(i2) = "$_LOCK_ID"
                udtebytealign(i2) = 1
                udtetype(i2) = INTEGER64TYPE: udtesize(i2) = 64
                udtetypesize(i2) = 0 'tsize
                udtenext(i3) = i2
                i3 = i2
                lasttypeelement = lasttypeelement + 1: i2 = lasttypeelement
                udtename(i2) = "$_LOCK_OFFSET"
                udtecname(i2) = "$_LOCK_OFFSET"
                udtebytealign(i2) = 1
                udtetype(i2) = OFFSETTYPE: udtesize(i2) = ptrsz * 8
                udtetypesize(i2) = 0 'tsize
                udtenext(i3) = i2
                i3 = i2
                lasttypeelement = lasttypeelement + 1: i2 = lasttypeelement
                udtename(i2) = "TYPE"
                udtecname(i2) = "TYPE"
                udtebytealign(i2) = 1
                udtetype(i2) = OFFSETTYPE: udtesize(i2) = ptrsz * 8
                udtetypesize(i2) = 0 'tsize
                udtenext(i3) = i2
                i3 = i2
                lasttypeelement = lasttypeelement + 1: i2 = lasttypeelement
                udtename(i2) = "ELEMENTSIZE"
                udtecname(i2) = "ELEMENTSIZE"
                udtebytealign(i2) = 1
                udtetype(i2) = OFFSETTYPE: udtesize(i2) = ptrsz * 8
                udtetypesize(i2) = 0 'tsize
                udtenext(i3) = i2
                udtenext(i2) = 0
                i3 = i2
                lasttypeelement = lasttypeelement + 1: i2 = lasttypeelement
                udtename(i2) = "IMAGE"
                udtecname(i2) = "IMAGE"
                udtebytealign(i2) = 1
                udtetype(i2) = LONGTYPE: udtesize(i2) = 32
                udtetypesize(i2) = 0 'tsize
                udtenext(i3) = i2
                udtenext(i2) = 0
                i3 = i2
                lasttypeelement = lasttypeelement + 1: i2 = lasttypeelement
                udtename(i2) = "SOUND"
                udtecname(i2) = "SOUND"
                udtebytealign(i2) = 1
                udtetype(i2) = LONGTYPE: udtesize(i2) = 32
                udtetypesize(i2) = 0 'tsize
                udtenext(i3) = i2
                udtenext(i2) = 0










                'begin compilation
                FOR closeall = 1 TO 255: CLOSE closeall: NEXT
                    OPEN tmpdir$ + "temp.bin" FOR OUTPUT LOCK WRITE AS #26 'relock

                    ff = FREEFILE: OPEN tmpdir$ + "icon.rc" FOR OUTPUT AS #ff: CLOSE #ff

                    IF Debug THEN CLOSE #9: OPEN tmpdir$ + "debug.txt" FOR APPEND AS #9

                    qberrorhappened = -1
                    OPEN sourcefile$ FOR INPUT AS #1
                    qberrorhappened1:
                    IF qberrorhappened = 1 THEN
                        PRINT
                        PRINT "Cannot locate source file: " + sourcefile$
                        IF ConsoleMode THEN SYSTEM 1
                        END 1
                    ELSE
                        CLOSE #1
                    END IF
                    qberrorhappened = 0

                    reginternal

                    IF qbnexprefix_set THEN
                        qbnexprefix$ = ""

                        're-add internal keywords without the "_" prefix
                        reginternal

                        f = HASHFLAG_TYPE + HASHFLAG_RESERVED
                        HashAdd "UNSIGNED", f, 0
                        HashAdd "BIT", f, 0
                        HashAdd "BYTE", f, 0
                        HashAdd "INTEGER64", f, 0
                        HashAdd "OFFSET", f, 0
                        HashAdd "FLOAT", f, 0

                        f = HASHFLAG_RESERVED + HASHFLAG_CUSTOMSYNTAX
                        HashAdd "EXPLICIT", f, 0
                    END IF

                    OPEN tmpdir$ + "global.txt" FOR OUTPUT AS #18

                    IF NOT QuietMode THEN
                        ShowCompilerBanner
                    END IF

                    lineinput3load sourcefile$
                    IF compfailed <> 0 OR HasErrors% THEN
                        IF HasErrors% THEN PrintAllErrors
                        WarnIfStaleOutputBinary
                        IF ConsoleMode THEN SYSTEM 1
                        END 1
                    END IF

                    DO

                        '### STEVE EDIT FOR CONST EXPANSION 10/11/2013

                        wholeline$ = NextPrepassLine$
                        IF wholeline$ = CHR$(13) THEN EXIT DO

                        prepassline:
                        prepassLastLine:

                        IF lastLine <> 0 OR firstLine <> 0 THEN
                            lineBackup$ = wholeline$ 'backup the real line (will be blank when lastline is set)
                            forceIncludeFromRoot$ = ""
                            IF vWatchOn THEN
                                addingvWatch = 1
                                IF firstLine <> 0 THEN forceIncludeFromRoot$ = "internal\support\vwatch\vwatch.bi"
                                IF lastLine <> 0 THEN forceIncludeFromRoot$ = "internal\support\vwatch\vwatch.bm"
                            ELSE
                                'IF firstLine <> 0 THEN forceIncludeFromRoot$ = "internal\support\vwatch\vwatch_stub.bi"
                                IF lastLine <> 0 THEN forceIncludeFromRoot$ = "internal\support\vwatch\vwatch_stub.bm"
                            END IF
                            firstLine = 0: lastLine = 0
                            IF LEN(forceIncludeFromRoot$) THEN GOTO forceInclude_prepass
                            forceIncludeCompleted_prepass:
                            addingvWatch = 0
                            wholeline$ = lineBackup$
                        END IF

                        diagnosticSourceLine = wholeline$
                        wholestv$ = wholeline$ '### STEVE EDIT FOR CONST EXPANSION 10/11/2013

                        prepass = 1
                        layout = ""
                        layoutok = 0

                        linenumber = linenumber + 1
                        reallinenumber = reallinenumber + 1

                        EnsureInvalidLineCapacity linenumber
                        InValidLine(linenumber) = 0

                        IF LEN(wholeline$) THEN

                            IF UCASE$(_TRIM$(wholeline$)) = "$NOPREFIX" THEN
                                qbnexprefix_set_desiredState = 1
                                IF qbnexprefix_set = 0 THEN
                                    IF qbnexprefix_set_recompileAttempts = 0 THEN
                                        qbnexprefix_set_recompileAttempts = qbnexprefix_set_recompileAttempts + 1
                                        GOTO do_recompile
                                    END IF
                                END IF
                            END IF

                            wholeline$ = lineformat(wholeline$)
                            IF Error_Happened THEN GOTO errmes


                            temp$ = LTRIM$(RTRIM$(UCASE$(wholestv$)))

                            IF temp$ = "$COLOR:0" THEN
                                addmetainclude$ = ResolveColorSupportInclude$(0)
                                GOTO finishedlinepp
                            END IF

                            IF temp$ = "$COLOR:32" THEN
                                addmetainclude$ = ResolveColorSupportInclude$(32)
                                GOTO finishedlinepp
                            END IF

                            IF temp$ = "$DEBUG" THEN
                                vWatchDesiredState = 1
                                IF vWatchOn = 0 THEN
                                    IF vWatchRecompileAttempts = 0 THEN
                                        'this is the first time a conflict has occurred, so react immediately with a full recompilation using the desired state
                                        vWatchRecompileAttempts = vWatchRecompileAttempts + 1
                                        GOTO do_recompile
                                    ELSE
                                        'continue compilation to retrieve the final state requested and act on that as required
                                    END IF
                                END IF
                            END IF

                            directiveResult = HandlePrepassConditionalDirective%(temp$)
                            IF directiveResult = 1 THEN GOTO finishedlinepp
                            IF directiveResult = 2 THEN GOTO errmes
                        END IF

                        IF ExecLevel(ExecCounter) THEN
                            EnsureInvalidLineCapacity linenumber
                            InValidLine(linenumber) = -1
                            GOTO finishedlinepp 'we don't check for anything inside lines that we've marked for skipping
                        END IF

                        directiveResult = HandlePrepassMetaDirective%(temp$)
                        IF directiveResult = 1 THEN GOTO finishedlinepp
                        IF directiveResult = 2 THEN GOTO errmes


                        cwholeline$ = wholeline$
                        wholeline$ = eleucase$(wholeline$) '********REMOVE THIS LINE LATER********


                        addmetadynamic = 0: addmetastatic = 0
                        wholelinen = numelements(wholeline$)

                        IF wholelinen THEN

                            wholelinei = 1

                            'skip line number?
                            e$ = getelement$(wholeline$, 1)
                            IF (ASC(e$) >= 48 AND ASC(e$) <= 59) OR ASC(e$) = 46 THEN wholelinei = 2: GOTO ppskpl

                            'skip 'POSSIBLE' line label?
                            IF wholelinen >= 2 THEN
                                x2 = INSTR(wholeline$, sp + ":" + sp): x3 = x2 + 2
                                IF x2 = 0 THEN
                                    IF RIGHT$(wholeline$, 2) = sp + ":" THEN x2 = LEN(wholeline$) - 1: x3 = x2 + 1
                                END IF

                                IF x2 THEN
                                    e$ = LEFT$(wholeline$, x2 - 1)
                                    IF validlabel(e$) THEN
                                        wholeline$ = RIGHT$(wholeline$, LEN(wholeline$) - x3)
                                        cwholeline$ = RIGHT$(cwholeline$, LEN(wholeline$) - x3)
                                        wholelinen = numelements(wholeline$)
                                        GOTO ppskpl
                                    END IF 'valid
                                END IF 'includes ":"
                            END IF 'wholelinen>=2

                            ppskpl:
                            IF wholelinei <= wholelinen THEN
                                '----------------------------------------
                                a$ = ""
                                ca$ = ""
                                ppblda:
                                e$ = getelement$(wholeline$, wholelinei)
                                ce$ = getelement$(cwholeline$, wholelinei)
                                IF e$ = ":" OR e$ = "ELSE" OR e$ = "THEN" OR e$ = "" THEN
                                    IF LEN(a$) THEN
                                        IF Debug THEN PRINT #9, "PP[" + a$ + "]"
                                        n = numelements(a$)
                                        firstelement$ = getelement(a$, 1)
                                        secondelement$ = getelement(a$, 2)
                                        thirdelement$ = getelement(a$, 3)
                                        '========================================

                                        IF n = 2 AND firstelement$ = "END" AND (secondelement$ = "SUB" OR secondelement$ = "FUNCTION") THEN
                                            closedsubfunc = -1
                                        END IF

                                        'declare library
                                        IF declaringlibrary THEN

                                            IF firstelement$ = "END" THEN
                                                IF n <> 2 OR secondelement$ <> "DECLARE" THEN a$ = "Expected END DECLARE": GOTO errmes
                                                declaringlibrary = 0
                                                GOTO finishedlinepp
                                            END IF 'end declare

                                            declaringlibrary = 2

                                            IF firstelement$ = "SUB" OR firstelement$ = "FUNCTION" THEN subfuncn = subfuncn - 1: GOTO declaresubfunc

                                            a$ = "Expected SUB/FUNCTION definition or END DECLARE (#2)": GOTO errmes
                                        END IF

                                        'UDT TYPE definition
                                        IF definingtype THEN
                                            i = definingtype

                                            IF n >= 1 THEN
                                                IF firstelement$ = "END" THEN
                                                    IF n <> 2 OR secondelement$ <> "TYPE" THEN a$ = "Expected END TYPE": GOTO errmes
                                                    IF udtxnext(i) = 0 THEN a$ = "No elements defined in TYPE": GOTO errmes
                                                    definingtype = 0

                                                    'create global buffer for SWAP space
                                                    siz$ = str2$(udtxsize(i) \ 8)
                                                    PRINT #18, "char *g_tmp_udt_" + RTRIM$(udtxname(i)) + "=(char*)malloc(" + siz$ + ");"

                                                    'print "END TYPE";udtxsize(i);udtxbytealign(i)
                                                    GOTO finishedlinepp
                                                END IF
                                            END IF

                                            IF n < 3 THEN a$ = "Expected element-name AS type, AS type element-list, or END TYPE": GOTO errmes
                                            n$ = firstelement$

                                            IF n$ <> "AS" THEN
                                                'traditional variable-name AS type syntax, single-element
                                                lasttypeelement = lasttypeelement + 1
                                                i2 = lasttypeelement
                                                WHILE i2 > UBOUND(udtenext): increaseUDTArrays: WEND
                                                    udtenext(i2) = 0

                                                    ii = 2

                                                    udtearrayelements(i2) = 0

                                                    IF ii >= n OR getelement$(a$, ii) <> "AS" THEN a$ = "Expected element-name AS type, AS type element-list, or END TYPE": GOTO errmes
                                                    t$ = getelements$(a$, ii + 1, n)

                                                    IF t$ = RTRIM$(udtxname(definingtype)) THEN a$ = "Invalid self-reference": GOTO errmes
                                                    typ = typname2typ(t$)
                                                    IF Error_Happened THEN GOTO errmes
                                                    IF typ = 0 THEN a$ = "Undefined type": GOTO errmes
                                                    typsize = typname2typsize

                                                    IF validname(n$) = 0 THEN a$ = "Invalid name": GOTO errmes
                                                    udtename(i2) = n$
                                                    udtecname(i2) = getelement$(ca$, 1)
                                                    NormalTypeBlock:
                                                    typeDefinitions$ = typeDefinitions$ + MKL$(i2) + MKL$(LEN(n$)) + n$
                                                    udtetype(i2) = typ
                                                    udtetypesize(i2) = typsize

                                                    hashname$ = n$

                                                    'check for name conflicts (any similar reserved or element from current UDT)
                                                    hashchkflags = HASHFLAG_RESERVED + HASHFLAG_UDTELEMENT
                                                    hashres = HashFind(hashname$, hashchkflags, hashresflags, hashresref)
                                                    DO WHILE hashres
                                                        IF hashresflags AND HASHFLAG_UDTELEMENT THEN
                                                            IF hashresref = i THEN a$ = "Name already in use (" + hashname$ + ")": GOTO errmes
                                                        END IF
                                                        IF hashresflags AND HASHFLAG_RESERVED THEN
                                                            IF hashresflags AND (HASHFLAG_TYPE + HASHFLAG_CUSTOMSYNTAX + HASHFLAG_OPERATOR + HASHFLAG_XELEMENTNAME) THEN a$ = "Name already in use (" + hashname$ + ")": GOTO errmes
                                                        END IF
                                                        IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
                                                    LOOP
                                                    'add to hash table
                                                    HashAdd hashname$, HASHFLAG_UDTELEMENT, i

                                                    'Calculate element's size
                                                    IF typ AND ISUDT THEN
                                                        u = typ AND 511
                                                        udtesize(i2) = udtxsize(u)
                                                        IF udtxbytealign(u) THEN udtxbytealign(i) = 1: udtebytealign(i2) = 1
                                                        IF udtxvariable(u) THEN udtxvariable(i) = -1
                                                    ELSE
                                                        IF (typ AND ISSTRING) THEN
                                                            IF (typ AND ISFIXEDLENGTH) = 0 THEN
                                                                udtesize(i2) = OFFSETTYPE AND 511
                                                                udtxvariable(i) = -1
                                                            ELSE
                                                                udtesize(i2) = typsize * 8
                                                            END IF
                                                            udtxbytealign(i) = 1: udtebytealign(i2) = 1
                                                        ELSE
                                                            udtesize(i2) = typ AND 511
                                                            IF (typ AND ISOFFSETINBITS) = 0 THEN udtxbytealign(i) = 1: udtebytealign(i2) = 1
                                                        END IF
                                                    END IF

                                                    'Increase block size
                                                    IF udtebytealign(i2) THEN
                                                        IF udtxsize(i) MOD 8 THEN
                                                            udtxsize(i) = udtxsize(i) + (8 - (udtxsize(i) MOD 8))
                                                        END IF
                                                    END IF
                                                    udtxsize(i) = udtxsize(i) + udtesize(i2)

                                                    'Link element to previous element
                                                    IF udtxnext(i) = 0 THEN
                                                        udtxnext(i) = i2
                                                    ELSE
                                                        udtenext(i2 - 1) = i2
                                                    END IF

                                                    'print "+"+rtrim$(udtename(i2));udtetype(i2);udtesize(i2);udtebytealign(i2);udtxsize(i)
                                                    IF newAsTypeBlockSyntax THEN RETURN
                                                    GOTO finishedlinepp
                                                ELSE
                                                    'new AS type variable-list syntax, multiple elements
                                                    ii = 2

                                                    IF ii >= n THEN a$ = "Expected element-name AS type, AS type element-list, or END TYPE": GOTO errmes
                                                    previousElement$ = ""
                                                    t$ = ""
                                                    lastElement$ = ""
                                                    buildTypeName:
                                                    lastElement$ = getelement$(a$, ii)
                                                    IF lastElement$ <> "," AND lastElement$ <> "" THEN
                                                        n$ = lastElement$
                                                        cn$ = getelement$(ca$, ii)
                                                        IF LEN(previousElement$) THEN t$ = t$ + previousElement$ + " "
                                                        previousElement$ = n$
                                                        lastElement$ = ""
                                                        ii = ii + 1
                                                        GOTO buildTypeName
                                                    END IF

                                                    t$ = RTRIM$(t$)
                                                    IF t$ = RTRIM$(udtxname(definingtype)) THEN a$ = "Invalid self-reference": GOTO errmes
                                                    typ = typname2typ(t$)
                                                    IF Error_Happened THEN GOTO errmes
                                                    IF typ = 0 THEN a$ = "Undefined type": GOTO errmes
                                                    typsize = typname2typsize

                                                    previousElement$ = lastElement$
                                                    nexttypeelement:
                                                    lasttypeelement = lasttypeelement + 1
                                                    i2 = lasttypeelement
                                                    WHILE i2 > UBOUND(udtenext): increaseUDTArrays: WEND
                                                        udtenext(i2) = 0
                                                        udtearrayelements(i2) = 0

                                                        udtename(i2) = n$
                                                        udtecname(i2) = cn$

                                                        IF validname(n$) = 0 THEN a$ = "Invalid name": GOTO errmes

                                                        newAsTypeBlockSyntax = -1
                                                        GOSUB NormalTypeBlock
                                                        newAsTypeBlockSyntax = 0

                                                        getNextElement:
                                                        ii = ii + 1
                                                        lastElement$ = getelement$(a$, ii)
                                                        IF lastElement$ = "" THEN GOTO finishedlinepp
                                                        IF ii = n AND lastElement$ = "," THEN a$ = "Expected element-name": GOTO errmes
                                                        IF lastElement$ = "," THEN
                                                            IF previousElement$ = "," THEN a$ = "Expected element-name": GOTO errmes
                                                            previousElement$ = lastElement$
                                                            GOTO getNextElement
                                                        END IF
                                                        n$ = lastElement$
                                                        IF previousElement$ <> "," THEN a$ = "Expected ,": GOTO errmes
                                                        previousElement$ = lastElement$
                                                        cn$ = getelement$(ca$, ii)
                                                        GOTO nexttypeelement
                                                    END IF
                                                END IF 'definingtype

                                                IF definingtype AND n >= 1 THEN a$ = "Expected END TYPE": GOTO errmes

                                                IF n >= 1 THEN
                                                    IF firstelement$ = "TYPE" THEN
                                                        IF n <> 2 THEN a$ = "Expected TYPE typename": GOTO errmes
                                                        lasttype = lasttype + 1
                                                        typeDefinitions$ = typeDefinitions$ + MKL$(-1) + MKL$(lasttype)
                                                        definingtype = lasttype
                                                        i = definingtype
                                                        WHILE i > UBOUND(udtenext): increaseUDTArrays: WEND
                                                            IF validname(secondelement$) = 0 THEN a$ = "Invalid name": GOTO errmes
                                                            typeDefinitions$ = typeDefinitions$ + MKL$(LEN(secondelement$)) + secondelement$
                                                            udtxname(i) = secondelement$
                                                            udtxcname(i) = getelement(ca$, 2)
                                                            udtxnext(i) = 0
                                                            udtxsize(i) = 0
                                                            udtxvariable(i) = 0

                                                            hashname$ = secondelement$
                                                            hashflags = HASHFLAG_UDT
                                                            'check for name conflicts (any similar reserved/sub/function/UDT name)
                                                            hashchkflags = HASHFLAG_RESERVED + HASHFLAG_SUB + HASHFLAG_FUNCTION + HASHFLAG_UDT
                                                            hashres = HashFind(hashname$, hashchkflags, hashresflags, hashresref)
                                                            DO WHILE hashres
                                                                allow = 0
                                                                IF hashresflags AND (HASHFLAG_SUB + HASHFLAG_FUNCTION) THEN
                                                                    allow = 1
                                                                END IF
                                                                IF hashresflags AND HASHFLAG_RESERVED THEN
                                                                    IF (hashresflags AND (HASHFLAG_TYPE + HASHFLAG_OPERATOR + HASHFLAG_CUSTOMSYNTAX + HASHFLAG_XTYPENAME)) = 0 THEN allow = 1
                                                                END IF
                                                                IF allow = 0 THEN a$ = "Name already in use (" + hashname$ + ")": GOTO errmes
                                                                IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
                                                            LOOP

                                                            'add to hash table
                                                            HashAdd hashname$, hashflags, i

                                                            GOTO finishedlinepp
                                                        END IF
                                                    END IF





                                                    IF n >= 1 AND firstelement$ = "CONST" THEN
                                                        'l$ = "CONST"
                                                        'DEF... do not change type, the expression is stored in a suitable type
                                                        'based on its value if type isn't forced/specified

                                                        IF subfuncn > 0 AND closedsubfunc <> 0 THEN a$ = "Statement cannot be placed between SUB/FUNCTIONs": GOTO errmes

                                                        'convert periods to _046_
                                                        i2 = INSTR(a$, sp + "." + sp)
                                                        IF i2 THEN
                                                            DO
                                                                a$ = LEFT$(a$, i2 - 1) + fix046$ + RIGHT$(a$, LEN(a$) - i2 - 2)
                                                                ca$ = LEFT$(ca$, i2 - 1) + fix046$ + RIGHT$(ca$, LEN(ca$) - i2 - 2)
                                                                i2 = INSTR(a$, sp + "." + sp)
                                                            LOOP UNTIL i2 = 0
                                                            n = numelements(a$)
                                                            firstelement$ = getelement(a$, 1): secondelement$ = getelement(a$, 2): thirdelement$ = getelement(a$, 3)
                                                        END IF

                                                        IF n < 3 THEN a$ = "Expected CONST name = value/expression": GOTO errmes
                                                        i = 2
                                                        constdefpendingpp:
                                                        pending = 0

                                                        n$ = getelement$(ca$, i): i = i + 1
                                                        typeoverride = 0
                                                        s$ = removesymbol$(n$)
                                                        IF Error_Happened THEN GOTO errmes
                                                        IF s$ <> "" THEN
                                                            typeoverride = typname2typ(s$)
                                                            IF Error_Happened THEN GOTO errmes
                                                            IF typeoverride AND ISFIXEDLENGTH THEN a$ = "Invalid constant type": GOTO errmes
                                                            IF typeoverride = 0 THEN a$ = "Invalid constant type": GOTO errmes
                                                        END IF

                                                        IF getelement$(a$, i) <> "=" THEN a$ = "Expected =": GOTO errmes
                                                        i = i + 1

                                                        'get expression
                                                        e$ = ""
                                                        readable_e$ = ""
                                                        B = 0
                                                        FOR i2 = i TO n
                                                            e2$ = getelement$(ca$, i2)
                                                            IF e2$ = "(" THEN B = B + 1
                                                            IF e2$ = ")" THEN B = B - 1
                                                            IF e2$ = "," AND B = 0 THEN
                                                                pending = 1
                                                                i = i2 + 1
                                                                IF i > n - 2 THEN a$ = "Expected CONST ... , name = value/expression": GOTO errmes
                                                                EXIT FOR
                                                            END IF
                                                            IF LEN(e$) = 0 THEN e$ = e2$ ELSE e$ = e$ + sp + e2$

                                                            e3$ = e2$
                                                            IF LEN(e2$) > 1 THEN
                                                                IF ASC(e2$, 1) = 34 THEN
                                                                    removeComma = _INSTRREV(e2$, ",")
                                                                    e3$ = LEFT$(e2$, removeComma - 1)
                                                                ELSE
                                                                    removeComma = INSTR(e2$, ",")
                                                                    e3$ = MID$(e2$, removeComma + 1)
                                                                END IF
                                                            END IF

                                                            IF LEN(readable_e$) = 0 THEN
                                                                readable_e$ = e3$
                                                            ELSE
                                                                readable_e$ = readable_e$ + " " + e3$
                                                            END IF
                                                        NEXT

                                                        'intercept current expression and pass it through Evaluate_Expression$
                                                        '(unless it is a literal string)
                                                        IF LEFT$(readable_e$, 1) <> CHR$(34) THEN
                                                            temp1$ = _TRIM$(Evaluate_Expression$(readable_e$))
                                                            IF LEFT$(temp1$, 5) <> "ERROR" AND e$ <> temp1$ THEN
                                                                e$ = lineformat(temp1$) 'retrieve parseable format
                                                            ELSE
                                                                IF temp1$ = "ERROR - Division By Zero" THEN a$ = temp1$: GOTO errmes
                                                                IF INSTR(temp1$, "Improper operations") THEN
                                                                    a$ = "Invalid CONST expression.14": GOTO errmes
                                                                END IF
                                                            END IF
                                                        END IF

                                                        'Proceed as usual
                                                        e$ = fixoperationorder(e$)
                                                        IF Error_Happened THEN GOTO errmes

                                                        e$ = evaluateconst(e$, t)
                                                        IF Error_Happened THEN GOTO errmes

                                                        IF t AND ISSTRING THEN 'string type

                                                        IF typeoverride THEN
                                                            IF (typeoverride AND ISSTRING) = 0 THEN a$ = "Type mismatch": GOTO errmes
                                                        END IF

                                                    ELSE 'not a string type

                                                        IF typeoverride THEN
                                                            IF typeoverride AND ISSTRING THEN a$ = "Type mismatch": GOTO errmes
                                                        END IF

                                                        IF t AND ISFLOAT THEN
                                                            constval## = _CV(_FLOAT, e$)
                                                            constval&& = constval##
                                                            constval~&& = constval&&
                                                        ELSE
                                                            IF (t AND ISUNSIGNED) AND (t AND 511) = 64 THEN
                                                                constval~&& = _CV(_UNSIGNED _INTEGER64, e$)
                                                                constval&& = constval~&&
                                                                constval## = constval&&
                                                            ELSE
                                                                constval&& = _CV(_INTEGER64, e$)
                                                                constval## = constval&&
                                                                constval~&& = constval&&
                                                            END IF
                                                        END IF

                                                        'override type?
                                                        IF typeoverride THEN
                                                            'range check required here (noted in todo)
                                                            t = typeoverride
                                                        END IF

                                                    END IF 'not a string type

                                                    constlast = constlast + 1
                                                    IF constlast > constmax THEN
                                                        constmax = constmax * 2
                                                        REDIM _PRESERVE constname(constmax) AS STRING
                                                        REDIM _PRESERVE constcname(constmax) AS STRING
                                                        REDIM _PRESERVE constnamesymbol(constmax) AS STRING 'optional name symbol
                                                        REDIM _PRESERVE consttype(constmax) AS LONG 'variable type number
                                                        REDIM _PRESERVE constinteger(constmax) AS _INTEGER64
                                                        REDIM _PRESERVE constuinteger(constmax) AS _UNSIGNED _INTEGER64
                                                        REDIM _PRESERVE constfloat(constmax) AS _FLOAT
                                                        REDIM _PRESERVE conststring(constmax) AS STRING
                                                        REDIM _PRESERVE constsubfunc(constmax) AS LONG
                                                        REDIM _PRESERVE constdefined(constmax) AS LONG
                                                    END IF

                                                    i2 = constlast

                                                    constsubfunc(i2) = subfuncn
                                                    'IF subfunc = "" THEN constlastshared = i2

                                                    IF validname(n$) = 0 THEN a$ = "Invalid name": GOTO errmes
                                                    constname(i2) = UCASE$(n$)

                                                    hashname$ = n$
                                                    'check for name conflicts (any similar: reserved, sub, function, constant)

                                                    allow = 0
                                                    const_recheck:
                                                    hashchkflags = HASHFLAG_RESERVED + HASHFLAG_SUB + HASHFLAG_FUNCTION + HASHFLAG_CONSTANT
                                                    hashres = HashFind(hashname$, hashchkflags, hashresflags, hashresref)
                                                    DO WHILE hashres
                                                        IF hashresflags AND HASHFLAG_CONSTANT THEN
                                                            IF constsubfunc(hashresref) = subfuncn THEN
                                                                'If merely redefining a CONST with same value
                                                                'just issue a warning instead of an error
                                                                issueWarning = 0
                                                                IF t AND ISSTRING THEN
                                                                    IF conststring(hashresref) = e$ THEN issueWarning = -1: thisconstval$ = e$
                                                                ELSE
                                                                    IF t AND ISFLOAT THEN
                                                                        IF constfloat(hashresref) = constval## THEN issueWarning = -1: thisconstval$ = STR$(constval##)
                                                                    ELSE
                                                                        IF t AND ISUNSIGNED THEN
                                                                            IF constuinteger(hashresref) = constval~&& THEN issueWarning = -1: thisconstval$ = STR$(constval~&&)
                                                                        ELSE
                                                                            IF constinteger(hashresref) = constval&& THEN issueWarning = -1: thisconstval$ = STR$(constval&&)
                                                                        END IF
                                                                    END IF
                                                                END IF
                                                                IF issueWarning THEN
                                                                    IF NOT IgnoreWarnings THEN
                                                                        addWarning linenumber, inclevel, inclinenumber(inclevel), incname$(inclevel), "duplicate constant definition", n$ + " =" + thisconstval$
                                                                    END IF
                                                                    GOTO constAddDone
                                                                ELSE
                                                                    a$ = "Name already in use (" + hashname$ + ")": GOTO errmes
                                                                END IF
                                                            END IF
                                                        END IF
                                                        IF hashresflags AND HASHFLAG_RESERVED THEN
                                                            a$ = "Name already in use (" + hashname$ + ")": GOTO errmes
                                                        END IF
                                                        IF hashresflags AND (HASHFLAG_SUB + HASHFLAG_FUNCTION) THEN
                                                            IF ids(hashresref).internal_subfunc = 0 OR RTRIM$(ids(hashresref).musthave) <> "$" THEN a$ = "Name already in use (" + hashname$ + ")": GOTO errmes
                                                            IF t AND ISSTRING THEN a$ = "Name already in use (" + hashname$ + ")": GOTO errmes
                                                        END IF
                                                        IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
                                                    LOOP

                                                    'add to hash table
                                                    HashAdd hashname$, HASHFLAG_CONSTANT, i2





                                                    constdefined(i2) = 1
                                                    constcname(i2) = n$
                                                    constnamesymbol(i2) = typevalue2symbol$(t)
                                                    IF Error_Happened THEN GOTO errmes
                                                    consttype(i2) = t
                                                    IF t AND ISSTRING THEN
                                                        conststring(i2) = e$
                                                    ELSE
                                                        IF t AND ISFLOAT THEN
                                                            constfloat(i2) = constval##
                                                        ELSE
                                                            IF t AND ISUNSIGNED THEN
                                                                constuinteger(i2) = constval~&&
                                                            ELSE
                                                                constinteger(i2) = constval&&
                                                            END IF
                                                        END IF
                                                    END IF

                                                    constAddDone:

                                                    IF pending THEN
                                                        'l$ = l$ + sp2 + ","
                                                        GOTO constdefpendingpp
                                                    END IF

                                                    'layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
                                                    GOTO finishedlinepp
                                                END IF



                                                'DEFINE
                                                d = 0
                                                IF firstelement$ = "DEFINT" THEN d = 1
                                                IF firstelement$ = "DEFLNG" THEN d = 1
                                                IF firstelement$ = "DEFSNG" THEN d = 1
                                                IF firstelement$ = "DEFDBL" THEN d = 1
                                                IF firstelement$ = "DEFSTR" THEN d = 1
                                                IF firstelement$ = "_DEFINE" OR (firstelement$ = "DEFINE" AND qbnexprefix_set = 1) THEN d = 1
                                                IF d THEN
                                                    predefining = 1: GOTO predefine
                                                    predefined: predefining = 0
                                                    GOTO finishedlinepp
                                                END IF

                                                'declare library
                                                IF firstelement$ = "DECLARE" THEN
                                                    IF secondelement$ = "LIBRARY" OR secondelement$ = "DYNAMIC" OR secondelement$ = "CUSTOMTYPE" OR secondelement$ = "STATIC" THEN
                                                        declaringlibrary = 1
                                                        indirectlibrary = 0
                                                        IF secondelement$ = "CUSTOMTYPE" OR secondelement$ = "DYNAMIC" THEN indirectlibrary = 1
                                                        GOTO finishedlinepp
                                                    END IF
                                                END IF

                                                'SUB/FUNCTION
                                                dynamiclibrary = 0
                                                declaresubfunc:
                                                firstelement$ = getelement$(a$, 1)
                                                sf = 0
                                                IF firstelement$ = "FUNCTION" THEN sf = 1
                                                IF firstelement$ = "SUB" THEN sf = 2
                                                IF sf THEN

                                                    subfuncn = subfuncn + 1
                                                    closedsubfunc = 0

                                                    IF n = 1 THEN a$ = "Expected name after SUB/FUNCTION": GOTO errmes

                                                    'convert periods to _046_
                                                    i2 = INSTR(a$, sp + "." + sp)
                                                    IF i2 THEN
                                                        DO
                                                            a$ = LEFT$(a$, i2 - 1) + fix046$ + RIGHT$(a$, LEN(a$) - i2 - 2)
                                                            ca$ = LEFT$(ca$, i2 - 1) + fix046$ + RIGHT$(ca$, LEN(ca$) - i2 - 2)
                                                            i2 = INSTR(a$, sp + "." + sp)
                                                        LOOP UNTIL i2 = 0
                                                        n = numelements(a$)
                                                        firstelement$ = getelement(a$, 1): secondelement$ = getelement(a$, 2): thirdelement$ = getelement(a$, 3)
                                                    END IF

                                                    n$ = getelement$(ca$, 2)
                                                    symbol$ = removesymbol$(n$)
                                                    IF Error_Happened THEN GOTO errmes
                                                    IF sf = 2 AND symbol$ <> "" THEN a$ = "Type symbols after a SUB name are invalid": GOTO errmes

                                                    'remove STATIC (which is ignored)
                                                    e$ = getelement$(a$, n): IF e$ = "STATIC" THEN a$ = LEFT$(a$, LEN(a$) - 7): ca$ = LEFT$(ca$, LEN(ca$) - 7): n = n - 1

                                                    'check for ALIAS
                                                    aliasname$ = n$ 'use given name by default
                                                    IF n > 2 THEN
                                                        e$ = getelement$(a$, 3)
                                                        IF e$ = "ALIAS" THEN
                                                            IF declaringlibrary = 0 THEN a$ = "ALIAS can only be used with DECLARE LIBRARY": GOTO errmes
                                                            IF n = 3 THEN a$ = "Expected ALIAS name-in-library": GOTO errmes
                                                            e$ = getelement$(ca$, 4)
                                                            'strip string content (optional)
                                                            IF LEFT$(e$, 1) = CHR$(34) THEN
                                                                e$ = RIGHT$(e$, LEN(e$) - 1)
                                                                x = INSTR(e$, CHR$(34)): IF x = 0 THEN a$ = "Expected " + CHR$(34): GOTO errmes
                                                                e$ = LEFT$(e$, x - 1)
                                                            END IF
                                                            'strip fix046$ (created by unquoted periods)
                                                            DO WHILE INSTR(e$, fix046$)
                                                                x = INSTR(e$, fix046$): e$ = LEFT$(e$, x - 1) + "." + RIGHT$(e$, LEN(e$) - x + 1 - LEN(fix046$))
                                                            LOOP
                                                            'validate alias name
                                                            IF LEN(e$) = 0 THEN a$ = "Expected ALIAS name-in-library": GOTO errmes
                                                            FOR x = 1 TO LEN(e$)
                                                                a = ASC(e$, x)
                                                                IF alphanumeric(a) = 0 AND a <> ASC_FULLSTOP AND a <> ASC_COLON THEN a$ = "Expected ALIAS name-in-library": GOTO errmes
                                                            NEXT
                                                            aliasname$ = e$
                                                            'remove ALIAS section from line
                                                            IF n <= 4 THEN a$ = getelements(a$, 1, 2)
                                                            IF n >= 5 THEN a$ = getelements(a$, 1, 2) + sp + getelements(a$, 5, n)
                                                            IF n <= 4 THEN ca$ = getelements(ca$, 1, 2)
                                                            IF n >= 5 THEN ca$ = getelements(ca$, 1, 2) + sp + getelements(ca$, 5, n)
                                                            n = n - 2
                                                        END IF
                                                    END IF

                                                    IF declaringlibrary THEN
                                                        IF indirectlibrary THEN
                                                            aliasname$ = n$ 'override the alias name
                                                        END IF
                                                    END IF

                                                    params = 0
                                                    params$ = ""
                                                    paramsize$ = ""
                                                    nele$ = ""
                                                    nelereq$ = ""
                                                    IF n > 2 THEN
                                                        e$ = getelement$(a$, 3)
                                                        IF e$ <> "(" THEN a$ = "Expected (": GOTO errmes
                                                        e$ = getelement$(a$, n)
                                                        IF e$ <> ")" THEN a$ = "Expected )": GOTO errmes
                                                        IF n < 4 THEN a$ = "Expected ( ... )": GOTO errmes
                                                        IF n = 4 THEN GOTO nosfparams
                                                        B = 0
                                                        a2$ = ""
                                                        FOR i = 4 TO n - 1
                                                            e$ = getelement$(a$, i)
                                                            IF e$ = "(" THEN B = B + 1
                                                            IF e$ = ")" THEN B = B - 1
                                                            IF e$ = "," AND B = 0 THEN
                                                                IF i = n - 1 THEN a$ = "Expected , ... )": GOTO errmes
                                                                getlastparam:
                                                                IF a2$ = "" THEN a$ = "Expected ... ,": GOTO errmes
                                                                a2$ = LEFT$(a2$, LEN(a2$) - 1)
                                                                'possible format: [BYVAL]a[%][(1)][AS][type]
                                                                n2 = numelements(a2$)
                                                                array = 0
                                                                t2$ = ""

                                                                i2 = 1
                                                                e$ = getelement$(a2$, i2): i2 = i2 + 1

                                                                byvalue = 0
                                                                IF e$ = "BYVAL" THEN
                                                                    IF declaringlibrary = 0 THEN a$ = "BYVAL can currently only be used with DECLARE LIBRARY": GOTO errmes
                                                                    e$ = getelement$(a2$, i2): i2 = i2 + 1: byvalue = 1
                                                                END IF

                                                                n2$ = e$
                                                                symbol2$ = removesymbol$(n2$)
                                                                IF validname(n2$) = 0 THEN a$ = "Invalid name": GOTO errmes

                                                                IF Error_Happened THEN GOTO errmes
                                                                m = 0
                                                                FOR i2 = i2 TO n2
                                                                    e$ = getelement$(a2$, i2)
                                                                    IF e$ = "(" THEN
                                                                        IF m <> 0 THEN a$ = "Syntax error - too many opening brackets": GOTO errmes
                                                                        m = 1
                                                                        array = 1
                                                                        GOTO gotaa
                                                                    END IF
                                                                    IF e$ = ")" THEN
                                                                        IF m <> 1 THEN a$ = "Syntax error - closing bracket without opening bracket": GOTO errmes
                                                                        m = 2
                                                                        GOTO gotaa
                                                                    END IF
                                                                    IF e$ = "AS" THEN
                                                                        IF m <> 0 AND m <> 2 THEN a$ = "Syntax error - check your brackets": GOTO errmes
                                                                        m = 3
                                                                        GOTO gotaa
                                                                    END IF
                                                                    IF m = 1 THEN GOTO gotaa 'ignore contents of bracket
                                                                    IF m <> 3 THEN a$ = "Syntax error - check your brackets": GOTO errmes
                                                                    IF t2$ = "" THEN t2$ = e$ ELSE t2$ = t2$ + " " + e$
                                                                    gotaa:
                                                                NEXT i2

                                                                params = params + 1: IF params > 100 THEN a$ = "SUB/FUNCTION exceeds 100 parameter limit": GOTO errmes

                                                                argnelereq = 0

                                                                IF symbol2$ <> "" AND t2$ <> "" THEN a$ = "Syntax error - check parameter types": GOTO errmes
                                                                IF t2$ = "" AND e$ = "AS" THEN a$ = "Expected AS type": GOTO errmes
                                                                IF t2$ = "" THEN t2$ = symbol2$
                                                                IF t2$ = "" THEN
                                                                    IF LEFT$(n2$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(n2$)) - 64
                                                                    t2$ = defineaz(v)
                                                                END IF

                                                                paramsize = 0
                                                                IF array = 1 THEN
                                                                    t = typname2typ(t2$)
                                                                    IF Error_Happened THEN GOTO errmes
                                                                    IF t = 0 THEN a$ = "Illegal SUB/FUNCTION parameter": GOTO errmes
                                                                    IF (t AND ISFIXEDLENGTH) THEN paramsize = typname2typsize
                                                                    t = t + ISARRAY
                                                                    'check for recompilation override
                                                                    FOR i10 = 0 TO sflistn
                                                                        IF sfidlist(i10) = idn + 1 THEN
                                                                            IF sfarglist(i10) = params THEN
                                                                                argnelereq = sfelelist(i10)
                                                                            END IF
                                                                        END IF
                                                                    NEXT
                                                                ELSE
                                                                    t = typname2typ(t2$)
                                                                    IF Error_Happened THEN GOTO errmes
                                                                    IF t = 0 THEN a$ = "Illegal SUB/FUNCTION parameter": GOTO errmes
                                                                    IF (t AND ISFIXEDLENGTH) THEN paramsize = typname2typsize

                                                                    IF byvalue THEN
                                                                        IF t AND ISPOINTER THEN t = t - ISPOINTER
                                                                    END IF

                                                                END IF
                                                                nelereq$ = nelereq$ + CHR$(argnelereq)

                                                                'consider changing 0 in following line too!
                                                                nele$ = nele$ + CHR$(0)

                                                                paramsize$ = paramsize$ + MKL$(paramsize)
                                                                params$ = params$ + MKL$(t)
                                                                a2$ = ""
                                                            ELSE
                                                                a2$ = a2$ + e$ + sp
                                                                IF i = n - 1 THEN GOTO getlastparam
                                                            END IF
                                                        NEXT i
                                                    END IF 'n>2
                                                    nosfparams:

                                                    IF sf = 1 THEN
                                                        'function
                                                        clearid
                                                        id.n = n$
                                                        id.subfunc = 1

                                                        id.callname = "FUNC_" + UCASE$(n$)
                                                        IF declaringlibrary THEN
                                                            id.ccall = 1
                                                            IF indirectlibrary = 0 THEN id.callname = aliasname$
                                                        END IF
                                                        id.args = params
                                                        id.arg = params$
                                                        id.argsize = paramsize$
                                                        id.nele = nele$
                                                        id.nelereq = nelereq$
                                                        IF symbol$ <> "" THEN
                                                            id.ret = typname2typ(symbol$)
                                                            IF Error_Happened THEN GOTO errmes
                                                        ELSE
                                                            IF LEFT$(n$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(n$)) - 64
                                                            symbol$ = defineaz(v)
                                                            id.ret = typname2typ(symbol$)
                                                            IF Error_Happened THEN GOTO errmes
                                                        END IF
                                                        IF id.ret = 0 THEN a$ = "Invalid FUNCTION return type": GOTO errmes

                                                        IF declaringlibrary THEN

                                                            ctype$ = typ2ctyp$(id.ret, "")
                                                            IF Error_Happened THEN GOTO errmes
                                                            IF ctype$ = "qbs" THEN ctype$ = "char*"
                                                            id.callname = "(  " + ctype$ + "  )" + RTRIM$(id.callname)

                                                        END IF

                                                        s$ = LEFT$(symbol$, 1)
                                                        IF s$ <> "~" AND s$ <> "`" AND s$ <> "%" AND s$ <> "&" AND s$ <> "!" AND s$ <> "#" AND s$ <> "$" THEN
                                                            symbol$ = type2symbol$(symbol$)
                                                            IF Error_Happened THEN GOTO errmes
                                                        END IF
                                                        id.mayhave = symbol$
                                                        IF id.ret AND ISPOINTER THEN
                                                            IF (id.ret AND ISSTRING) = 0 THEN id.ret = id.ret - ISPOINTER
                                                        END IF
                                                        regid
                                                        IF Error_Happened THEN GOTO errmes
                                                    ELSE
                                                        'sub
                                                        clearid
                                                        id.n = n$
                                                        id.subfunc = 2
                                                        id.callname = "SUB_" + UCASE$(n$)
                                                        IF declaringlibrary THEN
                                                            id.ccall = 1
                                                            IF indirectlibrary = 0 THEN id.callname = aliasname$
                                                        END IF
                                                        id.args = params
                                                        id.arg = params$
                                                        id.argsize = paramsize$
                                                        id.nele = nele$
                                                        id.nelereq = nelereq$

                                                        IF UCASE$(n$) = "_GL" AND params = 0 AND UseGL = 0 THEN reginternalsubfunc = 1: UseGL = 1: id.n = "_GL": DEPENDENCY(DEPENDENCY_GL) = 1
                                                        regid
                                                        reginternalsubfunc = 0

                                                        IF Error_Happened THEN GOTO errmes
                                                    END IF


                                                END IF

                                                '========================================
                                                finishedlinepp:
                                                firstLine = 0
                                            END IF
                                            a$ = ""
                                            ca$ = ""
                                        ELSE
                                            IF a$ = "" THEN a$ = e$: ca$ = ce$ ELSE a$ = a$ + sp + e$: ca$ = ca$ + sp + ce$
                                        END IF
                                        IF wholelinei <= wholelinen THEN wholelinei = wholelinei + 1: GOTO ppblda
                                        '----------------------------------------
                                    END IF 'wholelinei<=wholelinen
                                END IF 'wholelinen
                                'Include Manager #1



                                IF LEN(addmetainclude$) THEN
                                    IF Debug THEN PRINT #9, "Pre-pass:INCLUDE$-ing file:'" + addmetainclude$ + "':On line"; linenumber
                                    a$ = addmetainclude$: addmetainclude$ = "" 'read/clear message

                                    IF inclevel = 0 THEN
                                        includingFromRoot = 0
                                        forceIncludingFile = 0
                                        forceInclude_prepass:
                                        IF forceIncludeFromRoot$ <> "" THEN
                                            a$ = forceIncludeFromRoot$
                                            forceIncludeFromRoot$ = ""
                                            forceIncludingFile = 1
                                            includingFromRoot = 1
                                        END IF
                                    END IF

                                    IF inclevel = 100 THEN a$ = "Too many indwelling INCLUDE files": GOTO errmes
                                    '1. Verify file exists (location is either (a)relative to source file or (b)absolute)
                                    fh = 99 + inclevel + 1

                                    firstTryMethod = 1
                                    IF includingFromRoot <> 0 AND inclevel = 0 THEN firstTryMethod = 2
                                    FOR try = firstTryMethod TO 2 'if including file from root, do not attempt including from relative location
                                        IF try = 1 THEN
                                            IF inclevel = 0 THEN
                                                p$ = getfilepath$(sourcefile$)
                                            ELSE
                                                p$ = getfilepath$(incname(inclevel))
                                            END IF
                                            f$ = p$ + a$
                                        END IF
                                        IF try = 2 THEN f$ = a$
                                        IF _FILEEXISTS(f$) THEN
                                            qberrorhappened = -3
                                            'We're using the faster LINE INPUT, which requires a BINARY open.
                                            OPEN f$ FOR BINARY AS #fh
                                            'And another line below edited
                                            qberrorhappened3:
                                            IF qberrorhappened = -3 THEN EXIT FOR
                                        END IF
                                        qberrorhappened = 0
                                    NEXT
                                    IF qberrorhappened <> -3 THEN qberrorhappened = 0: a$ = "File " + a$ + " not found": GOTO errmes
                                    inclevel = inclevel + 1: incname$(inclevel) = f$: inclinenumber(inclevel) = 0
                                END IF 'fall through to next section...
                                '--------------------
                                DO WHILE inclevel

                                    fh = 99 + inclevel
                                    '2. Feed next line
                                    IF LEN(classSyntaxQueue$) THEN
                                        wholeline$ = ClassSyntax_DequeueLine$
                                        wholeline$ = TopLevelRuntime_ProcessLine$(wholeline$)
                                        linenumber = linenumber - 1 'lower official linenumber to counter later increment

                                        IF Debug THEN PRINT #9, "Pre-pass:Feeding INCLUDE$ line:[" + wholeline$ + "]"

                                        GOTO prepassline
                                    END IF
                                    IF EOF(fh) = 0 THEN
                                        LINE INPUT #fh, x$

                                        wholeline$ = ClassSyntax_ProcessLine$(x$)
                                        wholeline$ = TopLevelRuntime_ProcessLine$(wholeline$)
                                        inclinenumber(inclevel) = inclinenumber(inclevel) + 1
                                        'create extended error string 'incerror$'
                                        errorLineInInclude = inclinenumber(inclevel)
                                        e$ = " in line " + str2(inclinenumber(inclevel)) + " of " + incname$(inclevel) + " included"
                                        IF inclevel > 1 THEN
                                            e$ = e$ + " (through "
                                            FOR x = 1 TO inclevel - 1 STEP 1
                                                e$ = e$ + incname$(x)
                                                IF x < inclevel - 1 THEN 'a sep is req
                                                IF x = inclevel - 2 THEN
                                                    e$ = e$ + " then "
                                                ELSE
                                                    e$ = e$ + ", "
                                                END IF
                                            END IF
                                        NEXT
                                        e$ = e$ + ")"
                                    END IF
                                    incerror$ = e$
                                    linenumber = linenumber - 1 'lower official linenumber to counter later increment

                                    IF Debug THEN PRINT #9, "Pre-pass:Feeding INCLUDE$ line:[" + wholeline$ + "]"

                                    GOTO prepassline
                                END IF
                                IF LEN(classSyntaxDeferredQueue$) THEN
                                    wholeline$ = ClassSyntax_DequeueDeferredLine$
                                    linenumber = linenumber - 1

                                    IF Debug THEN PRINT #9, "Pre-pass:Feeding deferred CLASS line:[" + wholeline$ + "]"

                                    GOTO prepassline
                                END IF
                                '3. Close & return control
                                CLOSE #fh
                                inclevel = inclevel - 1
                                IF forceIncludingFile = 1 AND inclevel = 0 THEN
                                    forceIncludingFile = 0
                                    GOTO forceIncludeCompleted_prepass
                                END IF
                            LOOP
                            '(end manager)

                        LOOP

                        'add final line
                        IF lastLineReturn = 0 THEN
                            lastLineReturn = 1
                            lastLine = 1
                            wholeline$ = ""
                            GOTO prepassLastLine
                        END IF

                        IF definingtype THEN definingtype = 0 'ignore this error so that auto-formatting can be performed and catch it again later
                        IF declaringlibrary THEN declaringlibrary = 0 'ignore this error so that auto-formatting can be performed and catch it again later

                        totallinenumber = reallinenumber

                        'prepass finished

                        lineinput3index = 1 'reset input line

                        ResetPrepassManagers

                        'reset altered variables
                        ResetPostPrepassState

                        IF compfailed <> 0 OR HasErrors% THEN
                            IF HasErrors% THEN PrintAllErrors
                            WarnIfStaleOutputBinary
                            CleanupErrorHandler
                            SYSTEM 1
                        END IF

                        OPEN tmpdir$ + "data.bin" FOR OUTPUT AS #16: CLOSE #16
                        OPEN tmpdir$ + "data.bin" FOR BINARY AS #16


                        OPEN tmpdir$ + "main.txt" FOR OUTPUT AS #12
                        OPEN tmpdir$ + "maindata.txt" FOR OUTPUT AS #13

                        OPEN tmpdir$ + "regsf.txt" FOR OUTPUT AS #17

                        OPEN tmpdir$ + "mainfree.txt" FOR OUTPUT AS #19
                        OPEN tmpdir$ + "runline.txt" FOR OUTPUT AS #21

                        OPEN tmpdir$ + "mainerr.txt" FOR OUTPUT AS #14 'main error handler
                        'i. check the value of error_line
                        'ii. jump to the appropriate label
                        errorlabels = 0
                        PRINT #14, "if (error_occurred){ error_occurred=0;"

                        OPEN tmpdir$ + "chain.txt" FOR OUTPUT AS #22: CLOSE #22 'will be appended to as necessary
                        OPEN tmpdir$ + "inpchain.txt" FOR OUTPUT AS #23: CLOSE #23 'will be appended to as necessary
                        '*** #22 & #23 are reserved for usage by chain & inpchain ***

                        OPEN tmpdir$ + "ontimer.txt" FOR OUTPUT AS #24
                        OPEN tmpdir$ + "ontimerj.txt" FOR OUTPUT AS #25

                        '*****#26 used for locking qbnex

                        OPEN tmpdir$ + "onkey.txt" FOR OUTPUT AS #27
                        OPEN tmpdir$ + "onkeyj.txt" FOR OUTPUT AS #28

                        OPEN tmpdir$ + "onstrig.txt" FOR OUTPUT AS #29
                        OPEN tmpdir$ + "onstrigj.txt" FOR OUTPUT AS #30

                        gosubid = 1
                        'to be included whenever return without a label is called

                        'return [label] in QBASIC was not possible in a sub/function, but QBNex will support this
                        'special codes will represent special return conditions:
                        '0=return from main to calling sub/function/proc by return [NULL];
                        '1... a global number representing a return point after a gosub
                        'note: RETURN [label] should fail if a "return [NULL];" type return is required
                        StartMainPassSession


                        DO
                            includeline:
                            mainpassLastLine:

                            IF lastLine <> 0 OR firstLine <> 0 THEN
                                lineBackup$ = a3$ 'backup the real first line (will be blank when lastline is set)
                                forceIncludeFromRoot$ = ""
                                IF vWatchOn THEN
                                    addingvWatch = 1
                                    IF firstLine <> 0 THEN forceIncludeFromRoot$ = "internal\support\vwatch\vwatch.bi"
                                    IF lastLine <> 0 THEN forceIncludeFromRoot$ = "internal\support\vwatch\vwatch.bm"
                                ELSE
                                    'IF firstLine <> 0 THEN forceIncludeFromRoot$ = "internal\support\vwatch\vwatch_stub.bi"
                                    IF lastLine <> 0 THEN forceIncludeFromRoot$ = "internal\support\vwatch\vwatch_stub.bm"
                                END IF
                                firstLine = 0: lastLine = 0
                                IF LEN(forceIncludeFromRoot$) THEN GOTO forceInclude
                                forceIncludeCompleted:
                                addingvWatch = 0
                                a3$ = lineBackup$
                            END IF

                            prepass = 0

                            stringprocessinghappened = 0

                            IF continuelinefrom THEN
                                start = continuelinefrom
                                continuelinefrom = 0
                                GOTO contline
                            END IF

                            'begin a new line

                            impliedendif = 0
                            THENGOTO = 0
                            continueline = 0
                            endifs = 0
                            lineelseused = 0
                            newif = 0

                            'apply metacommands from previous line
                            IF addmetadynamic = 1 THEN addmetadynamic = 0: DynamicMode = 1
                            IF addmetastatic = 1 THEN addmetastatic = 0: DynamicMode = 0

                            'a3$ is passed in when using $include
                            a3$ = NextMainPassLine$(a3$)
                            IF a3$ = CHR$(13) THEN EXIT DO
                            linenumber = linenumber + 1
                            reallinenumber = reallinenumber + 1

                            IF InValidLine(linenumber) THEN
                                layoutok = 1
                                layout$ = SPACE$(controllevel + 1) + LTRIM$(RTRIM$(a3$))
                                GOTO nextmainpassline
                            END IF

                            layout = ""
                            layoutok = 1

                            IF NOT QuietMode THEN
                                IF totallinenumber > 0 THEN
                                    x = (reallinenumber * 100) \ totallinenumber
                                    IF x > 100 THEN x = 100
                                    IF x <> percentage THEN
                                        percentage = x
                                        UpdateCompilerProgress percentage
                                    END IF
                                END IF
                            END IF

                            a3$ = LTRIM$(RTRIM$(a3$))
                            diagnosticSourceLine = a3$
                            wholeline = a3$

                            layoutoriginal$ = a3$
                            layoutcomment$ = "" 'clear any previous layout comment
                            lhscontrollevel = controllevel

                            linefragment = "[INFORMATION UNAVAILABLE]"
                            IF LEN(a3$) = 0 THEN GOTO finishednonexec
                            IF Debug THEN PRINT #9, "########" + a3$ + "########"

                            layoutdone = 1 'validates layout of any following goto finishednonexec/finishedline

                            'We've already figured out in the prepass which lines are invalidated by the precompiler
                            'No need to go over those lines again.
                            'IF InValidLine(linenumber) THEN goto nextmainpassline 'layoutdone = 0: GOTO finishednonexec

                            a3u$ = UCASE$(a3$)

                            'QBNex Metacommands
                            IF ASC(a3$) = 36 THEN '$

                            'precompiler commands should always be executed FIRST.
                            directiveResult = HandleMainPassConditionalDirective%(a3u$)
                            IF directiveResult = 1 THEN GOTO finishednonexec
                            IF directiveResult = 2 THEN GOTO errmes

                            IF ExecLevel(ExecCounter) THEN 'don't check for any more metacommands except the one's which worth with the precompiler
                            layoutdone = 0
                            GOTO finishednonexec 'we don't check for anything inside lines that we've marked for skipping
                        END IF

                        directiveResult = HandleSimpleDirective%(a3u$)
                        IF directiveResult = 1 THEN GOTO finishednonexec
                        IF directiveResult = 2 THEN GOTO errmes
                        IF directiveResult = 3 THEN GOTO finishedline2

                        directiveResult = HandleVersionInfoDirective%(a3u$, a3$)
                        IF directiveResult = 1 THEN GOTO finishednonexec
                        IF directiveResult = 2 THEN GOTO errmes

                        directiveResult = HandleExeIconDirective%(a3u$, a3$)
                        IF directiveResult = 2 THEN GOTO errmes
                        IF directiveResult = 3 THEN GOTO finishedline2

                    END IF 'QBNex Metacommands

                    IF ExecLevel(ExecCounter) THEN
                        layoutdone = 0
                        GOTO finishednonexec 'we don't check for anything inside lines that we've marked for skipping
                    END IF


                    linedataoffset = DataOffset

                    entireline$ = lineformat(a3$): IF LEN(entireline$) = 0 THEN GOTO finishednonexec
                    IF Error_Happened THEN GOTO errmes
                    u$ = UCASE$(entireline$)

                    newif = 0

                    'Convert "CASE ELSE" to "CASE C-EL" to avoid confusing compiler
                    'note: CASE does not have to begin on a new line
                    s = 1
                    i = INSTR(s, u$, "CASE" + sp + "ELSE")
                    DO WHILE i
                        skip = 0
                        IF i <> 1 THEN
                            IF MID$(u$, i - 1, 1) <> sp THEN skip = 1
                        END IF
                        IF i <> LEN(u$) - 8 THEN
                            IF MID$(u$, i + 9, 1) <> sp THEN skip = 1
                        END IF
                        IF skip = 0 THEN
                            MID$(entireline$, i) = "CASE" + sp + "C-EL"
                            u$ = UCASE$(entireline$)
                        END IF
                        s = i + 9
                        i = INSTR(s, u$, "CASE" + sp + "ELSE")
                    LOOP

                    n = numelements(entireline$)

                    'line number?
                    a = ASC(entireline$)
                    IF (a >= 48 AND a <= 57) OR a = 46 THEN 'numeric
                    label$ = getelement(entireline$, 1)
                    IF validlabel(label$) THEN

                        IF closedmain <> 0 AND subfunc = "" THEN a$ = "Labels cannot be placed between SUB/FUNCTIONs": GOTO errmes

                        v = HashFind(label$, HASHFLAG_LABEL, ignore, r)
                        addlabchk100:
                        IF v THEN
                            s = Labels(r).Scope
                            IF s = subfuncn OR s = -1 THEN 'same scope?
                            IF s = -1 THEN Labels(r).Scope = subfuncn 'acquire scope
                            IF Labels(r).State = 1 THEN a$ = "Duplicate label (" + RTRIM$(Labels(r).cn) + ")": GOTO errmes
                            'aquire state 0 types
                            tlayout$ = RTRIM$(Labels(r).cn)
                            GOTO addlabaq100
                        END IF 'same scope
                        IF v = 2 THEN v = HashFindCont(ignore, r): GOTO addlabchk100
                    END IF

                    'does not exist
                    nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
                    Labels(nLabels) = Empty_Label
                    HashAdd label$, HASHFLAG_LABEL, nLabels
                    r = nLabels
                    Labels(r).cn = tlayout$
                    Labels(r).Scope = subfuncn
                    addlabaq100:
                    Labels(r).State = 1
                    Labels(r).Data_Offset = linedataoffset

                    layout$ = tlayout$
                    PRINT #12, "LABEL_" + label$ + ":;"


                    IF INSTR(label$, "p") THEN MID$(label$, INSTR(label$, "p"), 1) = "."
                    IF RIGHT$(label$, 1) = "d" OR RIGHT$(label$, 1) = "s" THEN label$ = LEFT$(label$, LEN(label$) - 1)
                    PRINT #12, "last_line=" + label$ + ";"
                    inclinenump$ = ""
                    IF inclinenumber(inclevel) THEN
                        inclinenump$ = "," + str2$(inclinenumber(inclevel))
                        thisincname$ = getfilepath$(incname$(inclevel))
                        thisincname$ = MID$(incname$(inclevel), LEN(thisincname$) + 1)
                        inclinenump$ = inclinenump$ + "," + CHR$(34) + thisincname$ + CHR$(34)
                    END IF
                    IF NoChecks = 0 THEN
                        IF vWatchOn AND inclinenumber(inclevel) = 0 THEN temp$ = vWatchErrorCall$ ELSE temp$ = ""
                        PRINT #12, "if(qbevent){" + temp$ + "evnt(" + str2$(linenumber) + inclinenump$ + ");r=0;}"
                    END IF
                    IF n = 1 THEN GOTO finishednonexec
                    entireline$ = getelements(entireline$, 2, n): u$ = UCASE$(entireline$): n = n - 1
                    'note: fall through, numeric labels can be followed by alphanumeric label
                END IF 'validlabel
            END IF 'numeric
            'it wasn't a line number

            'label?
            'note: ignores possibility that this could be a single command SUB/FUNCTION (as in QBASIC?)
            IF n >= 2 THEN
                x2 = INSTR(entireline$, sp + ":")
                IF x2 THEN
                    IF x2 = LEN(entireline$) - 1 THEN x3 = x2 + 1 ELSE x3 = x2 + 2
                    a$ = LEFT$(entireline$, x2 - 1)

                    CreatingLabel = 1
                    IF validlabel(a$) THEN

                        IF validname(a$) = 0 THEN a$ = "Invalid name": GOTO errmes

                        IF closedmain <> 0 AND subfunc = "" THEN a$ = "Labels cannot be placed between SUB/FUNCTIONs": GOTO errmes

                        v = HashFind(a$, HASHFLAG_LABEL, ignore, r)
                        addlabchk:
                        IF v THEN
                            s = Labels(r).Scope
                            IF s = subfuncn OR s = -1 THEN 'same scope?
                            IF s = -1 THEN Labels(r).Scope = subfuncn 'acquire scope
                            IF Labels(r).State = 1 THEN a$ = "Duplicate label (" + RTRIM$(Labels(r).cn) + ")": GOTO errmes
                            'aquire state 0 types
                            tlayout$ = RTRIM$(Labels(r).cn)
                            GOTO addlabaq
                        END IF 'same scope
                        IF v = 2 THEN v = HashFindCont(ignore, r): GOTO addlabchk
                    END IF
                    'does not exist
                    nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
                    Labels(nLabels) = Empty_Label
                    HashAdd a$, HASHFLAG_LABEL, nLabels
                    r = nLabels
                    Labels(r).cn = tlayout$
                    Labels(r).Scope = subfuncn
                    addlabaq:
                    Labels(r).State = 1
                    Labels(r).Data_Offset = linedataoffset
                    Labels(r).SourceLineNumber = linenumber

                    IF LEN(layout$) THEN layout$ = layout$ + sp + tlayout$ + ":" ELSE layout$ = tlayout$ + ":"

                    PRINT #12, "LABEL_" + a$ + ":;"
                    inclinenump$ = ""
                    IF inclinenumber(inclevel) THEN
                        inclinenump$ = "," + str2$(inclinenumber(inclevel))
                        thisincname$ = getfilepath$(incname$(inclevel))
                        thisincname$ = MID$(incname$(inclevel), LEN(thisincname$) + 1)
                        inclinenump$ = inclinenump$ + "," + CHR$(34) + thisincname$ + CHR$(34)
                    END IF
                    IF NoChecks = 0 THEN
                        IF vWatchOn AND inclinenumber(inclevel) = 0 THEN temp$ = vWatchErrorCall$ ELSE temp$ = ""
                        PRINT #12, "if(qbevent){" + temp$ + "evnt(" + str2$(linenumber) + inclinenump$ + ");r=0;}"
                    END IF
                    entireline$ = RIGHT$(entireline$, LEN(entireline$) - x3): u$ = UCASE$(entireline$)
                    n = numelements(entireline$): IF n = 0 THEN GOTO finishednonexec
                END IF 'valid
            END IF 'includes sp+":"
        END IF 'n>=2

        'remove leading ":"
        DO WHILE ASC(u$) = 58 '":"
            IF LEN(layout$) THEN layout$ = layout$ + sp2 + ":" ELSE layout$ = ":"
            IF LEN(u$) = 1 THEN GOTO finishednonexec
            entireline$ = getelements(entireline$, 2, n): u$ = UCASE$(entireline$): n = n - 1
        LOOP

        'ELSE at the beginning of a line
        IF ASC(u$) = 69 THEN '"E"

        e1$ = getelement(u$, 1)

        IF e1$ = "ELSE" THEN
            a$ = "ELSE"
            IF n > 1 THEN continuelinefrom = 2
            GOTO gotcommand
        END IF

        IF e1$ = "ELSEIF" THEN
            IF n < 3 THEN a$ = "Expected ... THEN": GOTO errmes
            IF getelement(u$, n) = "THEN" THEN a$ = entireline$: GOTO gotcommand
            FOR i = 3 TO n - 1
                IF getelement(u$, i) = "THEN" THEN
                    a$ = getelements(entireline$, 1, i)
                    continuelinefrom = i + 1
                    GOTO gotcommand
                END IF
            NEXT
            a$ = "Expected THEN": GOTO errmes
        END IF

    END IF '"E"

    start = 1

    GOTO skipcontinit

    contline:

    n = numelements(entireline$)
    u$ = UCASE$(entireline$)

    skipcontinit:

    'jargon:
    'lineelseused - counts how many line ELSEs can POSSIBLY follow
    'endifs - how many C++ endifs "}" need to be added at the end of the line
    'lineelseused - counts the number of indwelling ELSE statements on a line
    'impliedendif - stops autoformat from adding "END IF"

    a$ = ""

    FOR i = start TO n
        e$ = getelement(u$, i)


        IF e$ = ":" THEN
            IF i = start THEN
                layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp2 + ":" ELSE layout$ = ":"
                IF i <> n THEN continuelinefrom = i + 1
                GOTO finishednonexec
            END IF
            IF i <> n THEN continuelinefrom = i
            GOTO gotcommand
        END IF


        'begin scanning an 'IF' statement
        IF e$ = "IF" AND a$ = "" THEN newif = 1


        IF e$ = "THEN" OR (e$ = "GOTO" AND newif = 1) THEN
            IF newif = 0 THEN a$ = "THEN without IF": GOTO errmes
            newif = 0
            IF lineelseused > 0 THEN lineelseused = lineelseused - 1
            IF e$ = "GOTO" THEN
                IF i = n THEN a$ = "Expected IF expression GOTO label": GOTO errmes
                i = i - 1
            END IF
            a$ = a$ + sp + e$ '+"THEN"/"GOTO"
            IF i <> n THEN continuelinefrom = i + 1: endifs = endifs + 1
            GOTO gotcommand
        END IF


        IF e$ = "ELSE" THEN

            IF start = i THEN
                IF lineelseused >= 1 THEN
                    'note: more than one else used (in a row) on this line, so close first if with an 'END IF' first
                    'note: parses 'END IF' then (after continuelinefrom) parses 'ELSE'
                    'consider the following: (square brackets make reading easier)
                    'eg. if a=1 then [if b=2 then c=2 else d=2] else e=3
                    impliedendif = 1: a$ = "END" + sp + "IF"
                    endifs = endifs - 1
                    continuelinefrom = i
                    lineelseused = lineelseused - 1
                    GOTO gotcommand
                END IF
                'follow up previously encountered 'ELSE' by applying 'ELSE'
                a$ = "ELSE": continuelinefrom = i + 1
                lineelseused = lineelseused + 1
                GOTO gotcommand
            END IF 'start=i

            'apply everything up to (but not including) 'ELSE'
            continuelinefrom = i
            GOTO gotcommand
        END IF '"ELSE"


        e$ = getelement(entireline$, i): IF a$ = "" THEN a$ = e$ ELSE a$ = a$ + sp + e$
    NEXT


    'we're reached the end of the line
    IF endifs > 0 THEN
        endifs = endifs - 1
        impliedendif = 1: entireline$ = entireline$ + sp + ":" + sp + "END" + sp + "IF": n = n + 3
        i = i + 1 'skip the ":" (i is now equal to n+2)
        continuelinefrom = i
        GOTO gotcommand
    END IF


    gotcommand:

    dynscope = 0

    ca$ = a$
    a$ = eleucase$(ca$) '***REVISE THIS SECTION LATER***


    layoutdone = 0

    linefragment = a$
    IF Debug THEN PRINT #9, a$
    n = numelements(a$)
    IF n = 0 THEN GOTO finishednonexec

    'convert non-UDT dimensioned periods to _046_
    IF INSTR(ca$, sp + "." + sp) THEN
        a3$ = getelement(ca$, 1)
        except = 0
        aa$ = a3$ + sp 'rebuilt a$ (always has a trailing spacer)
        lastfuse = -1
        FOR x = 2 TO n
            a2$ = getelement(ca$, x)
            IF except = 1 THEN except = 2: GOTO udtperiod 'skip element name
            IF a2$ = "." AND x <> n THEN
                IF except = 2 THEN except = 1: GOTO udtperiod 'sub-element of UDT

                IF a3$ = ")" THEN
                    'assume it was something like typevar(???).x and treat as a UDT
                    except = 1
                    GOTO udtperiod
                END IF

                'find an ID of that type
                try = findid(UCASE$(a3$))
                IF Error_Happened THEN GOTO errmes
                DO WHILE try
                    IF ((id.t AND ISUDT) <> 0) OR ((id.arraytype AND ISUDT) <> 0) THEN
                        except = 1
                        GOTO udtperiod
                    END IF
                    IF try = 2 THEN findanotherid = 1: try = findid(UCASE$(a3$)) ELSE try = 0
                    IF Error_Happened THEN GOTO errmes
                LOOP
                'not a udt; fuse lhs & rhs with _046_
                IF isalpha(ASC(a3$)) = 0 AND lastfuse <> x - 2 THEN a$ = "Invalid '.'": GOTO errmes
                aa$ = LEFT$(aa$, LEN(aa$) - 1) + fix046$
                lastfuse = x
                GOTO periodfused
            END IF '"."
            except = 0
            udtperiod:
            aa$ = aa$ + a2$ + sp
            periodfused:
            a3$ = a2$
        NEXT
        a$ = LEFT$(aa$, LEN(aa$) - 1)
        ca$ = a$
        a$ = eleucase$(ca$)
        n = numelements(a$)
    END IF

    arrayprocessinghappened = 0

    firstelement$ = getelement(a$, 1)
    secondelement$ = getelement(a$, 2)
    thirdelement$ = getelement(a$, 3)

    'non-executable section

    IF n = 1 THEN
        IF firstelement$ = "'" THEN layoutdone = 1: GOTO finishednonexec 'nop
    END IF

    IF n <= 2 THEN
        IF firstelement$ = "DATA" THEN
            l$ = SCase$("Data")
            IF n = 2 THEN

                e$ = SPACE$((LEN(secondelement$) - 1) \ 2)
                FOR x = 1 TO LEN(e$)
                    v1 = ASC(secondelement$, x * 2)
                    v2 = ASC(secondelement$, x * 2 + 1)
                    IF v1 < 65 THEN v1 = v1 - 48 ELSE v1 = v1 - 55
                    IF v2 < 65 THEN v2 = v2 - 48 ELSE v2 = v2 - 55
                    ASC(e$, x) = v1 + v2 * 16
                NEXT
                l$ = l$ + sp + e$
            END IF 'n=2

            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$

            GOTO finishednonexec
        END IF
    END IF



    'declare library
    IF declaringlibrary THEN

        IF firstelement$ = "END" THEN
            IF n <> 2 OR secondelement$ <> "DECLARE" THEN a$ = "Expected END DECLARE": GOTO errmes
            declaringlibrary = 0
            l$ = SCase$("End" + sp + "Declare")
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishednonexec
        END IF 'end declare

        declaringlibrary = 2

        IF firstelement$ = "SUB" OR firstelement$ = "FUNCTION" THEN
            GOTO declaresubfunc2
        END IF

        a$ = "Expected SUB/FUNCTION definition or END DECLARE": GOTO errmes
    END IF 'declaringlibrary

    'check TYPE declarations (created on prepass)
    IF definingtype THEN

        IF firstelement$ = "END" THEN
            IF n <> 2 OR secondelement$ <> "TYPE" THEN a$ = "Expected END TYPE": GOTO errmes
            definingtype = 0
            l$ = SCase$("End" + sp + "Type")
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishednonexec
        END IF

        'IF n < 3 THEN definingtypeerror = linenumber: a$ = "Expected element-name AS type or AS type element-list": GOTO errmes
        IF n < 3 THEN a$ = "Expected element-name AS type or AS type element-list": GOTO errmes
        definingtype = 2
        IF firstelement$ = "AS" THEN
            l$ = SCase$("As")
            t$ = ""
            wordsInTypeName = 0
            DO
                nextElement$ = getelement$(a$, 2 + wordsInTypeName)
                IF nextElement$ = "," THEN
                    'element-list
                    wordsInTypeName = wordsInTypeName - 2
                    EXIT DO
                END IF

                wordsInTypeName = wordsInTypeName + 1
                IF wordsInTypeName = n - 2 THEN
                    'single element in line
                    wordsInTypeName = wordsInTypeName - 1
                    EXIT DO
                END IF
            LOOP

            t$ = getelements$(a$, 2, 2 + wordsInTypeName)
            typ = typname2typ(t$)
            IF Error_Happened THEN GOTO errmes
            IF typ = 0 THEN a$ = "Undefined type": GOTO errmes
            IF typ AND ISUDT THEN
                IF UCASE$(RTRIM$(t$)) = "MEM" AND RTRIM$(udtxcname(typ AND 511)) = "_MEM" AND qbnexprefix_set = 1 THEN
                    t$ = MID$(RTRIM$(udtxcname(typ AND 511)), 2)
                ELSE
                    t$ = RTRIM$(udtxcname(typ AND 511))
                END IF
                l$ = l$ + sp + t$
            ELSE
                l$ = l$ + sp + SCase2$(t$)
            END IF

            'Now add each variable:
            FOR i = 3 + wordsInTypeName TO n
                thisElement$ = getelement$(ca$, i)
                IF thisElement$ = "," THEN
                    l$ = l$ + thisElement$
                ELSE
                    l$ = l$ + sp + thisElement$
                END IF
            NEXT
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        ELSE
            l$ = getelement(ca$, 1) + sp + SCase$("As")
            t$ = getelements$(a$, 3, n)
            typ = typname2typ(t$)
            IF Error_Happened THEN GOTO errmes
            IF typ = 0 THEN a$ = "Undefined type": GOTO errmes
            IF typ AND ISUDT THEN
                IF UCASE$(RTRIM$(t$)) = "MEM" AND RTRIM$(udtxcname(typ AND 511)) = "_MEM" AND qbnexprefix_set = 1 THEN
                    t$ = MID$(RTRIM$(udtxcname(typ AND 511)), 2)
                ELSE
                    t$ = RTRIM$(udtxcname(typ AND 511))
                END IF
                l$ = l$ + sp + t$
            ELSE
                l$ = l$ + sp + SCase2$(t$)
            END IF
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        END IF
        GOTO finishednonexec

    END IF 'defining type

    IF firstelement$ = "TYPE" THEN
        IF n <> 2 THEN a$ = "Expected TYPE type-name": GOTO errmes
        l$ = SCase$("Type") + sp + getelement(ca$, 2)
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        definingtype = 1
        definingtypeerror = linenumber
        GOTO finishednonexec
    END IF

    'skip DECLARE SUB/FUNCTION
    IF n >= 1 THEN
        IF firstelement$ = "DECLARE" THEN

            IF secondelement$ = "LIBRARY" OR secondelement$ = "DYNAMIC" OR secondelement$ = "CUSTOMTYPE" OR secondelement$ = "STATIC" THEN

                declaringlibrary = 1
                dynamiclibrary = 0
                customtypelibrary = 0
                indirectlibrary = 0
                staticlinkedlibrary = 0

                x = 3
                l$ = SCase$("Declare" + sp + "Library")

                IF secondelement$ = "DYNAMIC" THEN
                    e$ = getelement$(a$, 3): IF e$ <> "LIBRARY" THEN a$ = "Expected DYNAMIC LIBRARY " + CHR$(34) + "..." + CHR$(34): GOTO errmes
                    dynamiclibrary = 1
                    x = 4
                    l$ = SCase$("Declare" + sp + "Dynamic" + sp + "Library")
                    IF n = 3 THEN a$ = "Expected DECLARE DYNAMIC LIBRARY " + CHR$(34) + "..." + CHR$(34): GOTO errmes
                    indirectlibrary = 1
                END IF

                IF secondelement$ = "CUSTOMTYPE" THEN
                    e$ = getelement$(a$, 3): IF e$ <> "LIBRARY" THEN a$ = "Expected CUSTOMTYPE LIBRARY": GOTO errmes
                    customtypelibrary = 1
                    x = 4
                    l$ = SCase$("Declare" + sp + "CustomType" + sp + "Library")
                    indirectlibrary = 1
                END IF

                IF secondelement$ = "STATIC" THEN
                    e$ = getelement$(a$, 3): IF e$ <> "LIBRARY" THEN a$ = "Expected STATIC LIBRARY": GOTO errmes
                    x = 4
                    l$ = SCase$("Declare" + sp + "Static" + sp + "Library")
                    staticlinkedlibrary = 1
                END IF

                sfdeclare = 0: sfheader = 0

                IF n >= x THEN

                    sfdeclare = 1

                    addlibrary:

                    libname$ = ""
                    headername$ = ""


                    'assume library name in double quotes follows
                    'assume library is in main qbnex folder
                    x$ = getelement$(ca$, x)
                    IF ASC(x$) <> 34 THEN a$ = "Expected LIBRARY " + CHR$(34) + "..." + CHR$(34): GOTO errmes
                    x$ = RIGHT$(x$, LEN(x$) - 1)
                    z = INSTR(x$, CHR$(34))
                    IF z = 0 THEN a$ = "Expected LIBRARY " + CHR$(34) + "..." + CHR$(34): GOTO errmes
                    x$ = LEFT$(x$, z - 1)

                    IF dynamiclibrary <> 0 AND LEN(x$) = 0 THEN a$ = "Expected DECLARE DYNAMIC LIBRARY " + CHR$(34) + "..." + CHR$(34): GOTO errmes
                    IF customtypelibrary <> 0 AND LEN(x$) = 0 THEN a$ = "Expected DECLARE CUSTOMTYPE LIBRARY " + CHR$(34) + "..." + CHR$(34): GOTO errmes













                    'convert '\\' to '\'
                    WHILE INSTR(x$, "\\")
                        z = INSTR(x$, "\\")
                        x$ = LEFT$(x$, z - 1) + RIGHT$(x$, LEN(x$) - z)
                    WEND

                    autoformat_x$ = x$ 'used for autolayout purposes

                    'Remove version number from library name
                    'Eg. libname:1.0 becomes libname <-> 1.0 which later becomes libname.so.1.0
                    v$ = ""
                    striplibver:
                    FOR z = LEN(x$) TO 1 STEP -1
                        a = ASC(x$, z)
                        IF a = ASC_BACKSLASH OR a = ASC_FORWARDSLASH THEN EXIT FOR
                        IF a = ASC_FULLSTOP OR a = ASC_COLON THEN
                            IF isuinteger(RIGHT$(x$, LEN(x$) - z)) THEN
                                IF LEN(v$) THEN v$ = RIGHT$(x$, LEN(x$) - z) + "." + v$ ELSE v$ = RIGHT$(x$, LEN(x$) - z)
                                x$ = LEFT$(x$, z - 1)
                                IF a = ASC_COLON THEN EXIT FOR
                                GOTO striplibver
                            ELSE
                                EXIT FOR
                            END IF
                        END IF
                    NEXT
                    libver$ = v$


                    IF os$ = "WIN" THEN
                        'convert forward-slashes to back-slashes
                        DO WHILE INSTR(x$, "/")
                            z = INSTR(x$, "/")
                            x$ = LEFT$(x$, z - 1) + "\" + RIGHT$(x$, LEN(x$) - z)
                        LOOP
                    END IF

                    IF os$ = "LNX" THEN
                        'convert any back-slashes to forward-slashes
                        DO WHILE INSTR(x$, "\")
                            z = INSTR(x$, "\")
                            x$ = LEFT$(x$, z - 1) + "/" + RIGHT$(x$, LEN(x$) - z)
                        LOOP
                    END IF

                    'Separate path from name
                    libpath$ = ""
                    FOR z = LEN(x$) TO 1 STEP -1
                        a = ASC(x$, z)
                        IF a = 47 OR a = 92 THEN '\ or /
                        libpath$ = LEFT$(x$, z)
                        x$ = RIGHT$(x$, LEN(x$) - z)
                        EXIT FOR
                    END IF
                NEXT

                'Accept ./ and .\ as a reference to the source file
                'folder, replacing it with the actual full path, if available
                IF libpath$ = "./" OR libpath$ = ".\" THEN
                    libpath$ = ""
                    libpath$ = path.source$
                    IF LEN(libpath$) > 0 AND RIGHT$(libpath$, 1) <> pathsep$ THEN libpath$ = libpath$ + pathsep$
                END IF

                'Create a path which can be used for inline code (uses \\ instead of \)
                libpath_inline$ = ""
                FOR z = 1 TO LEN(libpath$)
                    a = ASC(libpath$, z)
                    libpath_inline$ = libpath_inline$ + CHR$(a)
                    IF a = 92 THEN libpath_inline$ = libpath_inline$ + "\"
                NEXT

                IF LEN(x$) THEN
                    IF dynamiclibrary = 0 THEN
                        'Static library

                        IF os$ = "WIN" THEN
                            'check for .lib
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS(libpath$ + x$ + ".lib") THEN
                                    libname$ = libpath$ + x$ + ".lib"
                                    inlinelibname$ = libpath_inline$ + x$ + ".lib"
                                END IF
                            END IF
                            'check for .a
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS(libpath$ + x$ + ".a") THEN
                                    libname$ = libpath$ + x$ + ".a"
                                    inlinelibname$ = libpath_inline$ + x$ + ".a"
                                END IF
                            END IF
                            'check for .o
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS(libpath$ + x$ + ".o") THEN
                                    libname$ = libpath$ + x$ + ".o"
                                    inlinelibname$ = libpath_inline$ + x$ + ".o"
                                END IF
                            END IF
                            'check for .lib
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS(x$ + ".lib") THEN
                                    libname$ = x$ + ".lib"
                                    inlinelibname$ = x$ + ".lib"
                                END IF
                            END IF
                            'check for .a
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS(x$ + ".a") THEN
                                    libname$ = x$ + ".a"
                                    inlinelibname$ = x$ + ".a"
                                END IF
                            END IF
                            'check for .o
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS(x$ + ".o") THEN
                                    libname$ = x$ + ".o"
                                    inlinelibname$ = x$ + ".o"
                                END IF
                            END IF
                        END IF 'Windows

                        IF os$ = "LNX" THEN
                            IF staticlinkedlibrary = 0 THEN

                                IF MacOSX THEN 'dylib support
                                'check for .dylib (direct)
                                IF LEN(libname$) = 0 THEN
                                    IF _FILEEXISTS(libpath$ + "lib" + x$ + "." + libver$ + ".dylib") THEN
                                        libname$ = libpath$ + "lib" + x$ + "." + libver$ + ".dylib"
                                        inlinelibname$ = libpath_inline$ + "lib" + x$ + "." + libver$ + ".dylib"
                                        IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath " + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath ./ "
                                    END IF
                                END IF
                                IF LEN(libname$) = 0 THEN
                                    IF _FILEEXISTS(libpath$ + "lib" + x$ + ".dylib") THEN
                                        libname$ = libpath$ + "lib" + x$ + ".dylib"
                                        inlinelibname$ = libpath_inline$ + "lib" + x$ + ".dylib"
                                        IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath " + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath ./ "
                                    END IF
                                END IF
                            END IF

                            'check for .so (direct)
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS(libpath$ + "lib" + x$ + ".so." + libver$) THEN
                                    libname$ = libpath$ + "lib" + x$ + ".so." + libver$
                                    inlinelibname$ = libpath_inline$ + "lib" + x$ + ".so." + libver$
                                    IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath " + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath ./ "
                                END IF
                            END IF
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS(libpath$ + "lib" + x$ + ".so") THEN
                                    libname$ = libpath$ + "lib" + x$ + ".so"
                                    inlinelibname$ = libpath_inline$ + "lib" + x$ + ".so"
                                    IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath " + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath ./ "
                                END IF
                            END IF
                        END IF
                        'check for .a (direct)
                        IF LEN(libname$) = 0 THEN
                            IF _FILEEXISTS(libpath$ + "lib" + x$ + ".a") THEN
                                libname$ = libpath$ + "lib" + x$ + ".a"
                                inlinelibname$ = libpath_inline$ + "lib" + x$ + ".a"
                            END IF
                        END IF
                        'check for .o (direct)
                        IF LEN(libname$) = 0 THEN
                            IF _FILEEXISTS(libpath$ + "lib" + x$ + ".o") THEN
                                libname$ = libpath$ + "lib" + x$ + ".o"
                                inlinelibname$ = libpath_inline$ + "lib" + x$ + ".o"
                            END IF
                        END IF
                        IF staticlinkedlibrary = 0 THEN
                            'check for .so (usr/lib64)
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS("/usr/lib64/" + libpath$ + "lib" + x$ + ".so." + libver$) THEN
                                    libname$ = "/usr/lib64/" + libpath$ + "lib" + x$ + ".so." + libver$
                                    inlinelibname$ = "/usr/lib64/" + libpath_inline$ + "lib" + x$ + ".so." + libver$
                                    IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib64/" + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib64/ "
                                END IF
                            END IF
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS("/usr/lib64/" + libpath$ + "lib" + x$ + ".so") THEN
                                    libname$ = "/usr/lib64/" + libpath$ + "lib" + x$ + ".so"
                                    inlinelibname$ = "/usr/lib64/" + libpath_inline$ + "lib" + x$ + ".so"
                                    IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib64/" + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib64/ "
                                END IF
                            END IF
                        END IF
                        'check for .a (usr/lib64)
                        IF LEN(libname$) = 0 THEN
                            IF _FILEEXISTS("/usr/lib64/" + libpath$ + "lib" + x$ + ".a") THEN
                                libname$ = "/usr/lib64/" + libpath$ + "lib" + x$ + ".a"
                                inlinelibname$ = "/usr/lib64/" + libpath_inline$ + "lib" + x$ + ".a"
                            END IF
                        END IF
                        IF staticlinkedlibrary = 0 THEN

                            IF MacOSX THEN 'dylib support
                            'check for .dylib (usr/lib)
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS("/usr/lib/" + libpath$ + "lib" + x$ + "." + libver$ + ".dylib") THEN
                                    libname$ = "/usr/lib/" + libpath$ + "lib" + x$ + "." + libver$ + ".dylib"
                                    inlinelibname$ = "/usr/lib/" + libpath_inline$ + "lib" + x$ + "." + libver$ + ".dylib"
                                    IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/" + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/ "
                                END IF
                            END IF
                            IF LEN(libname$) = 0 THEN
                                IF _FILEEXISTS("/usr/lib/" + libpath$ + "lib" + x$ + ".dylib") THEN
                                    libname$ = "/usr/lib/" + libpath$ + "lib" + x$ + ".dylib"
                                    inlinelibname$ = "/usr/lib/" + libpath_inline$ + "lib" + x$ + ".dylib"
                                    IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/" + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/ "
                                END IF
                            END IF
                        END IF

                        'check for .so (usr/lib)
                        IF LEN(libname$) = 0 THEN
                            IF _FILEEXISTS("/usr/lib/" + libpath$ + "lib" + x$ + ".so." + libver$) THEN
                                libname$ = "/usr/lib/" + libpath$ + "lib" + x$ + ".so." + libver$
                                inlinelibname$ = "/usr/lib/" + libpath_inline$ + "lib" + x$ + ".so." + libver$
                                IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/" + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/ "
                            END IF
                        END IF
                        IF LEN(libname$) = 0 THEN
                            IF _FILEEXISTS("/usr/lib/" + libpath$ + "lib" + x$ + ".so") THEN
                                libname$ = "/usr/lib/" + libpath$ + "lib" + x$ + ".so"
                                inlinelibname$ = "/usr/lib/" + libpath_inline$ + "lib" + x$ + ".so"
                                IF LEN(libpath$) THEN mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/" + libpath$ + " " ELSE mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/ "
                            END IF
                        END IF
                    END IF
                    'check for .a (usr/lib)
                    IF LEN(libname$) = 0 THEN
                        IF _FILEEXISTS("/usr/lib/" + libpath$ + "lib" + x$ + ".a") THEN
                            libname$ = "/usr/lib/" + libpath$ + "lib" + x$ + ".a"
                            inlinelibname$ = "/usr/lib/" + libpath_inline$ + "lib" + x$ + ".a"
                        END IF
                    END IF
                    '--------------------------(without path)------------------------------
                    IF staticlinkedlibrary = 0 THEN

                        IF MacOSX THEN 'dylib support
                        'check for .dylib (direct)
                        IF LEN(libname$) = 0 THEN
                            IF _FILEEXISTS("lib" + x$ + "." + libver$ + ".dylib") THEN
                                libname$ = "lib" + x$ + "." + libver$ + ".dylib"
                                inlinelibname$ = "lib" + x$ + "." + libver$ + ".dylib"
                                mylibopt$ = mylibopt$ + " -Wl,-rpath ./ "
                            END IF
                        END IF
                        IF LEN(libname$) = 0 THEN
                            IF _FILEEXISTS("lib" + x$ + ".dylib") THEN
                                libname$ = "lib" + x$ + ".dylib"
                                inlinelibname$ = "lib" + x$ + ".dylib"
                                mylibopt$ = mylibopt$ + " -Wl,-rpath ./ "
                            END IF
                        END IF
                    END IF

                    'check for .so (direct)
                    IF LEN(libname$) = 0 THEN
                        IF _FILEEXISTS("lib" + x$ + ".so." + libver$) THEN
                            libname$ = "lib" + x$ + ".so." + libver$
                            inlinelibname$ = "lib" + x$ + ".so." + libver$
                            mylibopt$ = mylibopt$ + " -Wl,-rpath ./ "
                        END IF
                    END IF
                    IF LEN(libname$) = 0 THEN
                        IF _FILEEXISTS("lib" + x$ + ".so") THEN
                            libname$ = "lib" + x$ + ".so"
                            inlinelibname$ = "lib" + x$ + ".so"
                            mylibopt$ = mylibopt$ + " -Wl,-rpath ./ "
                        END IF
                    END IF
                END IF
                'check for .a (direct)
                IF LEN(libname$) = 0 THEN
                    IF _FILEEXISTS("lib" + x$ + ".a") THEN
                        libname$ = "lib" + x$ + ".a"
                        inlinelibname$ = "lib" + x$ + ".a"
                    END IF
                END IF
                'check for .o (direct)
                IF LEN(libname$) = 0 THEN
                    IF _FILEEXISTS("lib" + x$ + ".o") THEN
                        libname$ = "lib" + x$ + ".o"
                        inlinelibname$ = "lib" + x$ + ".o"
                    END IF
                END IF
                IF staticlinkedlibrary = 0 THEN
                    'check for .so (usr/lib64)
                    IF LEN(libname$) = 0 THEN
                        IF _FILEEXISTS("/usr/lib64/" + "lib" + x$ + ".so." + libver$) THEN
                            libname$ = "/usr/lib64/" + "lib" + x$ + ".so." + libver$
                            inlinelibname$ = "/usr/lib64/" + "lib" + x$ + ".so." + libver$
                            mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib64/ "
                        END IF
                    END IF
                    IF LEN(libname$) = 0 THEN
                        IF _FILEEXISTS("/usr/lib64/" + "lib" + x$ + ".so") THEN
                            libname$ = "/usr/lib64/" + "lib" + x$ + ".so"
                            inlinelibname$ = "/usr/lib64/" + "lib" + x$ + ".so"
                            mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib64/ "
                        END IF
                    END IF
                END IF
                'check for .a (usr/lib64)
                IF LEN(libname$) = 0 THEN
                    IF _FILEEXISTS("/usr/lib64/" + "lib" + x$ + ".a") THEN
                        libname$ = "/usr/lib64/" + "lib" + x$ + ".a"
                        inlinelibname$ = "/usr/lib64/" + "lib" + x$ + ".a"
                    END IF
                END IF
                IF staticlinkedlibrary = 0 THEN

                    IF MacOSX THEN 'dylib support
                    'check for .dylib (usr/lib)
                    IF LEN(libname$) = 0 THEN
                        IF _FILEEXISTS("/usr/lib/" + "lib" + x$ + "." + libver$ + ".dylib") THEN
                            libname$ = "/usr/lib/" + "lib" + x$ + "." + libver$ + ".dylib"
                            inlinelibname$ = "/usr/lib/" + "lib" + x$ + "." + libver$ + ".dylib"
                        END IF
                    END IF
                    IF LEN(libname$) = 0 THEN
                        IF _FILEEXISTS("/usr/lib/" + "lib" + x$ + ".dylib") THEN
                            libname$ = "/usr/lib/" + "lib" + x$ + ".dylib"
                            inlinelibname$ = "/usr/lib/" + "lib" + x$ + ".dylib"
                            mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/ "
                        END IF
                    END IF
                END IF

                'check for .so (usr/lib)
                IF LEN(libname$) = 0 THEN
                    IF _FILEEXISTS("/usr/lib/" + "lib" + x$ + ".so." + libver$) THEN
                        libname$ = "/usr/lib/" + "lib" + x$ + ".so." + libver$
                        inlinelibname$ = "/usr/lib/" + "lib" + x$ + ".so." + libver$
                    END IF
                END IF
                IF LEN(libname$) = 0 THEN
                    IF _FILEEXISTS("/usr/lib/" + "lib" + x$ + ".so") THEN
                        libname$ = "/usr/lib/" + "lib" + x$ + ".so"
                        inlinelibname$ = "/usr/lib/" + "lib" + x$ + ".so"
                        mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/ "
                    END IF
                END IF
            END IF
            'check for .a (usr/lib)
            IF LEN(libname$) = 0 THEN
                IF _FILEEXISTS("/usr/lib/" + "lib" + x$ + ".a") THEN
                    libname$ = "/usr/lib/" + "lib" + x$ + ".a"
                    inlinelibname$ = "/usr/lib/" + "lib" + x$ + ".a"
                    mylibopt$ = mylibopt$ + " -Wl,-rpath /usr/lib/ "
                END IF
            END IF
        END IF 'Linux


        'check for header
        IF LEN(headername$) = 0 THEN
            IF os$ = "WIN" THEN
                IF _FILEEXISTS(libpath$ + x$ + ".h") THEN
                    headername$ = libpath_inline$ + x$ + ".h"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                IF _FILEEXISTS(libpath$ + x$ + ".hpp") THEN
                    headername$ = libpath_inline$ + x$ + ".hpp"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                '--------------------------(without path)------------------------------
                IF _FILEEXISTS(x$ + ".h") THEN
                    headername$ = x$ + ".h"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                IF _FILEEXISTS(x$ + ".hpp") THEN
                    headername$ = x$ + ".hpp"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
            END IF 'Windows

            IF os$ = "LNX" THEN
                IF _FILEEXISTS(libpath$ + x$ + ".h") THEN
                    headername$ = libpath_inline$ + x$ + ".h"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                IF _FILEEXISTS(libpath$ + x$ + ".hpp") THEN
                    headername$ = libpath_inline$ + x$ + ".hpp"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                IF _FILEEXISTS("/usr/include/" + libpath$ + x$ + ".h") THEN
                    headername$ = "/usr/include/" + libpath_inline$ + x$ + ".h"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                IF _FILEEXISTS("/usr/include/" + libpath$ + x$ + ".hpp") THEN
                    headername$ = "/usr/include/" + libpath_inline$ + x$ + ".hpp"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                '--------------------------(without path)------------------------------
                IF _FILEEXISTS(x$ + ".h") THEN
                    headername$ = x$ + ".h"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                IF _FILEEXISTS(x$ + ".hpp") THEN
                    headername$ = x$ + ".hpp"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                IF _FILEEXISTS("/usr/include/" + x$ + ".h") THEN
                    headername$ = "/usr/include/" + x$ + ".h"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
                IF _FILEEXISTS("/usr/include/" + x$ + ".hpp") THEN
                    headername$ = "/usr/include/" + x$ + ".hpp"
                    IF customtypelibrary = 0 THEN sfdeclare = 0
                    sfheader = 1
                    GOTO GotHeader
                END IF
            END IF 'Linux

            GotHeader:
        END IF

    ELSE
        'dynamic library

        IF os$ = "WIN" THEN
            'check for .dll (direct)
            IF LEN(libname$) = 0 THEN
                IF _FILEEXISTS(libpath$ + x$ + ".dll") THEN
                    libname$ = libpath$ + x$ + ".dll"
                    inlinelibname$ = libpath_inline$ + x$ + ".dll"
                END IF
            END IF
            'check for .dll (system32)
            IF LEN(libname$) = 0 THEN
                IF _FILEEXISTS(ENVIRON$("SYSTEMROOT") + "\System32\" + libpath$ + x$ + ".dll") THEN
                    libname$ = libpath$ + x$ + ".dll"
                    inlinelibname$ = libpath_inline$ + x$ + ".dll"
                END IF
            END IF
            '--------------------------(without path)------------------------------
            'check for .dll (direct)
            IF LEN(libname$) = 0 THEN
                IF _FILEEXISTS(x$ + ".dll") THEN
                    libname$ = x$ + ".dll"
                    inlinelibname$ = x$ + ".dll"
                END IF
            END IF
            'check for .dll (system32)
            IF LEN(libname$) = 0 THEN
                IF _FILEEXISTS(ENVIRON$("SYSTEMROOT") + "\System32\" + x$ + ".dll") THEN
                    libname$ = x$ + ".dll"
                    inlinelibname$ = x$ + ".dll"
                END IF
            END IF
        END IF 'Windows

        IF os$ = "LNX" THEN
            'Note: STATIC libraries (.a/.o) cannot be loaded as dynamic objects


            IF MacOSX THEN 'dylib support
            'check for .dylib (direct)
            IF LEN(libname$) = 0 THEN
                IF _FILEEXISTS(libpath$ + "lib" + x$ + "." + libver$ + ".dylib") THEN
                    libname$ = libpath$ + "lib" + x$ + "." + libver$ + ".dylib"
                    inlinelibname$ = libpath_inline$ + "lib" + x$ + "." + libver$ + ".dylib"
                    IF LEFT$(libpath$, 1) <> "/" THEN libname$ = "./" + libname$: inlinelibname$ = "./" + inlinelibname$
                END IF
            END IF
            IF LEN(libname$) = 0 THEN
                IF _FILEEXISTS(libpath$ + "lib" + x$ + ".dylib") THEN
                    libname$ = libpath$ + "lib" + x$ + ".dylib"
                    inlinelibname$ = libpath_inline$ + "lib" + x$ + ".dylib"
                    IF LEFT$(libpath$, 1) <> "/" THEN libname$ = "./" + libname$: inlinelibname$ = "./" + inlinelibname$
                END IF
            END IF
        END IF

        'check for .so (direct)
        IF LEN(libname$) = 0 THEN
            IF _FILEEXISTS(libpath$ + "lib" + x$ + ".so." + libver$) THEN
                libname$ = libpath$ + "lib" + x$ + ".so." + libver$
                inlinelibname$ = libpath_inline$ + "lib" + x$ + ".so." + libver$
                IF LEFT$(libpath$, 1) <> "/" THEN libname$ = "./" + libname$: inlinelibname$ = "./" + inlinelibname$
            END IF
        END IF
        IF LEN(libname$) = 0 THEN
            IF _FILEEXISTS(libpath$ + "lib" + x$ + ".so") THEN
                libname$ = libpath$ + "lib" + x$ + ".so"
                inlinelibname$ = libpath_inline$ + "lib" + x$ + ".so"
                IF LEFT$(libpath$, 1) <> "/" THEN libname$ = "./" + libname$: inlinelibname$ = "./" + inlinelibname$
            END IF
        END IF
        'check for .so (usr/lib64)
        IF LEN(libname$) = 0 THEN
            IF _FILEEXISTS("/usr/lib64/" + libpath$ + "lib" + x$ + ".so." + libver$) THEN
                libname$ = "/usr/lib64/" + libpath$ + "lib" + x$ + ".so." + libver$
                inlinelibname$ = "/usr/lib64/" + libpath_inline$ + "lib" + x$ + ".so." + libver$
            END IF
        END IF
        IF LEN(libname$) = 0 THEN
            IF _FILEEXISTS("/usr/lib64/" + libpath$ + "lib" + x$ + ".so") THEN
                libname$ = "/usr/lib64/" + libpath$ + "lib" + x$ + ".so"
                inlinelibname$ = "/usr/lib64/" + libpath_inline$ + "lib" + x$ + ".so"
            END IF
        END IF

        IF MacOSX THEN 'dylib support
        'check for .dylib (usr/lib)
        IF LEN(libname$) = 0 THEN
            IF _FILEEXISTS("/usr/lib/" + libpath$ + "lib" + x$ + "." + libver$ + ".dylib") THEN
                libname$ = "/usr/lib/" + libpath$ + "lib" + x$ + "." + libver$ + ".dylib"
                inlinelibname$ = "/usr/lib/" + libpath_inline$ + "lib" + x$ + "." + libver$ + ".dylib"
            END IF
        END IF
        IF LEN(libname$) = 0 THEN
            IF _FILEEXISTS("/usr/lib/" + libpath$ + "lib" + x$ + ".dylib") THEN
                libname$ = "/usr/lib/" + libpath$ + "lib" + x$ + ".dylib"
                inlinelibname$ = "/usr/lib/" + libpath_inline$ + "lib" + x$ + ".dylib"
            END IF
        END IF
    END IF

    'check for .so (usr/lib)
    IF LEN(libname$) = 0 THEN
        IF _FILEEXISTS("/usr/lib/" + libpath$ + "lib" + x$ + ".so." + libver$) THEN
            libname$ = "/usr/lib/" + libpath$ + "lib" + x$ + ".so." + libver$
            inlinelibname$ = "/usr/lib/" + libpath_inline$ + "lib" + x$ + ".so." + libver$
        END IF
    END IF
    IF LEN(libname$) = 0 THEN
        IF _FILEEXISTS("/usr/lib/" + libpath$ + "lib" + x$ + ".so") THEN
            libname$ = "/usr/lib/" + libpath$ + "lib" + x$ + ".so"
            inlinelibname$ = "/usr/lib/" + libpath_inline$ + "lib" + x$ + ".so"
        END IF
    END IF
    '--------------------------(without path)------------------------------
    IF MacOSX THEN 'dylib support
    'check for .dylib (direct)
    IF LEN(libname$) = 0 THEN
        IF _FILEEXISTS("lib" + x$ + "." + libver$ + ".dylib") THEN
            libname$ = "lib" + x$ + "." + libver$ + ".dylib"
            inlinelibname$ = "lib" + x$ + "." + libver$ + ".dylib"
            libname$ = "./" + libname$: inlinelibname$ = "./" + inlinelibname$
        END IF
    END IF
    IF LEN(libname$) = 0 THEN
        IF _FILEEXISTS("lib" + x$ + ".dylib") THEN
            libname$ = "lib" + x$ + ".dylib"
            inlinelibname$ = "lib" + x$ + ".dylib"
            libname$ = "./" + libname$: inlinelibname$ = "./" + inlinelibname$
        END IF
    END IF
END IF

'check for .so (direct)
IF LEN(libname$) = 0 THEN
    IF _FILEEXISTS("lib" + x$ + ".so." + libver$) THEN
        libname$ = "lib" + x$ + ".so." + libver$
        inlinelibname$ = "lib" + x$ + ".so." + libver$
        libname$ = "./" + libname$: inlinelibname$ = "./" + inlinelibname$
    END IF
END IF
IF LEN(libname$) = 0 THEN
    IF _FILEEXISTS("lib" + x$ + ".so") THEN
        libname$ = "lib" + x$ + ".so"
        inlinelibname$ = "lib" + x$ + ".so"
        libname$ = "./" + libname$: inlinelibname$ = "./" + inlinelibname$
    END IF
END IF
'check for .so (usr/lib64)
IF LEN(libname$) = 0 THEN
    IF _FILEEXISTS("/usr/lib64/" + "lib" + x$ + ".so." + libver$) THEN
        libname$ = "/usr/lib64/" + "lib" + x$ + ".so." + libver$
        inlinelibname$ = "/usr/lib64/" + "lib" + x$ + ".so." + libver$
    END IF
END IF
IF LEN(libname$) = 0 THEN
    IF _FILEEXISTS("/usr/lib64/" + "lib" + x$ + ".so") THEN
        libname$ = "/usr/lib64/" + "lib" + x$ + ".so"
        inlinelibname$ = "/usr/lib64/" + "lib" + x$ + ".so"
    END IF
END IF

IF MacOSX THEN 'dylib support
'check for .dylib (usr/lib)
IF LEN(libname$) = 0 THEN
    IF _FILEEXISTS("/usr/lib/" + "lib" + x$ + "." + libver$ + ".dylib") THEN
        libname$ = "/usr/lib/" + "lib" + x$ + "." + libver$ + ".dylib"
        inlinelibname$ = "/usr/lib/" + "lib" + x$ + "." + libver$ + ".dylib"
    END IF
END IF
IF LEN(libname$) = 0 THEN
    IF _FILEEXISTS("/usr/lib/" + "lib" + x$ + ".dylib") THEN
        libname$ = "/usr/lib/" + "lib" + x$ + ".dylib"
        inlinelibname$ = "/usr/lib/" + "lib" + x$ + ".dylib"
    END IF
END IF
END IF

'check for .so (usr/lib)
IF LEN(libname$) = 0 THEN
    IF _FILEEXISTS("/usr/lib/" + "lib" + x$ + ".so." + libver$) THEN
        libname$ = "/usr/lib/" + "lib" + x$ + ".so." + libver$
        inlinelibname$ = "/usr/lib/" + "lib" + x$ + ".so." + libver$
    END IF
END IF
IF LEN(libname$) = 0 THEN
    IF _FILEEXISTS("/usr/lib/" + "lib" + x$ + ".so") THEN
        libname$ = "/usr/lib/" + "lib" + x$ + ".so"
        inlinelibname$ = "/usr/lib/" + "lib" + x$ + ".so"
    END IF
END IF
END IF 'Linux

END IF 'Dynamic

'library found?
IF dynamiclibrary <> 0 AND LEN(libname$) = 0 THEN a$ = "DYNAMIC LIBRARY not found": GOTO errmes
IF LEN(libname$) = 0 AND LEN(headername$) = 0 THEN a$ = "LIBRARY not found": GOTO errmes

'***actual method should cull redundant header and library entries***

IF dynamiclibrary = 0 THEN

    'static
    IF LEN(libname$) THEN
        IF os$ = "WIN" THEN
            IF MID$(libname$, 2, 1) = ":" OR LEFT$(libname$, 1) = "\" THEN
                mylib$ = mylib$ + " " + libname$ + " "
            ELSE
                mylib$ = mylib$ + " ..\..\" + libname$ + " "
            END IF
        END IF
        IF os$ = "LNX" THEN
            IF LEFT$(libname$, 1) = "/" THEN
                mylib$ = mylib$ + " " + libname$ + " "
            ELSE
                mylib$ = mylib$ + " ../../" + libname$ + " "
            END IF
        END IF

    END IF

ELSE

    'dynamic
    IF LEN(headername$) = 0 THEN 'no header

    IF subfuncn THEN
        f = FREEFILE
        OPEN tmpdir$ + "maindata.txt" FOR APPEND AS #f
    ELSE
        f = 13
    END IF

    'make name a C-appropriate variable name
    'by converting everything except numbers and
    'letters to underscores
    x2$ = x$
    FOR x2 = 1 TO LEN(x2$)
        IF ASC(x2$, x2) < 48 THEN ASC(x2$, x2) = 95
        IF ASC(x2$, x2) > 57 AND ASC(x2$, x2) < 65 THEN ASC(x2$, x2) = 95
        IF ASC(x2$, x2) > 90 AND ASC(x2$, x2) < 97 THEN ASC(x2$, x2) = 95
        IF ASC(x2$, x2) > 122 THEN ASC(x2$, x2) = 95
    NEXT
    DLLname$ = x2$

    IF sfdeclare THEN

        IF os$ = "WIN" THEN
            PRINT #17, "HINSTANCE DLL_" + x2$ + "=NULL;"
            PRINT #f, "if (!DLL_" + x2$ + "){"
            PRINT #f, "DLL_" + x2$ + "=LoadLibrary(" + CHR$(34) + inlinelibname$ + CHR$(34) + ");"
            PRINT #f, "if (!DLL_" + x2$ + ") error(259);"
            PRINT #f, "}"
        END IF

        IF os$ = "LNX" THEN
            PRINT #17, "void *DLL_" + x2$ + "=NULL;"
            PRINT #f, "if (!DLL_" + x2$ + "){"
            PRINT #f, "DLL_" + x2$ + "=dlopen(" + CHR$(34) + inlinelibname$ + CHR$(34) + ",RTLD_LAZY);"
            PRINT #f, "if (!DLL_" + x2$ + ") error(259);"
            PRINT #f, "}"
        END IF


    END IF

    IF subfuncn THEN CLOSE #f

END IF 'no header

END IF 'dynamiclibrary

IF LEN(headername$) THEN
    IF os$ = "WIN" THEN
        IF MID$(headername$, 2, 1) = ":" OR LEFT$(headername$, 1) = "\" THEN
            PRINT #17, "#include " + CHR$(34) + headername$ + CHR$(34)
        ELSE
            PRINT #17, "#include " + CHR$(34) + "..\\..\\" + headername$ + CHR$(34)
        END IF
    END IF
    IF os$ = "LNX" THEN

        IF LEFT$(headername$, 1) = "/" THEN
            PRINT #17, "#include " + CHR$(34) + headername$ + CHR$(34)
        ELSE
            PRINT #17, "#include " + CHR$(34) + "../../" + headername$ + CHR$(34)
        END IF

    END IF
END IF

END IF

l$ = l$ + sp + CHR$(34) + autoformat_x$ + CHR$(34)

IF n > x THEN
    IF dynamiclibrary THEN a$ = "Cannot specify multiple DYNAMIC LIBRARY names in a single DECLARE statement": GOTO errmes
    x = x + 1: x2$ = getelement$(a$, x): IF x2$ <> "," THEN a$ = "Expected ,": GOTO errmes
    l$ = l$ + sp2 + ","
    x = x + 1: IF x > n THEN a$ = "Expected , ...": GOTO errmes
    GOTO addlibrary
END IF

END IF 'n>=x

layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
GOTO finishednonexec
END IF

GOTO finishednonexec 'note: no layout required
END IF
END IF

'begin SUB/FUNCTION
IF n >= 1 THEN
    dynamiclibrary = 0
    declaresubfunc2:
    sf = 0
    IF firstelement$ = "FUNCTION" THEN sf = 1
    IF firstelement$ = "SUB" THEN sf = 2
    IF sf THEN

        IF declaringlibrary = 0 THEN
            IF LEN(subfunc) THEN a$ = "Expected END SUB/FUNCTION before " + firstelement$: GOTO errmes
        END IF

        IF n = 1 THEN a$ = "Expected name after SUB/FUNCTION": GOTO errmes
        e$ = getelement$(ca$, 2)
        symbol$ = removesymbol$(e$) '$,%,etc.
        IF Error_Happened THEN GOTO errmes
        IF sf = 2 AND symbol$ <> "" THEN a$ = "Type symbols after a SUB name are invalid": GOTO errmes
        try = findid(e$)
        IF Error_Happened THEN GOTO errmes
        DO WHILE try
            IF id.subfunc = sf THEN GOTO createsf
            IF try = 2 THEN findanotherid = 1: try = findid(e$) ELSE try = 0
            IF Error_Happened THEN GOTO errmes
        LOOP
        a$ = "Unregistered SUB/FUNCTION encountered": GOTO errmes
        createsf:
        IF UCASE$(e$) = "_GL" THEN e$ = "_GL"
        IF firstelement$ = "SUB" THEN
            l$ = SCase$("Sub") + sp + e$ + symbol$
        ELSE
            l$ = SCase$("Function") + sp + e$ + symbol$
        END IF
        id2 = id
        targetid = currentid

        'check for ALIAS
        aliasname$ = RTRIM$(id.cn)
        IF n > 2 THEN
            ee$ = getelement$(a$, 3)
            IF ee$ = "ALIAS" THEN
                IF declaringlibrary = 0 THEN a$ = "ALIAS can only be used with DECLARE LIBRARY": GOTO errmes
                IF n = 3 THEN a$ = "Expected ALIAS name-in-library": GOTO errmes
                ee$ = getelement$(ca$, 4)

                'strip string content (optional)
                IF LEFT$(ee$, 1) = CHR$(34) THEN
                    ee$ = RIGHT$(ee$, LEN(ee$) - 1)
                    x = INSTR(ee$, CHR$(34)): IF x = 0 THEN a$ = "Expected " + CHR$(34): GOTO errmes
                    ee$ = LEFT$(ee$, x - 1)
                    l$ = l$ + sp + SCase$("Alias") + sp + CHR_QUOTE + ee$ + CHR_QUOTE
                ELSE
                    l$ = l$ + sp + SCase$("Alias") + sp + ee$
                END IF

                'strip fix046$ (created by unquoted periods)
                DO WHILE INSTR(ee$, fix046$)
                    x = INSTR(ee$, fix046$): ee$ = LEFT$(ee$, x - 1) + "." + RIGHT$(ee$, LEN(ee$) - x + 1 - LEN(fix046$))
                LOOP
                aliasname$ = ee$
                'remove ALIAS section from line
                IF n <= 4 THEN a$ = getelements(a$, 1, 2)
                IF n >= 5 THEN a$ = getelements(a$, 1, 2) + sp + getelements(a$, 5, n)
                IF n <= 4 THEN ca$ = getelements(ca$, 1, 2)
                IF n >= 5 THEN ca$ = getelements(ca$, 1, 2) + sp + getelements(ca$, 5, n)
                n = n - 2
            END IF
        END IF

        IF declaringlibrary THEN GOTO declibjmp1


        IF closedmain = 0 THEN closemain

        'check for open controls (copy #2)
        IF controllevel <> 0 AND controltype(controllevel) <> 6 THEN 'It's OK for subs to be inside $IF blocks
        a$ = "Unidentified open control block"
        SELECT CASE controltype(controllevel)
        CASE 1: a$ = "IF without END IF"
        CASE 2: a$ = "FOR without NEXT"
        CASE 3, 4: a$ = "DO without LOOP"
        CASE 5: a$ = "WHILE without WEND"
        CASE 10 TO 19: a$ = "SELECT CASE without END SELECT"
        END SELECT
        linenumber = controlref(controllevel)
        GOTO errmes
    END IF

    IF ideindentsubs THEN
        controllevel = controllevel + 1
        controltype(controllevel) = 32
        controlref(controllevel) = linenumber
    END IF

    subfunc = RTRIM$(id.callname) 'SUB_..."
    IF id.subfunc = 1 THEN subfuncoriginalname$ = "FUNCTION " ELSE subfuncoriginalname$ = "SUB "
    subfuncoriginalname$ = subfuncoriginalname$ + RTRIM$(id.cn)
    subfuncn = subfuncn + 1
    closedsubfunc = 0
    subfuncid = targetid

    subfuncret$ = ""

    CLOSE #13: OPEN tmpdir$ + "data" + str2$(subfuncn) + ".txt" FOR OUTPUT AS #13
    CLOSE #19: OPEN tmpdir$ + "free" + str2$(subfuncn) + ".txt" FOR OUTPUT AS #19
    CLOSE #15: OPEN tmpdir$ + "ret" + str2$(subfuncn) + ".txt" FOR OUTPUT AS #15
    PRINT #15, "if (next_return_point){"
    PRINT #15, "next_return_point--;"
    PRINT #15, "switch(return_point[next_return_point]){"
    PRINT #15, "case 0:"
    PRINT #15, "error(3);" 'return without gosub!
    PRINT #15, "break;"
    defdatahandle = 13

    declibjmp1:

    IF declaringlibrary THEN
        IF sfdeclare = 0 AND indirectlibrary = 0 THEN
            CLOSE #17
            OPEN tmpdir$ + "regsf_ignore.txt" FOR OUTPUT AS #17
        END IF
        IF sfdeclare = 1 AND customtypelibrary = 0 AND dynamiclibrary = 0 AND indirectlibrary = 0 THEN
            PRINT #17, "#include " + CHR$(34) + "externtype" + str2(ResolveStaticFunctions + 1) + ".txt" + CHR$(34)
            fh = FREEFILE: OPEN tmpdir$ + "externtype" + str2(ResolveStaticFunctions + 1) + ".txt" FOR OUTPUT AS #fh: CLOSE #fh
        END IF
    END IF




    IF sf = 1 THEN
        rettyp = id.ret
        t$ = typ2ctyp$(id.ret, "")
        IF Error_Happened THEN GOTO errmes
        IF t$ = "qbs" THEN t$ = "qbs*"

        IF declaringlibrary THEN
            IF rettyp AND ISSTRING THEN
                t$ = "char*"
            END IF
        END IF

        IF declaringlibrary <> 0 AND dynamiclibrary <> 0 THEN
            IF os$ = "WIN" THEN
                PRINT #17, "typedef " + t$ + " (CALLBACK* DLLCALL_" + removecast$(RTRIM$(id.callname)) + ")(";
            END IF
            IF os$ = "LNX" THEN
                PRINT #17, "typedef " + t$ + " (*DLLCALL_" + removecast$(RTRIM$(id.callname)) + ")(";
            END IF
        ELSEIF declaringlibrary <> 0 AND customtypelibrary <> 0 THEN
            PRINT #17, "typedef " + t$ + " CUSTOMCALL_" + removecast$(RTRIM$(id.callname)) + "(";
        ELSE
            PRINT #17, t$ + " " + removecast$(RTRIM$(id.callname)) + "(";
        END IF
        IF declaringlibrary THEN GOTO declibjmp2
        PRINT #12, t$ + " " + removecast$(RTRIM$(id.callname)) + "(";

        'create variable to return result
        'if type wasn't specified, define it
        IF symbol$ = "" THEN
            a = ASC(UCASE$(e$)): IF a = 95 THEN a = 91
            a = a - 64 'so A=1, Z=27 and _=28
            symbol$ = defineextaz(a)
        END IF
        reginternalvariable = 1
        ignore = dim2(e$, symbol$, 0, "")
        IF Error_Happened THEN GOTO errmes
        reginternalvariable = 0
        'the following line stops the return variable from being free'd before being returned
        CLOSE #19: OPEN tmpdir$ + "free" + str2$(subfuncn) + ".txt" FOR OUTPUT AS #19
        'create return
        IF (rettyp AND ISSTRING) THEN
            r$ = refer$(str2$(currentid), id.t, 1)
            IF Error_Happened THEN GOTO errmes
            subfuncret$ = subfuncret$ + "qbs_maketmp(" + r$ + ");"
            subfuncret$ = subfuncret$ + "return " + r$ + ";"
        ELSE
            r$ = refer$(str2$(currentid), id.t, 0)
            IF Error_Happened THEN GOTO errmes
            subfuncret$ = "return " + r$ + ";"
        END IF
    ELSE

        IF declaringlibrary <> 0 AND dynamiclibrary <> 0 THEN
            IF os$ = "WIN" THEN
                PRINT #17, "typedef void (CALLBACK* DLLCALL_" + removecast$(RTRIM$(id.callname)) + ")(";
            END IF
            IF os$ = "LNX" THEN
                PRINT #17, "typedef void (*DLLCALL_" + removecast$(RTRIM$(id.callname)) + ")(";
            END IF
        ELSEIF declaringlibrary <> 0 AND customtypelibrary <> 0 THEN
            PRINT #17, "typedef void CUSTOMCALL_" + removecast$(RTRIM$(id.callname)) + "(";
        ELSE
            PRINT #17, "void " + removecast$(RTRIM$(id.callname)) + "(";
        END IF
        IF declaringlibrary THEN GOTO declibjmp2
        PRINT #12, "void " + removecast$(RTRIM$(id.callname)) + "(";
    END IF
    declibjmp2:

    addstatic2layout = 0
    staticsf = 0
    e$ = getelement$(a$, n)
    IF e$ = "STATIC" THEN
        IF declaringlibrary THEN a$ = "STATIC cannot be used in a library declaration": GOTO errmes
        addstatic2layout = 1
        staticsf = 2
        a$ = LEFT$(a$, LEN(a$) - 7): n = n - 1 'remove STATIC
    END IF

    'check items to pass
    params = 0
    AllowLocalName = 1
    IF n > 2 THEN
        e$ = getelement$(a$, 3)
        IF e$ <> "(" THEN a$ = "Expected (": GOTO errmes
        e$ = getelement$(a$, n)
        IF e$ <> ")" THEN a$ = "Expected )": GOTO errmes
        l$ = l$ + sp + "("
        IF n = 4 THEN GOTO nosfparams2
        IF n < 4 THEN a$ = "Expected ( ... )": GOTO errmes
        B = 0
        a2$ = ""
        FOR i = 4 TO n - 1
            e$ = getelement$(ca$, i)
            IF e$ = "(" THEN B = B + 1
            IF e$ = ")" THEN B = B - 1
            IF e$ = "," AND B = 0 THEN
                IF i = n - 1 THEN a$ = "Expected , ... )": GOTO errmes
                getlastparam2:
                IF a2$ = "" THEN a$ = "Expected ... ,": GOTO errmes
                a2$ = LEFT$(a2$, LEN(a2$) - 1)
                'possible format: [BYVAL]a[%][(1)][AS][type]
                params = params + 1
                glinkid = targetid
                glinkarg = params



                IF params > 1 THEN
                    PRINT #17, ",";

                    IF declaringlibrary = 0 THEN
                        PRINT #12, ",";
                    END IF

                END IF
                n2 = numelements(a2$)
                array = 0
                t2$ = ""
                e$ = getelement$(a2$, 1)

                byvalue = 0
                IF UCASE$(e$) = "BYVAL" THEN
                    IF declaringlibrary = 0 THEN a$ = "BYVAL can only be used with DECLARE LIBRARY": GOTO errmes
                    byvalue = 1: a2$ = RIGHT$(a2$, LEN(a2$) - 6)
                    IF RIGHT$(l$, 1) = "(" THEN l$ = l$ + sp2 + SCase$("ByVal") ELSE l$ = l$ + sp + SCase$("Byval")
                    n2 = numelements(a2$): e$ = getelement$(a2$, 1)
                END IF

                IF RIGHT$(l$, 1) = "(" THEN l$ = l$ + sp2 + e$ ELSE l$ = l$ + sp + e$

                n2$ = e$
                dimmethod = 0


                symbol2$ = removesymbol$(n2$)
                IF validname(n2$) = 0 THEN a$ = "Invalid name": GOTO errmes

                IF Error_Happened THEN GOTO errmes
                IF symbol2$ <> "" THEN dimmethod = 1
                m = 0
                FOR i2 = 2 TO n2
                    e$ = getelement$(a2$, i2)
                    IF e$ = "(" THEN
                        IF m <> 0 THEN a$ = "Syntax error - too many opening brackets": GOTO errmes
                        m = 1
                        array = 1
                        l$ = l$ + sp2 + "("
                        GOTO gotaa2
                    END IF
                    IF e$ = ")" THEN
                        IF m <> 1 THEN a$ = "Syntax error - closing bracket without opening bracket": GOTO errmes
                        m = 2
                        l$ = l$ + sp2 + ")"
                        GOTO gotaa2
                    END IF
                    IF UCASE$(e$) = "AS" THEN
                        IF m <> 0 AND m <> 2 THEN a$ = "Syntax error - check your brackets": GOTO errmes
                        m = 3
                        l$ = l$ + sp + SCase$("As")
                        GOTO gotaa2
                    END IF
                    IF m = 1 THEN l$ = l$ + sp + e$: GOTO gotaa2 'ignore contents of option bracket telling how many dimensions (add to layout as is)
                    IF m <> 3 THEN a$ = "Syntax error - check your brackets": GOTO errmes
                    IF t2$ = "" THEN t2$ = e$ ELSE t2$ = t2$ + " " + e$
                    gotaa2:
                NEXT i2
                IF m = 1 THEN a$ = "Syntax error - check your brackets": GOTO errmes
                IF symbol2$ <> "" AND t2$ <> "" THEN a$ = "Syntax error - check parameter types": GOTO errmes


                IF LEN(t2$) THEN 'add type-name after AS
                t2$ = UCASE$(t2$)
                t3$ = t2$
                typ = typname2typ(t3$)
                IF Error_Happened THEN GOTO errmes
                IF typ = 0 THEN a$ = "Undefined type": GOTO errmes
                IF typ AND ISUDT THEN
                    IF RTRIM$(udtxcname(typ AND 511)) = "_MEM" AND UCASE$(t3$) = "MEM" AND qbnexprefix_set = 1 THEN
                        t3$ = MID$(RTRIM$(udtxcname(typ AND 511)), 2)
                    ELSE
                        t3$ = RTRIM$(udtxcname(typ AND 511))
                    END IF
                    l$ = l$ + sp + t3$
                ELSE
                    FOR t3i = 1 TO LEN(t3$)
                        IF ASC(t3$, t3i) = 32 THEN ASC(t3$, t3i) = ASC(sp)
                    NEXT
                    t3$ = SCase2$(t3$)
                    l$ = l$ + sp + t3$
                END IF
            END IF

            IF t2$ = "" THEN t2$ = symbol2$
            IF t2$ = "" THEN
                IF LEFT$(n2$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(n2$)) - 64
                t2$ = defineaz(v)
                dimmethod = 1
            END IF




            IF array = 1 THEN
                IF declaringlibrary THEN a$ = "Arrays cannot be passed to a library": GOTO errmes
                dimsfarray = 1
                'note: id2.nele is currently 0
                nelereq = ASC(MID$(id2.nelereq, params, 1))
                IF nelereq THEN
                    nele = nelereq
                    MID$(id2.nele, params, 1) = CHR$(nele)

                    ids(targetid) = id2

                    ignore = dim2(n2$, t2$, dimmethod, str2$(nele))
                    IF Error_Happened THEN GOTO errmes
                ELSE
                    nele = 1
                    MID$(id2.nele, params, 1) = CHR$(nele)

                    ids(targetid) = id2

                    ignore = dim2(n2$, t2$, dimmethod, "?")
                    IF Error_Happened THEN GOTO errmes
                END IF

                dimsfarray = 0
                r$ = refer$(str2$(currentid), id.t, 1)
                IF Error_Happened THEN GOTO errmes
                PRINT #17, "ptrszint*" + r$;
                PRINT #12, "ptrszint*" + r$;
            ELSE

                IF declaringlibrary THEN
                    'is it a udt?
                    FOR xx = 1 TO lasttype
                        IF t2$ = RTRIM$(udtxname(xx)) THEN
                            PRINT #17, "void*"
                            GOTO decudt
                        ELSEIF RTRIM$(udtxname(xx)) = "_MEM" AND t2$ = "MEM" AND qbnexprefix_set = 1 THEN
                            PRINT #17, "void*"
                            GOTO decudt
                        END IF
                    NEXT
                    t$ = typ2ctyp$(0, t2$)

                    IF Error_Happened THEN GOTO errmes
                    IF t$ = "qbs" THEN
                        t$ = "char*"
                        IF byvalue = 1 THEN a$ = "STRINGs cannot be passed using BYVAL": GOTO errmes
                        byvalue = 1 'use t$ as is
                    END IF
                    IF byvalue THEN PRINT #17, t$; ELSE PRINT #17, t$ + "*";
                    decudt:
                    GOTO declibjmp3
                END IF

                dimsfarray = 1
                ignore = dim2(n2$, t2$, dimmethod, "")
                IF Error_Happened THEN GOTO errmes


                dimsfarray = 0
                t$ = ""
                typ = id.t 'the typ of the ID created by dim2

                t$ = typ2ctyp$(typ, "")
                IF Error_Happened THEN GOTO errmes



                IF t$ = "" THEN a$ = "Cannot find C type to return array data": GOTO errmes
                'searchpoint
                'get the name of the variable
                r$ = refer$(str2$(currentid), id.t, 1)
                IF Error_Happened THEN GOTO errmes
                PRINT #17, t$ + "*" + r$;
                PRINT #12, t$ + "*" + r$;
                IF t$ = "qbs" THEN
                    u$ = str2$(uniquenumber)
                    PRINT #13, "qbs*oldstr" + u$ + "=NULL;"
                    PRINT #13, "if(" + r$ + "->tmp||" + r$ + "->fixed||" + r$ + "->readonly){"
                    PRINT #13, "oldstr" + u$ + "=" + r$ + ";"

                    PRINT #13, "if (oldstr" + u$ + "->cmem_descriptor){"
                    PRINT #13, r$ + "=qbs_new_cmem(oldstr" + u$ + "->len,0);"
                    PRINT #13, "}else{"
                    PRINT #13, r$ + "=qbs_new(oldstr" + u$ + "->len,0);"
                    PRINT #13, "}"

                    PRINT #13, "memcpy(" + r$ + "->chr,oldstr" + u$ + "->chr,oldstr" + u$ + "->len);"
                    PRINT #13, "}"

                    PRINT #19, "if(oldstr" + u$ + "){"
                    PRINT #19, "if(oldstr" + u$ + "->fixed)qbs_set(oldstr" + u$ + "," + r$ + ");"
                    PRINT #19, "qbs_free(" + r$ + ");"
                    PRINT #19, "}"
                END IF
            END IF
            declibjmp3:
            IF i <> n - 1 THEN l$ = l$ + sp2 + ","

            a2$ = ""
        ELSE
            a2$ = a2$ + e$ + sp
            IF i = n - 1 THEN GOTO getlastparam2
        END IF
    NEXT i
    nosfparams2:
    l$ = l$ + sp2 + ")"
END IF 'n>2
AllowLocalName = 0

IF addstatic2layout THEN l$ = l$ + sp + SCase$("Static")
layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$

PRINT #17, ");"

IF declaringlibrary THEN GOTO declibjmp4

PRINT #12, "){"
PRINT #12, "qbs *tqbs;"
PRINT #12, "ptrszint tmp_long;"
PRINT #12, "int32 tmp_fileno;"
PRINT #12, "uint32 qbs_tmp_base=qbs_tmp_list_nexti;"
PRINT #12, "uint8 *tmp_mem_static_pointer=mem_static_pointer;"
PRINT #12, "uint32 tmp_cmem_sp=cmem_sp;"
PRINT #12, "#include " + CHR$(34) + "data" + str2$(subfuncn) + ".txt" + CHR$(34)

'create new _MEM lock for this scope
PRINT #12, "mem_lock *sf_mem_lock;" 'MUST not be static for recursion reasons
PRINT #12, "new_mem_lock();"
PRINT #12, "sf_mem_lock=mem_lock_tmp;"
PRINT #12, "sf_mem_lock->type=3;"

IF vWatchOn = 1 THEN
    PRINT #12, "*__LONG_VWATCH_SUBLEVEL=*__LONG_VWATCH_SUBLEVEL+ 1 ;"
    IF subfunc <> "SUB_VWATCH" THEN
        inclinenump$ = ""
        IF inclinenumber(inclevel) THEN
            thisincname$ = getfilepath$(incname$(inclevel))
            thisincname$ = MID$(incname$(inclevel), LEN(thisincname$) + 1)
            inclinenump$ = "(" + thisincname$ + "," + STR$(inclinenumber(inclevel)) + ") "
        END IF

        PRINT #12, "qbs_set(__STRING_VWATCH_SUBNAME,qbs_new_txt_len(" + CHR$(34) + inclinenump$ + subfuncoriginalname$ + CHR$(34) + "," + str2$(LEN(inclinenump$ + subfuncoriginalname$)) + "));"
        PRINT #12, "qbs_cleanup(qbs_tmp_base,0);"
        PRINT #12, "qbs_set(__STRING_VWATCH_INTERNALSUBNAME,qbs_new_txt_len(" + CHR$(34) + subfunc + CHR$(34) + "," + str2$(LEN(subfunc)) + "));"
        PRINT #12, "qbs_cleanup(qbs_tmp_base,0);"
        PRINT #12, "*__LONG_VWATCH_LINENUMBER=-2; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);"
    END IF
END IF

PRINT #12, "if (new_error) goto exit_subfunc;"

'statementn = statementn + 1
'if nochecks=0 then PRINT #12, "S_" + str2$(statementn) + ":;"

dimstatic = staticsf

declibjmp4:

IF declaringlibrary THEN

    IF customtypelibrary THEN

        callname$ = removecast$(RTRIM$(id2.callname))

        PRINT #17, "CUSTOMCALL_" + callname$ + " *" + callname$ + "=NULL;"

        IF subfuncn THEN
            f = FREEFILE
            OPEN tmpdir$ + "maindata.txt" FOR APPEND AS #f
        ELSE
            f = 13
        END IF


        PRINT #f, callname$ + "=(CUSTOMCALL_" + callname$ + "*)&" + aliasname$ + ";"

        IF subfuncn THEN CLOSE #f

        'if no header exists to make the external function available, the function definition must be found
        IF sfheader = 0 AND sfdeclare <> 0 THEN
            ResolveStaticFunctions = ResolveStaticFunctions + 1
            'expand array if necessary
            IF ResolveStaticFunctions > UBOUND(ResolveStaticFunction_Name) THEN
                REDIM _PRESERVE ResolveStaticFunction_Name(1 TO ResolveStaticFunctions + 100) AS STRING
                REDIM _PRESERVE ResolveStaticFunction_File(1 TO ResolveStaticFunctions + 100) AS STRING
                REDIM _PRESERVE ResolveStaticFunction_Method(1 TO ResolveStaticFunctions + 100) AS LONG
            END IF
            ResolveStaticFunction_File(ResolveStaticFunctions) = libname$
            ResolveStaticFunction_Name(ResolveStaticFunctions) = aliasname$
            ResolveStaticFunction_Method(ResolveStaticFunctions) = 1
        END IF 'sfheader=0

    END IF

    IF dynamiclibrary THEN
        IF sfdeclare THEN

            PRINT #17, "DLLCALL_" + removecast$(RTRIM$(id2.callname)) + " " + removecast$(RTRIM$(id2.callname)) + "=NULL;"

            IF subfuncn THEN
                f = FREEFILE
                OPEN tmpdir$ + "maindata.txt" FOR APPEND AS #f
            ELSE
                f = 13
            END IF

            PRINT #f, "if (!" + removecast$(RTRIM$(id2.callname)) + "){"
            IF os$ = "WIN" THEN
                PRINT #f, removecast$(RTRIM$(id2.callname)) + "=(DLLCALL_" + removecast$(RTRIM$(id2.callname)) + ")GetProcAddress(DLL_" + DLLname$ + "," + CHR$(34) + aliasname$ + CHR$(34) + ");"
                PRINT #f, "if (!" + removecast$(RTRIM$(id2.callname)) + ") error(260);"
            END IF
            IF os$ = "LNX" THEN
                PRINT #f, removecast$(RTRIM$(id2.callname)) + "=(DLLCALL_" + removecast$(RTRIM$(id2.callname)) + ")dlsym(DLL_" + DLLname$ + "," + CHR$(34) + aliasname$ + CHR$(34) + ");"
                PRINT #f, "if (dlerror()) error(260);"
            END IF
            PRINT #f, "}"

            IF subfuncn THEN CLOSE #f

        END IF 'sfdeclare
    END IF 'dynamic

    IF sfdeclare = 1 AND customtypelibrary = 0 AND dynamiclibrary = 0 AND indirectlibrary = 0 THEN
        ResolveStaticFunctions = ResolveStaticFunctions + 1
        'expand array if necessary
        IF ResolveStaticFunctions > UBOUND(ResolveStaticFunction_Name) THEN
            REDIM _PRESERVE ResolveStaticFunction_Name(1 TO ResolveStaticFunctions + 100) AS STRING
            REDIM _PRESERVE ResolveStaticFunction_File(1 TO ResolveStaticFunctions + 100) AS STRING
            REDIM _PRESERVE ResolveStaticFunction_Method(1 TO ResolveStaticFunctions + 100) AS LONG
        END IF
        ResolveStaticFunction_File(ResolveStaticFunctions) = libname$
        ResolveStaticFunction_Name(ResolveStaticFunctions) = aliasname$
        ResolveStaticFunction_Method(ResolveStaticFunctions) = 2
    END IF

    IF sfdeclare = 0 AND indirectlibrary = 0 THEN
        CLOSE #17
        OPEN tmpdir$ + "regsf.txt" FOR APPEND AS #17
    END IF

END IF 'declaring library

GOTO finishednonexec
END IF
END IF

'END SUB/FUNCTION
IF n = 2 THEN
    IF firstelement$ = "END" THEN
        sf = 0
        IF secondelement$ = "FUNCTION" THEN sf = 1
        IF secondelement$ = "SUB" THEN sf = 2
        IF sf THEN

            IF LEN(subfunc) = 0 THEN a$ = "END " + secondelement$ + " without " + secondelement$: GOTO errmes

            'check for open controls (copy #3)
            IF controllevel <> 0 AND controltype(controllevel) <> 6 AND controltype(controllevel) <> 32 THEN 'It's OK for subs to be inside $IF blocks
            a$ = "Unidentified open control block"
            SELECT CASE controltype(controllevel)
            CASE 1: a$ = "IF without END IF"
            CASE 2: a$ = "FOR without NEXT"
            CASE 3, 4: a$ = "DO without LOOP"
            CASE 5: a$ = "WHILE without WEND"
            CASE 10 TO 19: a$ = "SELECT CASE without END SELECT"
            END SELECT
            linenumber = controlref(controllevel)
            GOTO errmes
        END IF

        IF controltype(controllevel) = 32 AND ideindentsubs THEN
            controltype(controllevel) = 0
            controllevel = controllevel - 1
        END IF

        IF LEFT$(subfunc, 4) = "SUB_" THEN secondelement$ = SCase$("Sub") ELSE secondelement$ = SCase$("Function")
        l$ = SCase$("End") + sp + secondelement$
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$

        IF vWatchOn = 1 THEN
            vWatchVariable "", 1
        END IF

        staticarraylist = "": staticarraylistn = 0 'remove previously listed arrays
        dimstatic = 0
        PRINT #12, "exit_subfunc:;"
        IF vWatchOn = 1 THEN
            IF NoChecks = 0 AND inclinenumber(inclevel) = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
                vWatchAddLabel 0, -1
            END IF
            PRINT #12, "*__LONG_VWATCH_SUBLEVEL=*__LONG_VWATCH_SUBLEVEL- 1 ;"

            IF inclinenumber(inclevel) = 0 AND firstLineNumberLabelvWatch > 0 THEN
                PRINT #12, "goto VWATCH_SKIPSETNEXTLINE;"
                PRINT #12, "VWATCH_SETNEXTLINE:;"
                PRINT #12, "switch (*__LONG_VWATCH_GOTO) {"
                FOR i = firstLineNumberLabelvWatch TO lastLineNumberLabelvWatch
                    WHILE i > LEN(vWatchUsedLabels)
                        vWatchUsedLabels = vWatchUsedLabels + SPACE$(1000)
                        vWatchUsedSkipLabels = vWatchUsedSkipLabels + SPACE$(1000)
                    WEND
                    IF ASC(vWatchUsedLabels, i) = 1 THEN
                        PRINT #12, "    case " + str2$(i) + ":"
                        PRINT #12, "        goto VWATCH_LABEL_" + str2$(i) + ";"
                        PRINT #12, "        break;"
                    END IF
                NEXT
                PRINT #12, "    default:"
                PRINT #12, "        *__LONG_VWATCH_GOTO=*__LONG_VWATCH_LINENUMBER;"
                PRINT #12, "        goto VWATCH_SETNEXTLINE;"
                PRINT #12, "}"

                PRINT #12, "VWATCH_SKIPLINE:;"
                PRINT #12, "switch (*__LONG_VWATCH_GOTO) {"
                FOR i = firstLineNumberLabelvWatch TO lastLineNumberLabelvWatch
                    IF ASC(vWatchUsedSkipLabels, i) = 1 THEN
                        PRINT #12, "    case -" + str2$(i) + ":"
                        PRINT #12, "        goto VWATCH_SKIPLABEL_" + str2$(i) + ";"
                        PRINT #12, "        break;"
                    END IF
                NEXT
                PRINT #12, "}"

                PRINT #12, "VWATCH_SKIPSETNEXTLINE:;"
            END IF
            firstLineNumberLabelvWatch = 0
        END IF

        'release _MEM lock for this scope
        PRINT #12, "free_mem_lock(sf_mem_lock);"

        PRINT #12, "#include " + CHR$(34) + "free" + str2$(subfuncn) + ".txt" + CHR$(34)
        PRINT #12, "if ((tmp_mem_static_pointer>=mem_static)&&(tmp_mem_static_pointer<=mem_static_limit)) mem_static_pointer=tmp_mem_static_pointer; else mem_static_pointer=mem_static;"
        PRINT #12, "cmem_sp=tmp_cmem_sp;"
        IF subfuncret$ <> "" THEN PRINT #12, subfuncret$

        PRINT #12, "}" 'skeleton sub
        'ret???.txt
        PRINT #15, "}" 'end case
        PRINT #15, "}"
        PRINT #15, "error(3);" 'no valid return possible
        subfunc = ""
        closedsubfunc = -1

        'unshare temp. shared variables
        FOR i = 1 TO idn
            IF ids(i).share AND 2 THEN ids(i).share = ids(i).share - 2
        NEXT

        FOR i = 1 TO revertmaymusthaven
            x = revertmaymusthave(i)
            SWAP ids(x).musthave, ids(x).mayhave
        NEXT
        revertmaymusthaven = 0

        'undeclare constants in sub/function's scope
        'constlast = constlastshared
        GOTO finishednonexec

    END IF
END IF
END IF



IF n >= 1 AND firstelement$ = "CONST" THEN
    l$ = SCase$("Const")
    'DEF... do not change type, the expression is stored in a suitable type
    'based on its value if type isn't forced/specified
    IF n < 3 THEN a$ = "Expected CONST name = value/expression": GOTO errmes
    i = 2

    constdefpending:
    pending = 0

    n$ = getelement$(ca$, i): i = i + 1
    l$ = l$ + sp + n$ + sp + "="
    typeoverride = 0
    s$ = removesymbol$(n$)
    IF Error_Happened THEN GOTO errmes
    IF s$ <> "" THEN
        typeoverride = typname2typ(s$)
        IF Error_Happened THEN GOTO errmes
        IF typeoverride AND ISFIXEDLENGTH THEN a$ = "Invalid constant type": GOTO errmes
        IF typeoverride = 0 THEN a$ = "Invalid constant type": GOTO errmes
    END IF

    IF getelement$(a$, i) <> "=" THEN a$ = "Expected =": GOTO errmes
    i = i + 1

    'get expression
    e$ = ""
    B = 0
    FOR i2 = i TO n
        e2$ = getelement$(ca$, i2)
        IF e2$ = "(" THEN B = B + 1
        IF e2$ = ")" THEN B = B - 1
        IF e2$ = "," AND B = 0 THEN
            pending = 1
            i = i2 + 1
            IF i > n - 2 THEN a$ = "Expected CONST ... , name = value/expression": GOTO errmes
            EXIT FOR
        END IF
        IF LEN(e$) = 0 THEN e$ = e2$ ELSE e$ = e$ + sp + e2$
    NEXT

    e$ = fixoperationorder(e$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + sp + tlayout$

    'Note: Actual CONST definition handled in prepass

    'Set CONST as defined
    hashname$ = n$
    hashchkflags = HASHFLAG_CONSTANT
    hashres = HashFind(hashname$, hashchkflags, hashresflags, hashresref)
    DO WHILE hashres
        IF constsubfunc(hashresref) = subfuncn THEN constdefined(hashresref) = 1: EXIT DO
        IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
    LOOP

    IF pending THEN l$ = l$ + sp2 + ",": GOTO constdefpending

    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$

    GOTO finishednonexec
END IF

predefine:
IF n >= 2 THEN
    asreq = 0
    IF firstelement$ = "DEFINT" THEN l$ = SCase$("DefInt"): a$ = a$ + sp + "AS" + sp + "INTEGER": n = n + 2: GOTO definetype
    IF firstelement$ = "DEFLNG" THEN l$ = SCase$("DefLng"): a$ = a$ + sp + "AS" + sp + "LONG": n = n + 2: GOTO definetype
    IF firstelement$ = "DEFSNG" THEN l$ = SCase$("DefSng"): a$ = a$ + sp + "AS" + sp + "SINGLE": n = n + 2: GOTO definetype
    IF firstelement$ = "DEFDBL" THEN l$ = SCase$("DefDbl"): a$ = a$ + sp + "AS" + sp + "DOUBLE": n = n + 2: GOTO definetype
    IF firstelement$ = "DEFSTR" THEN l$ = SCase$("DefStr"): a$ = a$ + sp + "AS" + sp + "STRING": n = n + 2: GOTO definetype
    IF firstelement$ = "_DEFINE" OR (firstelement$ = "DEFINE" AND qbnexprefix_set = 1) THEN
        asreq = 1
        IF firstelement$ = "_DEFINE" THEN l$ = SCase$("_Define") ELSE l$ = SCase$("Define")
        definetype:
        'get type from rhs
        typ$ = ""
        typ2$ = ""
        t$ = ""
        FOR i = n TO 2 STEP -1
            t$ = getelement$(a$, i)
            IF t$ = "AS" THEN EXIT FOR
            typ$ = t$ + " " + typ$
            typ2$ = t$ + sp + typ2$
        NEXT
        typ$ = RTRIM$(typ$)
        IF t$ <> "AS" THEN a$ = qbnexprefix$ + "DEFINE: Expected ... AS ...": GOTO errmes
        IF i = n OR i = 2 THEN a$ = qbnexprefix$ + "DEFINE: Expected ... AS ...": GOTO errmes


        n = i - 1
        'the data is from element 2 to element n
        i = 2 - 1
        definenext:
        'expects an alphabet letter or underscore
        i = i + 1: e$ = getelement$(a$, i): E = ASC(UCASE$(e$))
        IF LEN(e$) > 1 THEN a$ = qbnexprefix$ + "DEFINE: Expected an alphabet letter or the underscore character (_)": GOTO errmes
        IF E <> 95 AND (E > 90 OR E < 65) THEN a$ = qbnexprefix$ + "DEFINE: Expected an alphabet letter or the underscore character (_)": GOTO errmes
        IF E = 95 THEN E = 27 ELSE E = E - 64
        defineaz(E) = typ$
        defineextaz(E) = type2symbol(typ$)
        IF Error_Happened THEN GOTO errmes
        firste = E
        l$ = l$ + sp + e$

        IF i = n THEN
            IF predefining = 1 THEN GOTO predefined
            IF asreq THEN l$ = l$ + sp + SCase$("As") + sp + typ2$
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishednonexec
        END IF

        'expects "-" or ","
        i = i + 1: e$ = getelement$(a$, i)
        IF e$ <> "-" AND e$ <> "," THEN a$ = qbnexprefix$ + "DEFINE: Expected - or ,": GOTO errmes
        IF e$ = "-" THEN
            l$ = l$ + sp2 + "-"
            IF i = n THEN a$ = qbnexprefix$ + "DEFINE: Syntax incomplete": GOTO errmes
            'expects an alphabet letter or underscore
            i = i + 1: e$ = getelement$(a$, i): E = ASC(UCASE$(e$))
            IF LEN(e$) > 1 THEN a$ = qbnexprefix$ + "DEFINE: Expected an alphabet letter or the underscore character (_)": GOTO errmes
            IF E <> 95 AND (E > 90 OR E < 65) THEN a$ = qbnexprefix$ + "DEFINE: Expected an alphabet letter or the underscore character (_)": GOTO errmes
            IF E = 95 THEN E = 27 ELSE E = E - 64
            IF firste > E THEN SWAP E, firste
            FOR e2 = firste TO E
                defineaz(e2) = typ$
                defineextaz(e2) = type2symbol(typ$)
                IF Error_Happened THEN GOTO errmes
            NEXT
            l$ = l$ + sp2 + e$
            IF i = n THEN
                IF predefining = 1 THEN GOTO predefined
                IF asreq THEN l$ = l$ + sp + SCase$("As") + sp + typ2$
                layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
                GOTO finishednonexec
            END IF
            'expects ","
            i = i + 1: e$ = getelement$(a$, i)
            IF e$ <> "," THEN a$ = qbnexprefix$ + "DEFINE: Expected ,": GOTO errmes
        END IF
        l$ = l$ + sp2 + ","
        GOTO definenext
    END IF '_DEFINE
END IF '2
IF predefining = 1 THEN GOTO predefined

IF closedmain <> 0 AND subfunc = "" THEN a$ = "Statement cannot be placed between SUB/FUNCTIONs": GOTO errmes

'executable section:

statementn = statementn + 1


IF n >= 1 THEN
    IF firstelement$ = "NEXT" THEN

        l$ = SCase$("Next")
        IF n = 1 THEN GOTO simplenext
        v$ = ""
        FOR i = 2 TO n
            a2$ = getelement(ca$, i)

            IF a2$ = "," THEN

                lastnextele:
                e$ = fixoperationorder(v$)
                IF Error_Happened THEN GOTO errmes
                IF LEN(l$) = 4 THEN l$ = l$ + sp + tlayout$ ELSE l$ = l$ + sp2 + "," + sp + tlayout$
                e$ = evaluate(e$, typ)
                IF Error_Happened THEN GOTO errmes
                IF (typ AND ISREFERENCE) THEN
                    getid VAL(e$)
                    IF Error_Happened THEN GOTO errmes
                    IF (id.t AND ISPOINTER) THEN
                        IF (id.t AND ISSTRING) = 0 THEN
                            IF (id.t AND ISOFFSETINBITS) = 0 THEN
                                IF (id.t AND ISARRAY) = 0 THEN
                                    GOTO fornextfoundvar2
                                END IF
                            END IF
                        END IF
                    END IF
                END IF
                a$ = "Unsupported variable after NEXT": GOTO errmes
                fornextfoundvar2:
                simplenext:
                IF controltype(controllevel) <> 2 THEN a$ = "NEXT without FOR": GOTO errmes
                IF n <> 1 AND controlvalue(controllevel) <> currentid THEN a$ = "Incorrect variable after NEXT": GOTO errmes
                PRINT #12, "fornext_continue_" + str2$(controlid(controllevel)) + ":;"
                IF vWatchOn = 1 AND inclinenumber(inclevel) = 0 AND NoChecks = 0 THEN
                    vWatchAddLabel linenumber, 0
                    PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
                END IF
                PRINT #12, "}"
                PRINT #12, "fornext_exit_" + str2$(controlid(controllevel)) + ":;"
                controllevel = controllevel - 1
                IF n = 1 THEN EXIT FOR
                v$ = ""

            ELSE

                IF LEN(v$) THEN v$ = v$ + sp + a2$ ELSE v$ = a2$
                IF i = n THEN GOTO lastnextele

            END IF

        NEXT

        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishednonexec '***no error causing code, event checking done by FOR***
    END IF
END IF



IF n >= 1 THEN
    IF firstelement$ = "WHILE" THEN
        IF NoChecks = 0 THEN PRINT #12, "S_" + str2$(statementn) + ":;": dynscope = 1

        'prevents code from being placed before 'CASE condition' in a SELECT CASE block
        IF SelectCaseCounter > 0 AND SelectCaseHasCaseBlock(SelectCaseCounter) = 0 THEN
            a$ = "Expected CASE expression": GOTO errmes
        END IF

        controllevel = controllevel + 1
        controlref(controllevel) = linenumber
        controltype(controllevel) = 5
        controlid(controllevel) = uniquenumber
        IF n >= 2 THEN
            e$ = fixoperationorder(getelements$(ca$, 2, n))
            IF Error_Happened THEN GOTO errmes
            l$ = SCase$("While") + sp + tlayout$
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            e$ = evaluate(e$, typ)
            IF Error_Happened THEN GOTO errmes
            IF (typ AND ISREFERENCE) THEN e$ = refer$(e$, typ, 0)
            IF Error_Happened THEN GOTO errmes
            IF stringprocessinghappened THEN e$ = cleanupstringprocessingcall$ + e$ + ")"
            IF (typ AND ISSTRING) THEN a$ = "WHILE ERROR! Cannot accept a STRING type.": GOTO errmes
            IF NoChecks = 0 AND vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            END IF
            PRINT #12, "while((" + e$ + ")||new_error){"
        ELSE
            a$ = "WHILE ERROR! Expected expression after WHILE.": GOTO errmes
        END IF

        GOTO finishedline
    END IF
END IF

IF n = 1 THEN
    IF firstelement$ = "WEND" THEN


        IF controltype(controllevel) <> 5 THEN a$ = "WEND without WHILE": GOTO errmes
        PRINT #12, "ww_continue_" + str2$(controlid(controllevel)) + ":;"
        PRINT #12, "}"
        PRINT #12, "ww_exit_" + str2$(controlid(controllevel)) + ":;"
        controllevel = controllevel - 1
        l$ = SCase$("Wend")
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishednonexec '***no error causing code, event checking done by WHILE***
    END IF
END IF





IF n >= 1 THEN
    IF firstelement$ = "DO" THEN
        IF NoChecks = 0 THEN PRINT #12, "S_" + str2$(statementn) + ":;": dynscope = 1

        'prevents code from being placed before 'CASE condition' in a SELECT CASE block
        IF SelectCaseCounter > 0 AND SelectCaseHasCaseBlock(SelectCaseCounter) = 0 THEN
            a$ = "Expected CASE expression": GOTO errmes
        END IF

        controllevel = controllevel + 1
        controlref(controllevel) = linenumber
        l$ = SCase$("Do")
        IF n >= 2 THEN
            whileuntil = 0
            IF secondelement$ = "WHILE" THEN whileuntil = 1: l$ = l$ + sp + SCase$("While")
            IF secondelement$ = "UNTIL" THEN whileuntil = 2: l$ = l$ + sp + SCase$("Until")
            IF whileuntil = 0 THEN a$ = "DO ERROR! Expected WHILE or UNTIL after DO.": GOTO errmes
            IF whileuntil > 0 AND n = 2 THEN a$ = "Condition expected after WHILE/UNTIL": GOTO errmes
            e$ = fixoperationorder(getelements$(ca$, 3, n))
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp + tlayout$
            e$ = evaluate(e$, typ)
            IF Error_Happened THEN GOTO errmes
            IF (typ AND ISREFERENCE) THEN e$ = refer$(e$, typ, 0)
            IF Error_Happened THEN GOTO errmes
            IF stringprocessinghappened THEN e$ = cleanupstringprocessingcall$ + e$ + ")"
            IF (typ AND ISSTRING) THEN a$ = "DO ERROR! Cannot accept a STRING type.": GOTO errmes
            IF whileuntil = 1 THEN PRINT #12, "while((" + e$ + ")||new_error){" ELSE PRINT #12, "while((!(" + e$ + "))||new_error){"
            IF NoChecks = 0 AND vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            END IF
            controltype(controllevel) = 4
        ELSE
            controltype(controllevel) = 3
            IF vWatchOn = 1 AND inclinenumber(inclevel) = 0 AND NoChecks = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "do{*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            ELSE
                PRINT #12, "do{"
            END IF
        END IF
        controlid(controllevel) = uniquenumber
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline
    END IF
END IF

IF n >= 1 THEN
    IF firstelement$ = "LOOP" THEN
        l$ = SCase$("Loop")
        IF controltype(controllevel) <> 3 AND controltype(controllevel) <> 4 THEN a$ = "PROGRAM FLOW ERROR!": GOTO errmes
        IF n >= 2 THEN
            IF NoChecks = 0 THEN PRINT #12, "S_" + str2$(statementn) + ":;": dynscope = 1
            IF controltype(controllevel) = 4 THEN a$ = "PROGRAM FLOW ERROR!": GOTO errmes
            whileuntil = 0
            IF secondelement$ = "WHILE" THEN whileuntil = 1: l$ = l$ + sp + SCase$("While")
            IF secondelement$ = "UNTIL" THEN whileuntil = 2: l$ = l$ + sp + SCase$("Until")
            IF whileuntil = 0 THEN a$ = "LOOP ERROR! Expected WHILE or UNTIL after LOOP.": GOTO errmes
            IF whileuntil > 0 AND n = 2 THEN a$ = "Condition expected after WHILE/UNTIL": GOTO errmes
            e$ = fixoperationorder(getelements$(ca$, 3, n))
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp + tlayout$
            e$ = evaluate(e$, typ)
            IF Error_Happened THEN GOTO errmes
            IF (typ AND ISREFERENCE) THEN e$ = refer$(e$, typ, 0)
            IF Error_Happened THEN GOTO errmes
            IF stringprocessinghappened THEN e$ = cleanupstringprocessingcall$ + e$ + ")"
            IF (typ AND ISSTRING) THEN a$ = "LOOP ERROR! Cannot accept a STRING type.": GOTO errmes
            PRINT #12, "dl_continue_" + str2$(controlid(controllevel)) + ":;"
            IF NoChecks = 0 AND vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            END IF
            IF whileuntil = 1 THEN PRINT #12, "}while((" + e$ + ")&&(!new_error));" ELSE PRINT #12, "}while((!(" + e$ + "))&&(!new_error));"
        ELSE
            PRINT #12, "dl_continue_" + str2$(controlid(controllevel)) + ":;"

            IF NoChecks = 0 AND vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            END IF

            IF controltype(controllevel) = 4 THEN
                PRINT #12, "}"
            ELSE
                PRINT #12, "}while(1);" 'infinite loop!
            END IF
        END IF
        PRINT #12, "dl_exit_" + str2$(controlid(controllevel)) + ":;"
        controllevel = controllevel - 1
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        IF n = 1 THEN GOTO finishednonexec '***no error causing code, event checking done by DO***
        GOTO finishedline
    END IF
END IF









IF n >= 1 THEN
    IF firstelement$ = "FOR" THEN
        IF NoChecks = 0 THEN PRINT #12, "S_" + str2$(statementn) + ":;": dynscope = 1

        l$ = SCase$("For")

        'prevents code from being placed before 'CASE condition' in a SELECT CASE block
        IF SelectCaseCounter > 0 AND SelectCaseHasCaseBlock(SelectCaseCounter) = 0 THEN
            a$ = "Expected CASE expression": GOTO errmes
        END IF

        controllevel = controllevel + 1
        controlref(controllevel) = linenumber
        controltype(controllevel) = 2
        controlid(controllevel) = uniquenumber

        v$ = ""
        startvalue$ = ""
        p3$ = "1": stepused = 0
        p2$ = ""
        mode = 0
        E = 0
        FOR i = 2 TO n
            e$ = getelement$(a$, i)
            IF e$ = "=" THEN
                IF mode <> 0 THEN E = 1: EXIT FOR
                mode = 1
                v$ = getelements$(ca$, 2, i - 1)
                equpos = i
            END IF
            IF e$ = "TO" THEN
                IF mode <> 1 THEN E = 1: EXIT FOR
                mode = 2
                startvalue$ = getelements$(ca$, equpos + 1, i - 1)
                topos = i
            END IF
            IF e$ = "STEP" THEN
                IF mode <> 2 THEN E = 1: EXIT FOR
                mode = 3
                stepused = 1
                p2$ = getelements$(ca$, topos + 1, i - 1)
                p3$ = getelements$(ca$, i + 1, n)
                EXIT FOR
            END IF
        NEXT
        IF mode < 2 THEN E = 1
        IF p2$ = "" THEN p2$ = getelements$(ca$, topos + 1, n)
        IF LEN(v$) = 0 OR LEN(startvalue$) = 0 OR LEN(p2$) = 0 THEN E = 1
        IF E <> 0 AND mode < 3 THEN a$ = "Expected FOR name = start TO end": GOTO errmes
        IF E THEN a$ = "Expected FOR name = start TO end STEP increment": GOTO errmes

        e$ = fixoperationorder(v$)
        IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp + tlayout$
        e$ = evaluate(e$, typ)
        IF Error_Happened THEN GOTO errmes
        IF (typ AND ISREFERENCE) THEN
            getid VAL(e$)
            IF Error_Happened THEN GOTO errmes
            IF (id.t AND ISPOINTER) THEN
                IF (id.t AND ISSTRING) = 0 THEN
                    IF (id.t AND ISOFFSETINBITS) = 0 THEN
                        IF (id.t AND ISARRAY) = 0 THEN
                            GOTO fornextfoundvar
                        END IF
                    END IF
                END IF
            END IF
        END IF
        a$ = "Unsupported variable used in FOR statement": GOTO errmes
        fornextfoundvar:
        controlvalue(controllevel) = currentid
        v$ = e$

        'find C++ datatype to match variable
        'markup to cater for greater range/accuracy
        ctype$ = ""
        ctyp = typ - ISPOINTER
        bits = typ AND 511
        IF (typ AND ISFLOAT) THEN
            IF bits = 32 THEN ctype$ = "double": ctyp = 64& + ISFLOAT
            IF bits = 64 THEN ctype$ = "long double": ctyp = 256& + ISFLOAT
            IF bits = 256 THEN ctype$ = "long double": ctyp = 256& + ISFLOAT
        ELSE
            IF bits = 8 THEN ctype$ = "int16": ctyp = 16&
            IF bits = 16 THEN ctype$ = "int32": ctyp = 32&
            IF bits = 32 THEN ctype$ = "int64": ctyp = 64&
            IF bits = 64 THEN ctype$ = "int64": ctyp = 64&
        END IF
        IF ctype$ = "" THEN a$ = "Unsupported variable used in FOR statement": GOTO errmes
        u$ = str2(uniquenumber)

        IF subfunc = "" THEN
            PRINT #13, "static " + ctype$ + " fornext_value" + u$ + ";"
            PRINT #13, "static " + ctype$ + " fornext_finalvalue" + u$ + ";"
            PRINT #13, "static " + ctype$ + " fornext_step" + u$ + ";"
            PRINT #13, "static uint8 fornext_step_negative" + u$ + ";"
        ELSE
            PRINT #13, ctype$ + " fornext_value" + u$ + ";"
            PRINT #13, ctype$ + " fornext_finalvalue" + u$ + ";"
            PRINT #13, ctype$ + " fornext_step" + u$ + ";"
            PRINT #13, "uint8 fornext_step_negative" + u$ + ";"
        END IF

        'calculate start
        e$ = fixoperationorder$(startvalue$)
        IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp + "=" + sp + tlayout$
        e$ = evaluatetotyp$(e$, ctyp)
        IF Error_Happened THEN GOTO errmes
        PRINT #12, "fornext_value" + u$ + "=" + e$ + ";"

        'final
        e$ = fixoperationorder$(p2$)
        IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp + SCase$("To") + sp + tlayout$
        e$ = evaluatetotyp(e$, ctyp)
        IF Error_Happened THEN GOTO errmes
        PRINT #12, "fornext_finalvalue" + u$ + "=" + e$ + ";"

        'step
        e$ = fixoperationorder$(p3$)
        IF Error_Happened THEN GOTO errmes
        IF stepused = 1 THEN l$ = l$ + sp + SCase$("Step") + sp + tlayout$
        e$ = evaluatetotyp(e$, ctyp)
        IF Error_Happened THEN GOTO errmes

        IF NoChecks = 0 AND vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
            vWatchAddLabel linenumber, 0
            PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
        END IF

        PRINT #12, "fornext_step" + u$ + "=" + e$ + ";"
        PRINT #12, "if (fornext_step" + u$ + "<0) fornext_step_negative" + u$ + "=1; else fornext_step_negative" + u$ + "=0;"

        PRINT #12, "if (new_error) goto fornext_error" + u$ + ";"
        PRINT #12, "goto fornext_entrylabel" + u$ + ";"
        PRINT #12, "while(1){"
        typbak = typ
        PRINT #12, "fornext_value" + u$ + "=fornext_step" + u$ + "+(" + refer$(v$, typ, 0) + ");"
        IF Error_Happened THEN GOTO errmes
        typ = typbak
        PRINT #12, "fornext_entrylabel" + u$ + ":"
        setrefer v$, typ, "fornext_value" + u$, 1
        IF Error_Happened THEN GOTO errmes
        PRINT #12, "if (fornext_step_negative" + u$ + "){"
        PRINT #12, "if (fornext_value" + u$ + "<fornext_finalvalue" + u$ + ") break;"
        PRINT #12, "}else{"
        PRINT #12, "if (fornext_value" + u$ + ">fornext_finalvalue" + u$ + ") break;"
        PRINT #12, "}"
        PRINT #12, "fornext_error" + u$ + ":;"

        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$

        GOTO finishedline
    END IF
END IF


IF n = 1 THEN
    IF firstelement$ = "ELSE" THEN

        'Routine to add error checking for ELSE so we'll no longer be able to do things like the following:
        'IF x = 1 THEN
        '    SELECT CASE s
        '        CASE 1
        '    END SELECT ELSE y = 2
        'END IF
        'Notice the ELSE with the SELECT CASE?  Before this patch, commands like those were considered valid QBNex code.
        temp$ = UCASE$(LTRIM$(RTRIM$(wholeline)))
        DO WHILE INSTR(temp$, CHR$(9))
            ASC(temp$, INSTR(temp$, CHR$(9))) = 32
        LOOP
        goodelse = 0 'a check to see if it's a good else
        IF LEFT$(temp$, 2) = "IF" THEN goodelse = -1: GOTO skipelsecheck 'If we have an IF, the else is probably good
        IF LEFT$(temp$, 4) = "ELSE" THEN goodelse = -1: GOTO skipelsecheck 'If it's an else by itself,then we'll call it good too at this point and let the rest of the syntax checking check for us
        DO
            spacelocation = INSTR(temp$, " ")
            IF spacelocation THEN temp$ = LEFT$(temp$, spacelocation - 1) + MID$(temp$, spacelocation + 1)
        LOOP UNTIL spacelocation = 0
        IF INSTR(temp$, ":ELSE") OR INSTR(temp$, ":IF") THEN goodelse = -1: GOTO skipelsecheck 'I personally don't like the idea of a :ELSE statement, but this checks for that and validates it as well.  YUCK!  (I suppose this might be useful if there's a label where the ELSE is, like thisline: ELSE
        count = 0
        DO
            count = count + 1
            SELECT CASE MID$(temp$, count, 1)
            CASE IS = "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", ":"
            CASE ELSE: EXIT DO
            END SELECT
        LOOP UNTIL count >= LEN(temp$)
        IF MID$(temp$, count, 4) = "ELSE" OR MID$(temp$, count, 2) = "IF" THEN goodelse = -1 'We only had numbers before our else
        IF NOT goodelse THEN a$ = "Invalid Syntax for ELSE": GOTO errmes
        skipelsecheck:
        'End of ELSE Error checking
        FOR i = controllevel TO 1 STEP -1
            t = controltype(i)
            IF t = 1 THEN
                IF controlstate(controllevel) = 2 THEN a$ = "IF-THEN already contains an ELSE statement": GOTO errmes
                PRINT #12, "}else{"
                controlstate(controllevel) = 2
                IF lineelseused = 0 THEN lhscontrollevel = lhscontrollevel - 1
                l$ = SCase$("Else")
                layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
                GOTO finishednonexec '***no error causing code, event checking done by IF***
            END IF
        NEXT
        a$ = "ELSE without IF": GOTO errmes
    END IF
END IF

IF n >= 3 THEN
    IF firstelement$ = "ELSEIF" THEN
        IF NoChecks = 0 THEN
            PRINT #12, "S_" + str2$(statementn) + ":;": dynscope = 1
            IF vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            END IF
        END IF
        FOR i = controllevel TO 1 STEP -1
            t = controltype(i)
            IF t = 1 THEN
                IF controlstate(controllevel) = 2 THEN a$ = "ELSEIF invalid after ELSE": GOTO errmes
                controlstate(controllevel) = 1
                controlvalue(controllevel) = controlvalue(controllevel) + 1
                e$ = getelement$(a$, n)
                IF e$ <> "THEN" THEN a$ = "Expected ELSEIF expression THEN": GOTO errmes
                PRINT #12, "}else{"
                e$ = fixoperationorder$(getelements$(ca$, 2, n - 1))
                IF Error_Happened THEN GOTO errmes
                l$ = SCase$("ElseIf") + sp + tlayout$ + sp + SCase$("Then")
                layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
                e$ = evaluate(e$, typ)
                IF Error_Happened THEN GOTO errmes
                IF (typ AND ISREFERENCE) THEN e$ = refer$(e$, typ, 0)
                IF Error_Happened THEN GOTO errmes
                IF typ AND ISSTRING THEN
                    a$ = "Expected ELSEIF LEN(stringexpression) THEN": GOTO errmes
                END IF
                IF stringprocessinghappened THEN
                    PRINT #12, "if (" + cleanupstringprocessingcall$ + e$ + ")){"
                ELSE
                    PRINT #12, "if (" + e$ + "){"
                END IF
                lhscontrollevel = lhscontrollevel - 1
                GOTO finishedline
            END IF
        NEXT
        a$ = "ELSEIF without IF": GOTO errmes
    END IF
END IF

IF n >= 3 THEN
    IF firstelement$ = "IF" THEN
        IF NoChecks = 0 THEN
            PRINT #12, "S_" + str2$(statementn) + ":;": dynscope = 1
            IF vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            END IF
        END IF

        'prevents code from being placed before 'CASE condition' in a SELECT CASE block
        IF SelectCaseCounter > 0 AND SelectCaseHasCaseBlock(SelectCaseCounter) = 0 THEN
            a$ = "Expected CASE expression": GOTO errmes
        END IF

        e$ = getelement(a$, n)
        iftype = 0
        IF e$ = "THEN" THEN iftype = 1
        IF e$ = "GOTO" THEN iftype = 2
        IF iftype = 0 THEN a$ = "Expected IF expression THEN/GOTO": GOTO errmes

        controllevel = controllevel + 1
        controlref(controllevel) = linenumber
        controltype(controllevel) = 1
        controlvalue(controllevel) = 0 'number of extra closing } required at END IF
        controlstate(controllevel) = 0

        e$ = fixoperationorder$(getelements(ca$, 2, n - 1))
        IF Error_Happened THEN GOTO errmes
        l$ = SCase$("If") + sp + tlayout$
        e$ = evaluate(e$, typ)
        IF Error_Happened THEN GOTO errmes
        IF (typ AND ISREFERENCE) THEN e$ = refer$(e$, typ, 0)
        IF Error_Happened THEN GOTO errmes

        IF typ AND ISSTRING THEN
            a$ = "Expected IF LEN(stringexpression) THEN": GOTO errmes
        END IF

        IF stringprocessinghappened THEN
            PRINT #12, "if ((" + cleanupstringprocessingcall$ + e$ + "))||new_error){"
        ELSE
            PRINT #12, "if ((" + e$ + ")||new_error){"
        END IF

        IF iftype = 1 THEN l$ = l$ + sp + SCase$("Then") 'note: 'GOTO' will be added when iftype=2
        layoutdone = 1: IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$

        IF iftype = 2 THEN 'IF ... GOTO
        GOTO finishedline
    END IF

    THENGOTO = 1 'possible: IF a=1 THEN 10
    GOTO finishedline2
END IF
END IF

'ENDIF
IF n = 1 AND getelement(a$, 1) = "ENDIF" THEN
    IF controltype(controllevel) <> 1 THEN a$ = "END IF without IF": GOTO errmes
    layoutdone = 1
    IF impliedendif = 0 THEN
        l$ = SCase$("End If")
        IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$
    END IF

    PRINT #12, "}"
    FOR i = 1 TO controlvalue(controllevel)
        PRINT #12, "}"
    NEXT
    controllevel = controllevel - 1
    GOTO finishednonexec '***no error causing code, event checking done by IF***
END IF


'END IF
IF n = 2 THEN
    IF getelement(a$, 1) = "END" AND getelement(a$, 2) = "IF" THEN


        IF controltype(controllevel) <> 1 THEN a$ = "END IF without IF": GOTO errmes
        layoutdone = 1
        IF impliedendif = 0 THEN
            l$ = SCase$("End" + sp + "If")
            IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$
        END IF

        IF NoChecks = 0 AND vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
            vWatchAddLabel linenumber, 0
            PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
        END IF

        PRINT #12, "}"
        FOR i = 1 TO controlvalue(controllevel)
            PRINT #12, "}"
        NEXT
        controllevel = controllevel - 1
        GOTO finishednonexec '***no error causing code, event checking done by IF***
    END IF
END IF



'SELECT CASE
IF n >= 1 THEN
    IF firstelement$ = "SELECT" THEN
        IF NoChecks = 0 THEN
            PRINT #12, "S_" + str2$(statementn) + ":;": dynscope = 1
            IF vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            END IF
        END IF

        'prevents code from being placed before 'CASE condition' in a SELECT CASE block
        IF SelectCaseCounter > 0 AND SelectCaseHasCaseBlock(SelectCaseCounter) = 0 THEN
            a$ = "Expected CASE expression": GOTO errmes
        END IF

        SelectCaseCounter = SelectCaseCounter + 1
        IF UBOUND(EveryCaseSet) <= SelectCaseCounter THEN REDIM _PRESERVE EveryCaseSet(SelectCaseCounter)
        IF UBOUND(SelectCaseHasCaseBlock) <= SelectCaseCounter THEN REDIM _PRESERVE SelectCaseHasCaseBlock(SelectCaseCounter)
        SelectCaseHasCaseBlock(SelectCaseCounter) = 0
        IF secondelement$ = "EVERYCASE" THEN
            EveryCaseSet(SelectCaseCounter) = -1
            IF n = 2 THEN a$ = "Expected SELECT CASE expression": GOTO errmes
            e$ = fixoperationorder(getelements$(ca$, 3, n))
            IF Error_Happened THEN GOTO errmes
            l$ = SCase$("Select EveryCase ") + tlayout$
        ELSE
            EveryCaseSet(SelectCaseCounter) = 0
            IF n = 1 OR secondelement$ <> "CASE" THEN a$ = "Expected CASE or EVERYCASE": GOTO errmes
            IF n = 2 THEN a$ = "Expected SELECT CASE expression": GOTO errmes
            e$ = fixoperationorder(getelements$(ca$, 3, n))
            IF Error_Happened THEN GOTO errmes
            l$ = SCase$("Select Case ") + tlayout$
        END IF

        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        e$ = evaluate(e$, typ)
        IF Error_Happened THEN GOTO errmes
        u = uniquenumber

        controllevel = controllevel + 1
        controlvalue(controllevel) = 0 'id

        t$ = ""
        IF (typ AND ISSTRING) THEN
            t = 0
            IF (typ AND ISUDT) = 0 AND (typ AND ISARRAY) = 0 AND (typ AND ISREFERENCE) <> 0 THEN
                controlvalue(controllevel) = VAL(e$)
            ELSE
                IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                IF Error_Happened THEN GOTO errmes
                PRINT #13, "static qbs *sc_" + str2$(u) + "=qbs_new(0,0);"
                PRINT #12, "qbs_set(sc_" + str2$(u) + "," + e$ + ");"
                IF stringprocessinghappened THEN PRINT #12, cleanupstringprocessingcall$ + "0);"
            END IF

        ELSE

            IF (typ AND ISFLOAT) THEN

                IF (typ AND 511) > 64 THEN t = 3: t$ = "long double"
                IF (typ AND 511) = 32 THEN t = 4: t$ = "float"
                IF (typ AND 511) = 64 THEN t = 5: t$ = "double"
                IF (typ AND ISUDT) = 0 AND (typ AND ISARRAY) = 0 AND (typ AND ISREFERENCE) <> 0 THEN
                    controlvalue(controllevel) = VAL(e$)
                ELSE
                    IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                    IF Error_Happened THEN GOTO errmes

                    PRINT #13, "static " + t$ + " sc_" + str2$(u) + ";"
                    PRINT #12, "sc_" + str2$(u) + "=" + e$ + ";"
                    IF stringprocessinghappened THEN PRINT #12, cleanupstringprocessingcall$ + "0);"
                END IF

            ELSE

                'non-float
                t = 1: t$ = "int64"
                IF (typ AND ISUNSIGNED) THEN
                    IF (typ AND 511) <= 32 THEN t = 7: t$ = "uint32"
                    IF (typ AND 511) > 32 THEN t = 2: t$ = "uint64"
                ELSE
                    IF (typ AND 511) <= 32 THEN t = 6: t$ = "int32"
                    IF (typ AND 511) > 32 THEN t = 1: t$ = "int64"
                END IF
                IF (typ AND ISUDT) = 0 AND (typ AND ISARRAY) = 0 AND (typ AND ISREFERENCE) <> 0 THEN
                    controlvalue(controllevel) = VAL(e$)
                ELSE
                    IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                    IF Error_Happened THEN GOTO errmes
                    PRINT #13, "static " + t$ + " sc_" + str2$(u) + ";"
                    PRINT #12, "sc_" + str2$(u) + "=" + e$ + ";"
                    IF stringprocessinghappened THEN PRINT #12, cleanupstringprocessingcall$ + "0);"
                END IF

            END IF
        END IF



        controlref(controllevel) = linenumber
        controltype(controllevel) = 10 + t
        controlid(controllevel) = u
        IF EveryCaseSet(SelectCaseCounter) THEN PRINT #13, "int32 sc_" + str2$(controlid(controllevel)) + "_var;"
        IF EveryCaseSet(SelectCaseCounter) THEN PRINT #12, "sc_" + str2$(controlid(controllevel)) + "_var=0;"
        GOTO finishedline
    END IF
END IF


'END SELECT
IF n = 2 THEN
    IF firstelement$ = "END" AND secondelement$ = "SELECT" THEN
        'complete current case if necessary
        '18=CASE (awaiting END SELECT/CASE/CASE ELSE)
        '19=CASE ELSE (awaiting END SELECT)
        IF controltype(controllevel) = 18 THEN
            everycasenewcase = everycasenewcase + 1
            PRINT #12, "sc_ec_" + str2$(everycasenewcase) + "_end:;"
            controllevel = controllevel - 1
            IF EveryCaseSet(SelectCaseCounter) = 0 THEN PRINT #12, "goto sc_" + str2$(controlid(controllevel)) + "_end;"
            PRINT #12, "}"
        END IF
        IF controltype(controllevel) = 19 THEN
            controllevel = controllevel - 1
            IF EveryCaseSet(SelectCaseCounter) THEN PRINT #12, "} /* End of SELECT EVERYCASE ELSE */"
        END IF

        PRINT #12, "sc_" + str2$(controlid(controllevel)) + "_end:;"
        IF controltype(controllevel) < 10 OR controltype(controllevel) > 17 THEN a$ = "END SELECT without SELECT CASE": GOTO errmes

        IF NoChecks = 0 AND vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
            vWatchAddLabel linenumber, 0
            PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
        END IF

        IF SelectCaseCounter > 0 AND SelectCaseHasCaseBlock(SelectCaseCounter) = 0 THEN
            'warn user of empty SELECT CASE block
            IF NOT IgnoreWarnings THEN
                addWarning linenumber, inclevel, inclinenumber(inclevel), incname$(inclevel), "empty SELECT CASE block", ""
            END IF
        END IF

        controllevel = controllevel - 1
        SelectCaseCounter = SelectCaseCounter - 1
        l$ = SCase$("End" + sp + "Select")
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishednonexec '***no error causing code, event checking done by SELECT CASE***
    END IF
END IF

'prevents code from being placed before 'CASE condition' in a SELECT CASE block
IF n >= 1 AND firstelement$ <> "CASE" AND SelectCaseCounter > 0 AND SelectCaseHasCaseBlock(SelectCaseCounter) = 0 THEN
    a$ = "Expected CASE expression": GOTO errmes
END IF


'CASE
IF n >= 1 THEN
    IF firstelement$ = "CASE" THEN

        l$ = SCase$("Case")
        'complete current case if necessary
        '18=CASE (awaiting END SELECT/CASE/CASE ELSE)
        '19=CASE ELSE (awaiting END SELECT)
        IF controltype(controllevel) = 19 THEN a$ = "Expected END SELECT": GOTO errmes
        IF controltype(controllevel) = 18 THEN
            lhscontrollevel = lhscontrollevel - 1
            controllevel = controllevel - 1
            everycasenewcase = everycasenewcase + 1
            PRINT #12, "sc_ec_" + str2$(everycasenewcase) + "_end:;"
            IF EveryCaseSet(SelectCaseCounter) = 0 THEN
                PRINT #12, "goto sc_" + str2$(controlid(controllevel)) + "_end;"
            ELSE
                PRINT #12, "sc_" + str2$(controlid(controllevel)) + "_var=-1;"
            END IF
            PRINT #12, "}"
            'following line fixes problem related to RESUME after error
            'statementn = statementn + 1
            'if nochecks=0 then PRINT #12, "S_" + str2$(statementn) + ":;"
        END IF

        IF controltype(controllevel) <> 6 AND (controltype(controllevel) < 10 OR controltype(controllevel) > 17) THEN a$ = "CASE without SELECT CASE": GOTO errmes
        IF n = 1 THEN a$ = "Expected CASE expression": GOTO errmes
        SelectCaseHasCaseBlock(SelectCaseCounter) = -1


        'upgrade:
        '#1: variables can be referred to directly by storing an id in 'controlref'
        '    (but not if part of an array etc.)
        'DIM controlvalue(1000) AS LONG
        '#2: more types will be available
        '    +SINGLE
        '    +DOUBLE
        '    -LONG DOUBLE
        '    +INT32
        '    +UINT32
        '14=SELECT CASE float ...
        '15=SELECT CASE double
        '16=SELECT CASE int32
        '17=SELECT CASE uint32

        '10=SELECT CASE qbs (awaiting END SELECT/CASE)
        '11=SELECT CASE int64 (awaiting END SELECT/CASE)
        '12=SELECT CASE uint64 (awaiting END SELECT/CASE)
        '13=SELECT CASE LONG double (awaiting END SELECT/CASE/CASE ELSE)
        '14=SELECT CASE float ...
        '15=SELECT CASE double
        '16=SELECT CASE int32
        '17=SELECT CASE uint32

        '    bits = targettyp AND 511
        '                                IF bits <= 16 THEN e$ = "qbr_float_to_long(" + e$ + ")"
        '                                IF bits > 16 AND bits < 32 THEN e$ = "qbr_double_to_long(" + e$ + ")"
        '                                IF bits >= 32 THEN e$ = "qbr(" + e$ + ")"


        t = controltype(controllevel) - 10
        'get required type cast, and float options
        flt = 0
        IF t = 0 THEN tc$ = ""
        IF t = 1 THEN tc$ = ""
        IF t = 2 THEN tc$ = ""
        IF t = 3 THEN tc$ = "": flt = 1
        IF t = 4 THEN tc$ = "(float)": flt = 1
        IF t = 5 THEN tc$ = "(double)": flt = 1
        IF t = 6 THEN tc$ = ""
        IF t = 7 THEN tc$ = ""

        n$ = "sc_" + str2$(controlid(controllevel))
        cv = controlvalue(controllevel)
        IF cv THEN
            n$ = refer$(str2$(cv), 0, 0)
            IF Error_Happened THEN GOTO errmes
        END IF

        'CASE ELSE
        IF n = 2 THEN
            IF getelement$(a$, 2) = "C-EL" THEN
                IF EveryCaseSet(SelectCaseCounter) THEN PRINT #12, "if (sc_" + str2$(controlid(controllevel)) + "_var==0) {"
                controllevel = controllevel + 1: controltype(controllevel) = 19
                controlref(controllevel) = controlref(controllevel - 1)
                l$ = l$ + sp + SCase$("Else")
                layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
                GOTO finishednonexec '***no error causing code, event checking done by SELECT CASE***
            END IF
        END IF

        IF NoChecks = 0 THEN
            PRINT #12, "S_" + str2$(statementn) + ":;": dynscope = 1
            IF vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
                vWatchAddLabel linenumber, 0
                PRINT #12, "*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            END IF
        END IF



        f12$ = ""

        nexp = 0
        B = 0
        e$ = ""
        FOR i = 2 TO n
            e2$ = getelement$(ca$, i)
            IF e2$ = "(" THEN B = B + 1
            IF e2$ = ")" THEN B = B - 1
            IF i = n THEN e$ = e$ + sp + e2$
            IF i = n OR (e2$ = "," AND B = 0) THEN
                IF nexp <> 0 THEN l$ = l$ + sp2 + ",": f12$ = f12$ + "||"
                IF e$ = "" THEN a$ = "Expected expression": GOTO errmes
                e$ = RIGHT$(e$, LEN(e$) - 1)



                'TYPE 1? ... TO ...
                n2 = numelements(e$)
                b2 = 0
                el$ = "": er$ = ""
                usedto = 0
                FOR i2 = 1 TO n2
                    e3$ = getelement$(e$, i2)
                    IF e3$ = "(" THEN b2 = b2 + 1
                    IF e3$ = ")" THEN b2 = b2 - 1
                    IF b2 = 0 AND UCASE$(e3$) = "TO" THEN
                        usedto = 1
                    ELSE
                        IF usedto = 0 THEN el$ = el$ + sp + e3$ ELSE er$ = er$ + sp + e3$
                    END IF
                NEXT
                IF usedto = 1 THEN
                    IF el$ = "" OR er$ = "" THEN a$ = "Expected expression TO expression": GOTO errmes
                    el$ = RIGHT$(el$, LEN(el$) - 1): er$ = RIGHT$(er$, LEN(er$) - 1)
                    'evaluate each side
                    FOR i2 = 1 TO 2
                        IF i2 = 1 THEN e$ = el$ ELSE e$ = er$
                        e$ = fixoperationorder(e$)
                        IF Error_Happened THEN GOTO errmes
                        IF i2 = 1 THEN l$ = l$ + sp + tlayout$ ELSE l$ = l$ + sp + SCase$("To") + sp + tlayout$
                        e$ = evaluate(e$, typ)
                        IF Error_Happened THEN GOTO errmes
                        IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                        IF Error_Happened THEN GOTO errmes
                        IF t = 0 THEN
                            IF (typ AND ISSTRING) = 0 THEN a$ = "Expected string expression": GOTO errmes
                            IF i2 = 1 THEN f12$ = f12$ + "(qbs_greaterorequal(" + n$ + "," + e$ + ")&&qbs_lessorequal(" + n$ + ","
                            IF i2 = 2 THEN f12$ = f12$ + e$ + "))"
                        ELSE
                            IF (typ AND ISSTRING) THEN a$ = "Expected numeric expression": GOTO errmes
                            'round to integer?
                            IF (typ AND ISFLOAT) THEN
                                IF t = 1 THEN e$ = "qbr(" + e$ + ")"
                                IF t = 2 THEN e$ = "qbr_longdouble_to_uint64(" + e$ + ")"
                                IF t = 6 OR t = 7 THEN e$ = "qbr_double_to_long(" + e$ + ")"
                            END IF
                            'cast result?
                            IF LEN(tc$) THEN e$ = tc$ + "(" + e$ + ")"
                            IF i2 = 1 THEN f12$ = f12$ + "((" + n$ + ">=(" + e$ + "))&&(" + n$ + "<=("
                            IF i2 = 2 THEN f12$ = f12$ + e$ + ")))"
                        END IF
                    NEXT
                    GOTO addedexp
                END IF

                '10=SELECT CASE qbs (awaiting END SELECT/CASE)
                '11=SELECT CASE int64 (awaiting END SELECT/CASE)
                '12=SELECT CASE uint64 (awaiting END SELECT/CASE)
                '13=SELECT CASE LONG double (awaiting END SELECT/CASE/CASE ELSE)
                '14=SELECT CASE float ...
                '15=SELECT CASE double
                '16=SELECT CASE int32
                '17=SELECT CASE uint32

                '    bits = targettyp AND 511
                '                                IF bits <= 16 THEN e$ = "qbr_float_to_long(" + e$ + ")"
                '                                IF bits > 16 AND bits < 32 THEN e$ = "qbr_double_to_long(" + e$ + ")"
                '                                IF bits >= 32 THEN e$ = "qbr(" + e$ + ")"






                o$ = "==" 'used by type 3

                'TYPE 2?
                x$ = getelement$(e$, 1)
                IF isoperator(x$) THEN 'non-standard usage correction
                IF x$ = "=" OR x$ = "<>" OR x$ = ">" OR x$ = "<" OR x$ = ">=" OR x$ = "<=" THEN
                    e$ = "IS" + sp + e$
                    x$ = "IS"
                END IF
            END IF
            IF UCASE$(x$) = "IS" THEN
                n2 = numelements(e$)
                IF n2 < 3 THEN a$ = "Expected IS =,<>,>,<,>=,<= expression": GOTO errmes
                o$ = getelement$(e$, 2)
                o2$ = o$
                o = 0
                IF o$ = "=" THEN o$ = "==": o = 1
                IF o$ = "<>" THEN o$ = "!=": o = 1
                IF o$ = ">" THEN o = 1
                IF o$ = "<" THEN o = 1
                IF o$ = ">=" THEN o = 1
                IF o$ = "<=" THEN o = 1
                IF o <> 1 THEN a$ = "Expected IS =,<>,>,<,>=,<= expression": GOTO errmes
                l$ = l$ + sp + SCase$("Is") + sp + o2$
                e$ = getelements$(e$, 3, n2)
                'fall through to type 3 using modified e$ & o$
            END IF

            'TYPE 3? simple expression
            e$ = fixoperationorder(e$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp + tlayout$
            e$ = evaluate(e$, typ)
            IF Error_Happened THEN GOTO errmes
            IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
            IF Error_Happened THEN GOTO errmes
            IF t = 0 THEN
                'string comparison
                IF (typ AND ISSTRING) = 0 THEN a$ = "Expected string expression": GOTO errmes
                IF o$ = "==" THEN o$ = "qbs_equal"
                IF o$ = "!=" THEN o$ = "qbs_notequal"
                IF o$ = ">" THEN o$ = "qbs_greaterthan"
                IF o$ = "<" THEN o$ = "qbs_lessthan"
                IF o$ = ">=" THEN o$ = "qbs_greaterorequal"
                IF o$ = "<=" THEN o$ = "qbs_lessorequal"
                f12$ = f12$ + o$ + "(" + n$ + "," + e$ + ")"
            ELSE
                'numeric
                IF (typ AND ISSTRING) THEN a$ = "Expected numeric expression": GOTO errmes
                'round to integer?
                IF (typ AND ISFLOAT) THEN
                    IF t = 1 THEN e$ = "qbr(" + e$ + ")"
                    IF t = 2 THEN e$ = "qbr_longdouble_to_uint64(" + e$ + ")"
                    IF t = 6 OR t = 7 THEN e$ = "qbr_double_to_long(" + e$ + ")"
                END IF
                'cast result?
                IF LEN(tc$) THEN e$ = tc$ + "(" + e$ + ")"
                f12$ = f12$ + "(" + n$ + o$ + "(" + e$ + "))"
            END IF

            addedexp:
            e$ = ""
            nexp = nexp + 1
        ELSE
            e$ = e$ + sp + e2$
        END IF
    NEXT

    IF stringprocessinghappened THEN
        PRINT #12, "if ((" + cleanupstringprocessingcall$ + f12$ + "))||new_error){"
    ELSE
        PRINT #12, "if ((" + f12$ + ")||new_error){"
    END IF

    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    controllevel = controllevel + 1
    controlref(controllevel) = controlref(controllevel - 1)
    controltype(controllevel) = 18
    GOTO finishedline
END IF
END IF












'static scope commands:

IF NoChecks = 0 THEN
    IF vWatchOn = 1 AND inclinenumber(inclevel) = 0 THEN
        vWatchAddLabel linenumber, 0
        PRINT #12, "do{*__LONG_VWATCH_LINENUMBER= " + str2$(linenumber) + "; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
    ELSE
        PRINT #12, "do{"
    END IF
    'PRINT #12, "S_" + str2$(statementn) + ":;"
END IF


IF n > 1 THEN
    IF firstelement$ = "PALETTE" THEN
        IF secondelement$ = "USING" THEN
            l$ = SCase$("Palette" + sp + "Using" + sp)
            IF n < 3 THEN a$ = "Expected PALETTE USING array-name": GOTO errmes
            'check array
            e$ = getelement$(ca$, 3)
            IF FindArray(e$) THEN
                IF Error_Happened THEN GOTO errmes
                z = 1
                t = id.arraytype
                IF (t AND 511) <> 16 AND (t AND 511) <> 32 THEN z = 0
                IF t AND ISFLOAT THEN z = 0
                IF t AND ISOFFSETINBITS THEN z = 0
                IF t AND ISSTRING THEN z = 0
                IF t AND ISUDT THEN z = 0
                IF t AND ISUNSIGNED THEN z = 0
                IF z = 0 THEN a$ = "Array must be of type INTEGER or LONG": GOTO errmes
                bits = t AND 511
                GOTO pu_gotarray
            END IF
            IF Error_Happened THEN GOTO errmes
            a$ = "Expected PALETTE USING array-name": GOTO errmes
            pu_gotarray:
            'add () if index not specified
            IF n = 3 THEN
                e$ = e$ + sp + "(" + sp + ")"
            ELSE
                IF n = 4 OR getelement$(a$, 4) <> "(" OR getelement$(a$, n) <> ")" THEN a$ = "Expected PALETTE USING array-name(...)": GOTO errmes
                e$ = e$ + sp + getelements$(ca$, 4, n)
            END IF
            e$ = fixoperationorder$(e$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + tlayout$
            e$ = evaluatetotyp(e$, -2)
            IF Error_Happened THEN GOTO errmes
            PRINT #12, "sub_paletteusing(" + e$ + "," + str2(bits) + ");"
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        END IF 'using
    END IF 'palette
END IF 'n>1


IF firstelement$ = "KEY" THEN
    IF n = 1 THEN a$ = "Expected KEY ...": GOTO errmes
    l$ = SCase$("KEY") + sp
    IF secondelement$ = "OFF" THEN
        IF n > 2 THEN a$ = "Expected KEY OFF only": GOTO errmes
        l$ = l$ + SCase$("Off"): layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        PRINT #12, "key_off();"
        GOTO finishedline
    END IF
    IF secondelement$ = "ON" THEN
        IF n > 2 THEN a$ = "Expected KEY ON only": GOTO errmes
        l$ = l$ + SCase$("On"): layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        PRINT #12, "key_on();"
        GOTO finishedline
    END IF
    IF secondelement$ = "LIST" THEN
        IF n > 2 THEN a$ = "Expected KEY LIST only": GOTO errmes
        l$ = l$ + SCase$("List"): layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        PRINT #12, "key_list();"
        GOTO finishedline
    END IF
    'search for comma to indicate assignment
    B = 0: e$ = ""
    FOR i = 2 TO n
        e2$ = getelement(ca$, i)
        IF e2$ = "(" THEN B = B + 1
        IF e2$ = ")" THEN B = B - 1
        IF e2$ = "," AND B = 0 THEN
            i = i + 1: GOTO key_assignment
        END IF
        IF LEN(e$) THEN e$ = e$ + sp + e2$ ELSE e$ = e2$
    NEXT
    'assume KEY(x) ON/OFF/STOP and handle as a sub
    GOTO key_fallthrough
    key_assignment:
    'KEY x, "string"
    'index
    e$ = fixoperationorder(e$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + tlayout$ + sp2 + "," + sp
    e$ = evaluatetotyp(e$, 32&)
    IF Error_Happened THEN GOTO errmes
    PRINT #12, "key_assign(" + e$ + ",";
    'string
    e$ = getelements$(ca$, i, n)
    e$ = fixoperationorder(e$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + tlayout$
    e$ = evaluatetotyp(e$, ISSTRING)
    IF Error_Happened THEN GOTO errmes
    PRINT #12, e$ + ");"
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    GOTO finishedline
END IF 'KEY
key_fallthrough:




IF firstelement$ = "FIELD" THEN

    'get filenumber
    B = 0: e$ = ""
    FOR i = 2 TO n
        e2$ = getelement(ca$, i)
        IF e2$ = "(" THEN B = B + 1
        IF e2$ = ")" THEN B = B - 1
        IF e2$ = "," AND B = 0 THEN
            i = i + 1: GOTO fieldgotfn
        END IF
        IF LEN(e$) THEN e$ = e$ + sp + e2$ ELSE e$ = e2$
    NEXT
    GOTO fielderror
    fieldgotfn:
    IF e$ = "#" OR LEN(e$) = 0 THEN GOTO fielderror
    IF LEFT$(e$, 2) = "#" + sp THEN e$ = RIGHT$(e$, LEN(e$) - 2): l$ = SCase$("Field") + sp + "#" + sp2 ELSE l$ = SCase$("Field") + sp
    e$ = fixoperationorder(e$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + tlayout$ + sp2 + "," + sp
    e$ = evaluatetotyp(e$, 32&)
    IF Error_Happened THEN GOTO errmes
    PRINT #12, "field_new(" + e$ + ");"

    fieldnext:

    'get fieldwidth
    IF i > n THEN GOTO fielderror
    B = 0: e$ = ""
    FOR i = i TO n
        e2$ = getelement(ca$, i)
        IF e2$ = "(" THEN B = B + 1
        IF e2$ = ")" THEN B = B - 1
        IF UCASE$(e2$) = "AS" AND B = 0 THEN
            i = i + 1: GOTO fieldgotfw
        END IF
        IF LEN(e$) THEN e$ = e$ + sp + e2$ ELSE e$ = e2$
    NEXT
    GOTO fielderror
    fieldgotfw:
    IF LEN(e$) = 0 THEN GOTO fielderror
    e$ = fixoperationorder(e$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + tlayout$ + sp + SCase$("As") + sp
    sizee$ = evaluatetotyp(e$, 32&)
    IF Error_Happened THEN GOTO errmes

    'get variable name
    IF i > n THEN GOTO fielderror
    B = 0: e$ = ""
    FOR i = i TO n
        e2$ = getelement(ca$, i)
        IF e2$ = "(" THEN B = B + 1
        IF e2$ = ")" THEN B = B - 1
        IF (i = n OR e2$ = ",") AND B = 0 THEN
            IF e2$ = "," THEN i = i - 1
            IF i = n THEN
                IF LEN(e$) THEN e$ = e$ + sp + e2$ ELSE e$ = e2$
            END IF
            GOTO fieldgotfname
        END IF
        IF LEN(e$) THEN e$ = e$ + sp + e2$ ELSE e$ = e2$
    NEXT
    GOTO fielderror
    fieldgotfname:
    IF LEN(e$) = 0 THEN GOTO fielderror
    'evaluate it to check it is a STRING
    e$ = fixoperationorder(e$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + tlayout$
    e$ = evaluate(e$, typ)
    IF Error_Happened THEN GOTO errmes
    IF (typ AND ISSTRING) = 0 THEN GOTO fielderror
    IF typ AND ISFIXEDLENGTH THEN a$ = "Fixed length strings cannot be used in a FIELD statement": GOTO errmes
    IF (typ AND ISREFERENCE) = 0 THEN GOTO fielderror
    e$ = refer(e$, typ, 0)
    IF Error_Happened THEN GOTO errmes
    PRINT #12, "field_add(" + e$ + "," + sizee$ + ");"

    IF i < n THEN
        i = i + 1
        e$ = getelement(a$, i)
        IF e$ <> "," THEN a$ = "Expected ,": GOTO errmes
        l$ = l$ + sp2 + "," + sp
        i = i + 1
        GOTO fieldnext
    END IF

    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    GOTO finishedline

    fielderror: a$ = "Expected FIELD #filenumber, characters AS variable$, ...": GOTO errmes
END IF





'1=IF (awaiting END IF)
'2=FOR (awaiting NEXT)
'3=DO (awaiting LOOP [UNTIL|WHILE param])
'4=DO WHILE/UNTIL (awaiting LOOP)
'5=WHILE (awaiting WEND)

IF n = 2 THEN
    IF firstelement$ = "EXIT" THEN

        l$ = SCase$("Exit") + sp

        IF secondelement$ = "DO" THEN
            'scan backwards until previous control level reached
            l$ = l$ + SCase$("Do")
            FOR i = controllevel TO 1 STEP -1
                t = controltype(i)
                IF t = 3 OR t = 4 THEN
                    PRINT #12, "goto dl_exit_" + str2$(controlid(i)) + ";"
                    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
                    GOTO finishedline
                END IF
            NEXT
            a$ = "EXIT DO without DO": GOTO errmes
        END IF

        IF secondelement$ = "FOR" THEN
            'scan backwards until previous control level reached
            l$ = l$ + SCase$("For")
            FOR i = controllevel TO 1 STEP -1
                t = controltype(i)
                IF t = 2 THEN
                    PRINT #12, "goto fornext_exit_" + str2$(controlid(i)) + ";"
                    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
                    GOTO finishedline
                END IF
            NEXT
            a$ = "EXIT FOR without FOR": GOTO errmes
        END IF

        IF secondelement$ = "WHILE" THEN
            'scan backwards until previous control level reached
            l$ = l$ + SCase$("While")
            FOR i = controllevel TO 1 STEP -1
                t = controltype(i)
                IF t = 5 THEN
                    PRINT #12, "goto ww_exit_" + str2$(controlid(i)) + ";"
                    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
                    GOTO finishedline
                END IF
            NEXT
            a$ = "EXIT WHILE without WHILE": GOTO errmes
        END IF

        IF secondelement$ = "SELECT" THEN
            'scan backwards until previous control level reached
            l$ = l$ + SCase$("Select")
            FOR i = controllevel TO 1 STEP -1
                t = controltype(i)
                IF t = 18 OR t = 19 THEN 'CASE/CASE ELSE
                PRINT #12, "goto sc_" + str2$(controlid(i - 1)) + "_end;"
                layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
                GOTO finishedline
            END IF
        NEXT
        a$ = "EXIT SELECT without SELECT": GOTO errmes
    END IF

    IF secondelement$ = "CASE" THEN
        'scan backwards until previous control level reached
        l$ = l$ + SCase$("Case")
        FOR i = controllevel TO 1 STEP -1
            t = controltype(i)
            IF t = 18 THEN 'CASE
            PRINT #12, "goto sc_ec_" + str2$(everycasenewcase + 1) + "_end;"
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        ELSEIF t = 19 THEN 'CASE ELSE
            PRINT #12, "goto sc_" + str2$(controlid(i - 1)) + "_end;"
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        END IF
    NEXT
    a$ = "EXIT CASE without CASE": GOTO errmes
END IF

END IF
END IF








IF n >= 2 THEN
    IF firstelement$ = "ON" AND secondelement$ = "STRIG" THEN
        DEPENDENCY(DEPENDENCY_DEVICEINPUT) = 1
        i = 3
        IF i > n THEN a$ = "Expected (": GOTO errmes
        a2$ = getelement$(ca$, i): i = i + 1
        IF a2$ <> "(" THEN a$ = "Expected (": GOTO errmes
        l$ = SCase$("On" + sp + "Strig" + sp2 + "(")
        IF i > n THEN a$ = "Expected ...": GOTO errmes
        B = 0
        x = 0
        e2$ = ""
        e3$ = ""
        FOR i = i TO n
            e$ = getelement$(ca$, i)
            a = ASC(e$)
            IF a = 40 THEN B = B + 1
            IF a = 41 THEN B = B - 1
            IF B = -1 THEN GOTO onstriggotarg
            IF a = 44 AND B = 0 THEN
                x = x + 1
                IF x > 1 THEN a$ = "Expected )": GOTO errmes
                IF e2$ = "" THEN a$ = "Expected ... ,": GOTO errmes
                e3$ = e2$
                e2$ = ""
            ELSE
                IF LEN(e2$) THEN e2$ = e2$ + sp + e$ ELSE e2$ = e$
            END IF
        NEXT
        a$ = "Expected )": GOTO errmes
        onstriggotarg:
        IF e2$ = "" THEN a$ = "Expected ... )": GOTO errmes
        PRINT #12, "onstrig_setup(";

        'sort scanned results
        IF LEN(e3$) THEN
            optI$ = e3$
            optController$ = e2$
            optPassed$ = "1"
        ELSE
            optI$ = e2$
            optController$ = "0"
            optPassed$ = "0"
        END IF

        'i
        e$ = fixoperationorder$(optI$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + tlayout$
        e$ = evaluatetotyp(e$, 32&): IF Error_Happened THEN GOTO errmes
        PRINT #12, e$ + ",";

        'controller , passed
        IF optPassed$ = "1" THEN
            e$ = fixoperationorder$(optController$): IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp2 + "," + sp + tlayout$
            e$ = evaluatetotyp(e$, 32&): IF Error_Happened THEN GOTO errmes
        ELSE
            e$ = optController$
        END IF
        PRINT #12, e$ + "," + optPassed$ + ",";

        l$ = l$ + sp2 + ")" + sp 'close brackets

        i = i + 1
        IF i > n THEN a$ = "Expected GOSUB/sub-name": GOTO errmes
        a2$ = getelement$(a$, i): i = i + 1
        onstrigid = onstrigid + 1
        PRINT #12, str2$(onstrigid) + ",";

        IF a2$ = "GOSUB" THEN
            IF i > n THEN a$ = "Expected linenumber/label": GOTO errmes
            a2$ = getelement$(ca$, i): i = i + 1

            PRINT #12, "0);"

            IF validlabel(a2$) = 0 THEN a$ = "Invalid label": GOTO errmes

            v = HashFind(a2$, HASHFLAG_LABEL, ignore, r)
            x = 1
            labchk60z:
            IF v THEN
                s = Labels(r).Scope
                IF s = 0 OR s = -1 THEN 'main scope?
                IF s = -1 THEN Labels(r).Scope = 0 'acquire scope
                x = 0 'already defined
                tlayout$ = RTRIM$(Labels(r).cn)
                Labels(r).Scope_Restriction = subfuncn
                Labels(r).Error_Line = linenumber
            ELSE
                IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk60z
            END IF
        END IF
        IF x THEN
            'does not exist
            nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
            Labels(nLabels) = Empty_Label
            HashAdd a2$, HASHFLAG_LABEL, nLabels
            r = nLabels
            Labels(r).State = 0
            Labels(r).cn = a2$
            Labels(r).Scope = 0
            Labels(r).Error_Line = linenumber
            Labels(r).Scope_Restriction = subfuncn
        END IF 'x
        l$ = l$ + SCase$("GoSub") + sp + tlayout$

        PRINT #30, "if(strig_event_id==" + str2$(onstrigid) + ")goto LABEL_" + a2$ + ";"

        PRINT #29, "case " + str2$(onstrigid) + ":"
        PRINT #29, "strig_event_occurred++;"
        PRINT #29, "strig_event_id=" + str2$(onstrigid) + ";"
        PRINT #29, "strig_event_occurred++;"
        PRINT #29, "return_point[next_return_point++]=0;"
        PRINT #29, "if (next_return_point>=return_points) more_return_points();"
        PRINT #29, "QBMAIN(NULL);"
        PRINT #29, "break;"

        IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$
        layoutdone = 1
        GOTO finishedline

    ELSE

        'establish whether sub a2$ exists using try
        x = 0
        try = findid(a2$)
        IF Error_Happened THEN GOTO errmes
        DO WHILE try
            IF id.subfunc = 2 THEN x = 1: EXIT DO
            IF try = 2 THEN findanotherid = 1: try = findid(a2$) ELSE try = 0
            IF Error_Happened THEN GOTO errmes
        LOOP
        IF x = 0 THEN a$ = "Expected GOSUB/sub": GOTO errmes

        l$ = l$ + RTRIM$(id.cn)

        PRINT #29, "case " + str2$(onstrigid) + ":"
        PRINT #29, RTRIM$(id.callname) + "(";

        IF id.args > 1 THEN a$ = "SUB requires more than one argument": GOTO errmes

        IF i > n THEN

            IF id.args = 1 THEN a$ = "Expected argument after SUB": GOTO errmes
            PRINT #12, "0);"
            PRINT #29, ");"

        ELSE

            IF id.args = 0 THEN a$ = "SUB has no arguments": GOTO errmes

            t = CVL(id.arg)
            B = t AND 511
            IF B = 0 OR (t AND ISARRAY) <> 0 OR (t AND ISFLOAT) <> 0 OR (t AND ISSTRING) <> 0 OR (t AND ISOFFSETINBITS) <> 0 THEN a$ = "Only SUB arguments of integer-type allowed": GOTO errmes
            IF B = 8 THEN ct$ = "int8"
            IF B = 16 THEN ct$ = "int16"
            IF B = 32 THEN ct$ = "int32"
            IF B = 64 THEN ct$ = "int64"
            IF t AND ISOFFSET THEN ct$ = "ptrszint"
            IF t AND ISUNSIGNED THEN ct$ = "u" + ct$
            PRINT #29, "(" + ct$ + "*)&i64);"

            e$ = getelements$(ca$, i, n)
            e$ = fixoperationorder$(e$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp + tlayout$
            e$ = evaluatetotyp(e$, INTEGER64TYPE - ISPOINTER)
            IF Error_Happened THEN GOTO errmes
            PRINT #12, e$ + ");"

        END IF

        PRINT #29, "break;"
        IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$
        layoutdone = 1
        GOTO finishedline
    END IF

END IF
END IF












IF n >= 2 THEN
    IF firstelement$ = "ON" AND secondelement$ = "TIMER" THEN
        i = 3
        IF i > n THEN a$ = "Expected (": GOTO errmes
        a2$ = getelement$(ca$, i): i = i + 1
        IF a2$ <> "(" THEN a$ = "Expected (": GOTO errmes
        l$ = SCase$("On" + sp + "Timer" + sp2 + "(")
        IF i > n THEN a$ = "Expected ...": GOTO errmes
        B = 0
        x = 0
        e2$ = ""
        e3$ = ""
        FOR i = i TO n
            e$ = getelement$(ca$, i)
            a = ASC(e$)
            IF a = 40 THEN B = B + 1
            IF a = 41 THEN B = B - 1
            IF B = -1 THEN GOTO ontimgotarg
            IF a = 44 AND B = 0 THEN
                x = x + 1
                IF x > 1 THEN a$ = "Expected )": GOTO errmes
                IF e2$ = "" THEN a$ = "Expected ... ,": GOTO errmes
                e3$ = e2$
                e2$ = ""
            ELSE
                IF LEN(e2$) THEN e2$ = e2$ + sp + e$ ELSE e2$ = e$
            END IF
        NEXT
        a$ = "Expected )": GOTO errmes
        ontimgotarg:
        IF e2$ = "" THEN a$ = "Expected ... )": GOTO errmes
        PRINT #12, "ontimer_setup(";
        'i
        IF LEN(e3$) THEN
            e$ = fixoperationorder$(e3$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp2 + tlayout$ + "," + sp
            e$ = evaluatetotyp(e$, 32&)
            IF Error_Happened THEN GOTO errmes
            PRINT #12, e$ + ",";
        ELSE
            PRINT #12, "0,";
            l$ = l$ + sp2
        END IF
        'sec
        e$ = fixoperationorder$(e2$)
        IF Error_Happened THEN GOTO errmes
        l$ = l$ + tlayout$ + sp2 + ")" + sp
        e$ = evaluatetotyp(e$, DOUBLETYPE - ISPOINTER)
        IF Error_Happened THEN GOTO errmes
        PRINT #12, e$ + ",";
        i = i + 1
        IF i > n THEN a$ = "Expected GOSUB/sub-name": GOTO errmes
        a2$ = getelement$(a$, i): i = i + 1
        ontimerid = ontimerid + 1
        PRINT #12, str2$(ontimerid) + ",";

        IF a2$ = "GOSUB" THEN
            IF i > n THEN a$ = "Expected linenumber/label": GOTO errmes
            a2$ = getelement$(ca$, i): i = i + 1

            PRINT #12, "0);"

            IF validlabel(a2$) = 0 THEN a$ = "Invalid label": GOTO errmes

            v = HashFind(a2$, HASHFLAG_LABEL, ignore, r)
            x = 1
            labchk60:
            IF v THEN
                s = Labels(r).Scope
                IF s = 0 OR s = -1 THEN 'main scope?
                IF s = -1 THEN Labels(r).Scope = 0 'acquire scope
                x = 0 'already defined
                tlayout$ = RTRIM$(Labels(r).cn)
                Labels(r).Scope_Restriction = subfuncn
                Labels(r).Error_Line = linenumber
            ELSE
                IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk60
            END IF
        END IF
        IF x THEN
            'does not exist
            nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
            Labels(nLabels) = Empty_Label
            HashAdd a2$, HASHFLAG_LABEL, nLabels
            r = nLabels
            Labels(r).State = 0
            Labels(r).cn = a2$
            Labels(r).Scope = 0
            Labels(r).Error_Line = linenumber
            Labels(r).Scope_Restriction = subfuncn
        END IF 'x
        l$ = l$ + SCase$("GoSub") + sp + tlayout$

        PRINT #25, "if(timer_event_id==" + str2$(ontimerid) + ")goto LABEL_" + a2$ + ";"

        PRINT #24, "case " + str2$(ontimerid) + ":"
        PRINT #24, "timer_event_occurred++;"
        PRINT #24, "timer_event_id=" + str2$(ontimerid) + ";"
        PRINT #24, "timer_event_occurred++;"
        PRINT #24, "return_point[next_return_point++]=0;"
        PRINT #24, "if (next_return_point>=return_points) more_return_points();"
        PRINT #24, "QBMAIN(NULL);"
        PRINT #24, "break;"



        'call validlabel (to validate the label) [see goto]
        'increment ontimerid
        'use ontimerid to generate the jumper routine
        'etc.


        IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$
        layoutdone = 1
        GOTO finishedline
    ELSE

        'establish whether sub a2$ exists using try
        x = 0
        try = findid(a2$)
        IF Error_Happened THEN GOTO errmes
        DO WHILE try
            IF id.subfunc = 2 THEN x = 1: EXIT DO
            IF try = 2 THEN findanotherid = 1: try = findid(a2$) ELSE try = 0
            IF Error_Happened THEN GOTO errmes
        LOOP
        IF x = 0 THEN a$ = "Expected GOSUB/sub": GOTO errmes

        l$ = l$ + RTRIM$(id.cn)

        PRINT #24, "case " + str2$(ontimerid) + ":"
        PRINT #24, RTRIM$(id.callname) + "(";

        IF id.args > 1 THEN a$ = "SUB requires more than one argument": GOTO errmes

        IF i > n THEN

            IF id.args = 1 THEN a$ = "Expected argument after SUB": GOTO errmes
            PRINT #12, "0);"
            PRINT #24, ");"

        ELSE

            IF id.args = 0 THEN a$ = "SUB has no arguments": GOTO errmes

            t = CVL(id.arg)
            B = t AND 511
            IF B = 0 OR (t AND ISARRAY) <> 0 OR (t AND ISFLOAT) <> 0 OR (t AND ISSTRING) <> 0 OR (t AND ISOFFSETINBITS) <> 0 THEN a$ = "Only SUB arguments of integer-type allowed": GOTO errmes
            IF B = 8 THEN ct$ = "int8"
            IF B = 16 THEN ct$ = "int16"
            IF B = 32 THEN ct$ = "int32"
            IF B = 64 THEN ct$ = "int64"
            IF t AND ISOFFSET THEN ct$ = "ptrszint"
            IF t AND ISUNSIGNED THEN ct$ = "u" + ct$
            PRINT #24, "(" + ct$ + "*)&i64);"

            e$ = getelements$(ca$, i, n)
            e$ = fixoperationorder$(e$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp + tlayout$
            e$ = evaluatetotyp(e$, INTEGER64TYPE - ISPOINTER)
            IF Error_Happened THEN GOTO errmes
            PRINT #12, e$ + ");"

        END IF

        PRINT #24, "break;"
        IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$
        layoutdone = 1
        GOTO finishedline
    END IF

END IF
END IF




IF n >= 2 THEN
    IF firstelement$ = "ON" AND secondelement$ = "KEY" THEN
        i = 3
        IF i > n THEN a$ = "Expected (": GOTO errmes
        a2$ = getelement$(ca$, i): i = i + 1
        IF a2$ <> "(" THEN a$ = "Expected (": GOTO errmes
        l$ = SCase$("On" + sp + "Key" + sp2 + "(")
        IF i > n THEN a$ = "Expected ...": GOTO errmes
        B = 0
        x = 0
        e2$ = ""
        FOR i = i TO n
            e$ = getelement$(ca$, i)
            a = ASC(e$)


            IF a = 40 THEN B = B + 1
            IF a = 41 THEN B = B - 1
            IF B = -1 THEN EXIT FOR
            IF LEN(e2$) THEN e2$ = e2$ + sp + e$ ELSE e2$ = e$
        NEXT
        IF i = n + 1 THEN a$ = "Expected )": GOTO errmes
        IF e2$ = "" THEN a$ = "Expected ... )": GOTO errmes

        e$ = fixoperationorder$(e2$)
        IF Error_Happened THEN GOTO errmes
        l$ = l$ + tlayout$ + sp2 + ")" + sp
        e$ = evaluatetotyp(e$, DOUBLETYPE - ISPOINTER)
        IF Error_Happened THEN GOTO errmes
        PRINT #12, "onkey_setup(" + e$ + ",";

        i = i + 1
        IF i > n THEN a$ = "Expected GOSUB/sub-name": GOTO errmes
        a2$ = getelement$(a$, i): i = i + 1
        onkeyid = onkeyid + 1
        PRINT #12, str2$(onkeyid) + ",";

        IF a2$ = "GOSUB" THEN
            IF i > n THEN a$ = "Expected linenumber/label": GOTO errmes
            a2$ = getelement$(ca$, i): i = i + 1

            PRINT #12, "0);"

            IF validlabel(a2$) = 0 THEN a$ = "Invalid label": GOTO errmes

            v = HashFind(a2$, HASHFLAG_LABEL, ignore, r)
            x = 1
            labchk61:
            IF v THEN
                s = Labels(r).Scope
                IF s = 0 OR s = -1 THEN 'main scope?
                IF s = -1 THEN Labels(r).Scope = 0 'acquire scope
                x = 0 'already defined
                tlayout$ = RTRIM$(Labels(r).cn)
                Labels(r).Scope_Restriction = subfuncn
                Labels(r).Error_Line = linenumber
            ELSE
                IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk61
            END IF
        END IF
        IF x THEN
            'does not exist
            nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
            Labels(nLabels) = Empty_Label
            HashAdd a2$, HASHFLAG_LABEL, nLabels
            r = nLabels
            Labels(r).State = 0
            Labels(r).cn = a2$
            Labels(r).Scope = 0
            Labels(r).Error_Line = linenumber
            Labels(r).Scope_Restriction = subfuncn
        END IF 'x
        l$ = l$ + SCase$("GoSub") + sp + tlayout$

        PRINT #28, "if(key_event_id==" + str2$(onkeyid) + ")goto LABEL_" + a2$ + ";"

        PRINT #27, "case " + str2$(onkeyid) + ":"
        PRINT #27, "key_event_occurred++;"
        PRINT #27, "key_event_id=" + str2$(onkeyid) + ";"
        PRINT #27, "key_event_occurred++;"
        PRINT #27, "return_point[next_return_point++]=0;"
        PRINT #27, "if (next_return_point>=return_points) more_return_points();"
        PRINT #27, "QBMAIN(NULL);"
        PRINT #27, "break;"

        IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$
        layoutdone = 1
        GOTO finishedline
    ELSE

        'establish whether sub a2$ exists using try
        x = 0
        try = findid(a2$)
        IF Error_Happened THEN GOTO errmes
        DO WHILE try
            IF id.subfunc = 2 THEN x = 1: EXIT DO
            IF try = 2 THEN findanotherid = 1: try = findid(a2$) ELSE try = 0
            IF Error_Happened THEN GOTO errmes
        LOOP
        IF x = 0 THEN a$ = "Expected GOSUB/sub": GOTO errmes

        l$ = l$ + RTRIM$(id.cn)

        PRINT #27, "case " + str2$(onkeyid) + ":"
        PRINT #27, RTRIM$(id.callname) + "(";

        IF id.args > 1 THEN a$ = "SUB requires more than one argument": GOTO errmes

        IF i > n THEN

            IF id.args = 1 THEN a$ = "Expected argument after SUB": GOTO errmes
            PRINT #12, "0);"
            PRINT #27, ");"

        ELSE

            IF id.args = 0 THEN a$ = "SUB has no arguments": GOTO errmes

            t = CVL(id.arg)
            B = t AND 511
            IF B = 0 OR (t AND ISARRAY) <> 0 OR (t AND ISFLOAT) <> 0 OR (t AND ISSTRING) <> 0 OR (t AND ISOFFSETINBITS) <> 0 THEN a$ = "Only SUB arguments of integer-type allowed": GOTO errmes
            IF B = 8 THEN ct$ = "int8"
            IF B = 16 THEN ct$ = "int16"
            IF B = 32 THEN ct$ = "int32"
            IF B = 64 THEN ct$ = "int64"
            IF t AND ISOFFSET THEN ct$ = "ptrszint"
            IF t AND ISUNSIGNED THEN ct$ = "u" + ct$
            PRINT #27, "(" + ct$ + "*)&i64);"

            e$ = getelements$(ca$, i, n)
            e$ = fixoperationorder$(e$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp + tlayout$
            e$ = evaluatetotyp(e$, INTEGER64TYPE - ISPOINTER)
            IF Error_Happened THEN GOTO errmes
            PRINT #12, e$ + ");"

        END IF

        PRINT #27, "break;"
        IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$
        layoutdone = 1
        GOTO finishedline
    END IF

END IF
END IF



























'SHARED (SUB)
IF n >= 1 THEN
    IF firstelement$ = "SHARED" THEN
        IF n = 1 THEN a$ = "Expected SHARED ...": GOTO errmes
        i = 2
        IF subfuncn = 0 THEN a$ = "SHARED must be used within a SUB/FUNCTION": GOTO errmes



        l$ = SCase$("Shared")
        subfuncshr:

        'get variable name
        n$ = getelement$(ca$, i): i = i + 1

        IF n$ = "" THEN a$ = "Expected SHARED variable-name or SHARED AS type variable-list": GOTO errmes

        IF UCASE$(n$) <> "AS" THEN
            'traditional dim syntax for SHARED
            newSharedSyntax = 0
            s$ = removesymbol(n$)
            IF Error_Happened THEN GOTO errmes
            l2$ = s$ 'either symbol or nothing

            'array?
            a = 0
            IF getelement$(a$, i) = "(" THEN
                IF getelement$(a$, i + 1) <> ")" THEN a$ = "Expected ()": GOTO errmes
                i = i + 2
                a = 1
                l2$ = l2$ + sp2 + "(" + sp2 + ")"
            END IF

            method = 1

            'specific type?
            t$ = ""
            ts$ = ""
            t3$ = ""
            IF getelement$(a$, i) = "AS" THEN
                l2$ = l2$ + sp + SCase$("As")
                getshrtyp:
                i = i + 1
                t2$ = getelement$(a$, i)
                IF t2$ <> "," AND t2$ <> "" THEN
                    IF t$ = "" THEN t$ = t2$ ELSE t$ = t$ + " " + t2$
                    IF t3$ = "" THEN t3$ = t2$ ELSE t3$ = t3$ + sp + t2$
                    GOTO getshrtyp
                END IF
                IF t$ = "" THEN a$ = "Expected AS type": GOTO errmes

                t = typname2typ(t$)
                IF Error_Happened THEN GOTO errmes
                IF t AND ISINCONVENTIONALMEMORY THEN t = t - ISINCONVENTIONALMEMORY
                IF t AND ISPOINTER THEN t = t - ISPOINTER
                IF t AND ISREFERENCE THEN t = t - ISREFERENCE
                tsize = typname2typsize
                method = 0
                IF (t AND ISUDT) = 0 THEN
                    ts$ = type2symbol$(t$)
                    l2$ = l2$ + sp + SCase2$(t3$)
                ELSE
                    t3$ = RTRIM$(udtxcname(t AND 511))
                    IF RTRIM$(udtxcname(t AND 511)) = "_MEM" AND UCASE$(t$) = "MEM" AND qbnexprefix_set = 1 THEN
                        t3$ = MID$(RTRIM$(udtxcname(t AND 511)), 2)
                    END IF
                    l2$ = l2$ + sp + t3$
                END IF
                IF Error_Happened THEN GOTO errmes

            END IF 'as

            IF LEN(s$) <> 0 AND LEN(t$) <> 0 THEN a$ = "Expected symbol or AS type after variable name": GOTO errmes

            'no symbol of type specified, apply default
            IF s$ = "" AND t$ = "" THEN
                IF LEFT$(n$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(n$)) - 64
                s$ = defineextaz(v)
            END IF

            NormalSharedBlock:
            'switch to main module
            oldsubfunc$ = subfunc$
            subfunc$ = ""
            defdatahandle = 18
            CLOSE #13: OPEN tmpdir$ + "maindata.txt" FOR APPEND AS #13
            CLOSE #19: OPEN tmpdir$ + "mainfree.txt" FOR APPEND AS #19

            'use 'try' to locate the variable (if it already exists)
            n2$ = n$ + s$ + ts$ 'note: either ts$ or s$ will exist unless it is a UDT
            try = findid(n2$)
            IF Error_Happened THEN GOTO errmes
            DO WHILE try
                IF a THEN
                    'an array

                    IF id.arraytype THEN
                        IF LEN(t$) = 0 THEN GOTO shrfound
                        t2 = id.arraytype: t2size = id.tsize
                        IF t2 AND ISINCONVENTIONALMEMORY THEN t2 = t2 - ISINCONVENTIONALMEMORY
                        IF t2 AND ISPOINTER THEN t2 = t2 - ISPOINTER
                        IF t2 AND ISREFERENCE THEN t2 = t2 - ISREFERENCE
                        IF t = t2 AND tsize = t2size THEN GOTO shrfound
                    END IF

                ELSE
                    'not an array

                    IF id.t THEN
                        IF LEN(t$) = 0 THEN GOTO shrfound
                        t2 = id.t: t2size = id.tsize
                        IF t2 AND ISINCONVENTIONALMEMORY THEN t2 = t2 - ISINCONVENTIONALMEMORY
                        IF t2 AND ISPOINTER THEN t2 = t2 - ISPOINTER
                        IF t2 AND ISREFERENCE THEN t2 = t2 - ISREFERENCE

                        IF Debug THEN PRINT #9, "SHARED:comparing:"; t; t2, tsize; t2size

                        IF t = t2 AND tsize = t2size THEN GOTO shrfound
                    END IF

                END IF

                IF try = 2 THEN findanotherid = 1: try = findid(n2$) ELSE try = 0
                IF Error_Happened THEN GOTO errmes
            LOOP
            'unknown variable
            IF a THEN a$ = "Array '" + n$ + "' not defined": GOTO errmes
            'create variable
            IF LEN(s$) THEN typ$ = s$ ELSE typ$ = t$
            IF optionexplicit THEN a$ = "Variable '" + n$ + "' (" + symbol2fulltypename$(typ$) + ") not defined": GOTO errmes
            bypassNextVariable = -1
            retval = dim2(n$, typ$, method, "")
            manageVariableList "", vWatchNewVariable$, 0, 2
            IF Error_Happened THEN GOTO errmes
            'note: variable created!

            shrfound:
            IF newSharedSyntax = 0 THEN
                l$ = l$ + sp + RTRIM$(id.cn) + l2$
            ELSE
                IF sharedAsLayoutAdded = 0 THEN
                    sharedAsLayoutAdded = -1
                    l$ = l$ + l2$ + sp$ + RTRIM$(id.cn) + l3$
                ELSE
                    l$ = l$ + sp$ + RTRIM$(id.cn) + l3$
                END IF
            END IF

            ids(currentid).share = ids(currentid).share OR 2 'set as temporarily shared

            'method must apply to the current sub/function regardless of how the variable was defined in 'main'
            lmay = LEN(RTRIM$(id.mayhave)): lmust = LEN(RTRIM$(id.musthave))
            IF lmay <> 0 OR lmust <> 0 THEN
                IF (method = 1 AND lmust = 0) OR (method = 0 AND lmay = 0) THEN
                    revertmaymusthaven = revertmaymusthaven + 1
                    revertmaymusthave(revertmaymusthaven) = currentid
                    SWAP ids(currentid).musthave, ids(currentid).mayhave
                END IF
            END IF

            'switch back to sub/func
            subfunc$ = oldsubfunc$
            defdatahandle = 13
            CLOSE #13: OPEN tmpdir$ + "data" + str2$(subfuncn) + ".txt" FOR APPEND AS #13
            CLOSE #19: OPEN tmpdir$ + "free" + str2$(subfuncn) + ".txt" FOR APPEND AS #19

            IF newSharedSyntax THEN RETURN

            IF getelement$(a$, i) = "," THEN i = i + 1: l$ = l$ + sp2 + ",": GOTO subfuncshr
            IF getelement$(a$, i) <> "" THEN a$ = "Expected ,": GOTO errmes

            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        ELSE
            'new dim syntax for SHARED!
            i = i - 1 'relocate back to "AS"

            'estabilish the data type:
            t$ = ""
            ts$ = ""
            t3$ = ""
            n$ = ""
            previousElement$ = ""
            l2$ = sp + SCase$("As")
            sharedAsLayoutAdded = 0
            getshrtyp2:
            i = i + 1
            t2$ = getelement$(a$, i)
            IF t2$ <> "," AND t2$ <> "(" AND t2$ <> "" THEN
                'get first variable name
                n$ = getelement$(ca$, i)

                IF LEN(previousElement$) THEN
                    IF t$ = "" THEN t$ = previousElement$ ELSE t$ = t$ + " " + previousElement$
                    IF t3$ = "" THEN t3$ = previousElement$ ELSE t3$ = t3$ + sp + previousElement$
                END IF
                previousElement$ = t2$
                GOTO getshrtyp2
            END IF
            IF t$ = "" THEN a$ = "Expected SHARED AS type variable-list or SHARED variable-name AS type": GOTO errmes

            t = typname2typ(t$)
            IF Error_Happened THEN GOTO errmes
            IF t AND ISINCONVENTIONALMEMORY THEN t = t - ISINCONVENTIONALMEMORY
            IF t AND ISPOINTER THEN t = t - ISPOINTER
            IF t AND ISREFERENCE THEN t = t - ISREFERENCE
            tsize = typname2typsize
            method = 0
            IF (t AND ISUDT) = 0 THEN
                ts$ = type2symbol$(t$)
                l2$ = l2$ + sp + SCase2$(t3$)
            ELSE
                t3$ = RTRIM$(udtxcname(t AND 511))
                IF RTRIM$(udtxcname(t AND 511)) = "_MEM" AND UCASE$(t$) = "MEM" AND qbnexprefix_set = 1 THEN
                    t3$ = MID$(RTRIM$(udtxcname(t AND 511)), 2)
                END IF
                l2$ = l2$ + sp + t3$
            END IF
            IF Error_Happened THEN GOTO errmes

            subfuncshr2:
            s$ = removesymbol(n$)
            IF Error_Happened THEN GOTO errmes
            IF s$ <> "" THEN
                a$ = "Cannot use type symbol with SHARED AS type variable-list (" + s$ + ")"
                GOTO errmes
            END IF

            'array?
            a = 0
            l3$ = ""
            IF getelement$(a$, i) = "(" THEN
                IF getelement$(a$, i + 1) <> ")" THEN a$ = "Expected ()": GOTO errmes
                i = i + 2
                a = 1
                l3$ = sp2 + "(" + sp2 + ")"
            END IF

            newSharedSyntax = -1
            GOSUB NormalSharedBlock
            newSharedSyntax = 0

            IF getelement$(a$, i) = "," THEN
                i = i + 1
                l$ = l$ + sp2 + ","

                'get next variable name
                n$ = getelement$(ca$, i): i = i + 1
                GOTO subfuncshr2
            END IF
            IF getelement$(a$, i) <> "" THEN a$ = "Expected ,": GOTO errmes

            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        END IF
    END IF
END IF

'EXIT SUB/FUNCTION
IF n = 2 THEN
    IF firstelement$ = "EXIT" THEN
        sf = 0
        IF secondelement$ = "FUNCTION" THEN sf = 1
        IF secondelement$ = "SUB" THEN sf = 2
        IF sf THEN

            IF LEN(subfunc) = 0 THEN a$ = "EXIT " + secondelement$ + " must be used within a " + secondelement$: GOTO errmes

            PRINT #12, "goto exit_subfunc;"
            IF LEFT$(subfunc, 4) = "SUB_" THEN secondelement$ = SCase$("Sub") ELSE secondelement$ = SCase$("Function")
            l$ = SCase$("Exit") + sp + secondelement$
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        END IF
    END IF
END IF


'_ECHO checking
IF firstelement$ = "_ECHO" OR (firstelement$ = "ECHO" AND qbnexprefix_set = 1) THEN
    IF Console = 0 THEN
        a$ = qbnexprefix$ + "ECHO requires $CONSOLE or $CONSOLE:ONLY to be set first": GOTO errmes
    END IF
END IF


'ASC statement (fully inline)
IF n >= 1 THEN
    IF firstelement$ = "ASC" THEN
        IF getelement$(a$, 2) <> "(" THEN a$ = "Expected ( after ASC": GOTO errmes

        'calculate 3 parts
        useposition = 0
        part = 1
        i = 3
        a3$ = ""
        stringvariable$ = ""
        position$ = ""
        B = 0
        DO

            IF i > n THEN 'got part 3
            IF part <> 3 OR LEN(a3$) = 0 THEN a$ = "Expected ASC ( ... , ... ) = ...": GOTO errmes
            expression$ = a3$
            EXIT DO
        END IF

        a2$ = getelement$(ca$, i)
        IF a2$ = "(" THEN B = B + 1
        IF a2$ = ")" THEN B = B - 1

        IF B = -1 THEN

            IF part = 1 THEN 'eg. ASC(a$)=65
            IF getelement$(a$, i + 1) <> "=" THEN a$ = "Expected =": GOTO errmes
            stringvariable$ = a3$
            position$ = "1"
            part = 3: a3$ = "": i = i + 1: GOTO ascgotpart
        END IF

        IF part = 2 THEN 'eg. ASC(a$,i)=65
        IF getelement$(a$, i + 1) <> "=" THEN a$ = "Expected =": GOTO errmes
        useposition = 1
        position$ = a3$
        part = 3: a3$ = "": i = i + 1: GOTO ascgotpart
    END IF

    'fall through, already in part 3

END IF

IF a2$ = "," AND B = 0 THEN
    IF part = 1 THEN stringvariable$ = a3$: part = 2: a3$ = "": GOTO ascgotpart
END IF

IF LEN(a3$) THEN a3$ = a3$ + sp + a2$ ELSE a3$ = a2$
ascgotpart:
i = i + 1
LOOP
IF LEN(stringvariable$) = 0 OR LEN(position$) = 0 THEN a$ = "Expected ASC ( ... , ... ) = ...": GOTO errmes

'validate stringvariable$
stringvariable$ = fixoperationorder$(stringvariable$)
IF Error_Happened THEN GOTO errmes
l$ = SCase$("Asc") + sp2 + "(" + sp2 + tlayout$

e$ = evaluate(stringvariable$, sourcetyp)
IF Error_Happened THEN GOTO errmes
IF (sourcetyp AND ISREFERENCE) = 0 OR (sourcetyp AND ISSTRING) = 0 THEN a$ = "Expected ASC ( string-variable , ...": GOTO errmes
stringvariable$ = evaluatetotyp(stringvariable$, ISSTRING)
IF Error_Happened THEN GOTO errmes



IF position$ = "1" THEN
    IF useposition THEN l$ = l$ + sp2 + "," + sp + "1" + sp2 + ")" + sp + "=" ELSE l$ = l$ + sp2 + ")" + sp + "="

    PRINT #12, "tqbs=" + stringvariable$ + "; if (!new_error){"
    e$ = fixoperationorder$(expression$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + sp + tlayout$
    e$ = evaluatetotyp(e$, 32&)
    IF Error_Happened THEN GOTO errmes
    PRINT #12, "tmp_long=" + e$ + "; if (!new_error){"
    PRINT #12, "if (tqbs->len){tqbs->chr[0]=tmp_long;}else{error(5);}"
    PRINT #12, "}}"

ELSE

    PRINT #12, "tqbs=" + stringvariable$ + "; if (!new_error){"
    e$ = fixoperationorder$(position$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + sp2 + "," + sp + tlayout$ + sp2 + ")" + sp + "="
    e$ = evaluatetotyp(e$, 32&)
    IF Error_Happened THEN GOTO errmes
    PRINT #12, "tmp_fileno=" + e$ + "; if (!new_error){"
    e$ = fixoperationorder$(expression$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + sp + tlayout$
    e$ = evaluatetotyp(e$, 32&)
    IF Error_Happened THEN GOTO errmes
    PRINT #12, "tmp_long=" + e$ + "; if (!new_error){"
    PRINT #12, "if ((tmp_fileno>0)&&(tmp_fileno<=tqbs->len)){tqbs->chr[tmp_fileno-1]=tmp_long;}else{error(5);}"
    PRINT #12, "}}}"

END IF
layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
GOTO finishedline
END IF
END IF




'MID$ statement
IF n >= 1 THEN
    IF firstelement$ = "MID$" THEN
        IF getelement$(a$, 2) <> "(" THEN a$ = "Expected ( after MID$": GOTO errmes
        'calculate 4 parts
        length$ = ""
        part = 1
        i = 3
        a3$ = ""
        stringvariable$ = ""
        start$ = ""
        B = 0
        DO
            IF i > n THEN
                IF part <> 4 OR a3$ = "" THEN a$ = "Expected MID$(...)=...": GOTO errmes
                stringexpression$ = a3$
                EXIT DO
            END IF
            a2$ = getelement$(ca$, i)
            IF a2$ = "(" THEN B = B + 1
            IF a2$ = ")" THEN B = B - 1
            IF B = -1 THEN
                IF part = 2 THEN
                    IF getelement$(a$, i + 1) <> "=" THEN a$ = "Expected = after )": GOTO errmes
                    start$ = a3$: part = 4: a3$ = "": i = i + 1: GOTO midgotpart
                END IF
                IF part = 3 THEN
                    IF getelement$(a$, i + 1) <> "=" THEN a$ = "Expected = after )": GOTO errmes
                    IF a3$ = "" THEN a$ = "Omit , before ) if omitting length in MID$ statement": GOTO errmes
                    length$ = a3$: part = 4: a3$ = "": i = i + 1: GOTO midgotpart
                END IF
            END IF
            IF a2$ = "," AND B = 0 THEN
                IF part = 1 THEN stringvariable$ = a3$: part = 2: a3$ = "": GOTO midgotpart
                IF part = 2 THEN start$ = a3$: part = 3: a3$ = "": GOTO midgotpart
            END IF
            IF LEN(a3$) THEN a3$ = a3$ + sp + a2$ ELSE a3$ = a2$
            midgotpart:
            i = i + 1
        LOOP
        IF stringvariable$ = "" THEN a$ = "Syntax error - first parameter must be a string variable/array-element": GOTO errmes
        IF start$ = "" THEN a$ = "Syntax error - second parameter not optional": GOTO errmes
        'check if it is a valid source string
        stringvariable$ = fixoperationorder$(stringvariable$)
        IF Error_Happened THEN GOTO errmes
        l$ = SCase$("Mid$") + sp2 + "(" + sp2 + tlayout$
        e$ = evaluate(stringvariable$, sourcetyp)
        IF Error_Happened THEN GOTO errmes
        IF (sourcetyp AND ISREFERENCE) = 0 OR (sourcetyp AND ISSTRING) = 0 THEN a$ = "MID$ expects a string variable/array-element as its first argument": GOTO errmes
        stringvariable$ = evaluatetotyp(stringvariable$, ISSTRING)
        IF Error_Happened THEN GOTO errmes

        start$ = fixoperationorder$(start$)
        IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + "," + sp + tlayout$
        start$ = evaluatetotyp((start$), 32&)

        stringexpression$ = fixoperationorder$(stringexpression$)
        IF Error_Happened THEN GOTO errmes
        l2$ = tlayout$
        stringexpression$ = evaluatetotyp(stringexpression$, ISSTRING)
        IF Error_Happened THEN GOTO errmes

        IF LEN(length$) THEN
            length$ = fixoperationorder$(length$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp2 + "," + sp + tlayout$
            length$ = evaluatetotyp(length$, 32&)
            IF Error_Happened THEN GOTO errmes
            PRINT #12, "sub_mid(" + stringvariable$ + "," + start$ + "," + length$ + "," + stringexpression$ + ",1);"
        ELSE
            PRINT #12, "sub_mid(" + stringvariable$ + "," + start$ + ",0," + stringexpression$ + ",0);"
        END IF

        l$ = l$ + sp2 + ")" + sp + "=" + sp + l2$
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline
    END IF
END IF


IF n >= 2 THEN
    IF firstelement$ = "ERASE" THEN
        i = 2
        l$ = SCase$("Erase")
        erasenextarray:
        var$ = getelement$(ca$, i)
        x$ = var$: ls$ = removesymbol(x$)
        IF Error_Happened THEN GOTO errmes

        IF FindArray(var$) THEN
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp + RTRIM$(id.cn) + ls$
            'erase the array
            clearerase:
            n$ = RTRIM$(id.callname)
            bytesperelement$ = str2((id.arraytype AND 511) \ 8)
            IF id.arraytype AND ISSTRING THEN bytesperelement$ = str2(id.tsize)
            IF id.arraytype AND ISOFFSETINBITS THEN bytesperelement$ = str2((id.arraytype AND 511)) + "/8+1"
            IF id.arraytype AND ISUDT THEN
                bytesperelement$ = str2(udtxsize(id.arraytype AND 511) \ 8)
            END IF
            PRINT #12, "if (" + n$ + "[2]&1){" 'array is defined
            PRINT #12, "if (" + n$ + "[2]&2){" 'array is static
            IF (id.arraytype AND ISSTRING) <> 0 AND (id.arraytype AND ISFIXEDLENGTH) = 0 THEN
                PRINT #12, "tmp_long=";
                FOR i2 = 1 TO ABS(id.arrayelements)
                    IF i2 <> 1 THEN PRINT #12, "*";
                    PRINT #12, n$ + "[" + str2(i2 * 4 - 4 + 5) + "]";
                NEXT
                PRINT #12, ";"
                PRINT #12, "while(tmp_long--){"
                PRINT #12, "((qbs*)(((uint64*)(" + n$ + "[0]))[tmp_long]))->len=0;"
                PRINT #12, "}"
            ELSE
                'numeric
                'clear array
                PRINT #12, "memset((void*)(" + n$ + "[0]),0,";
                FOR i2 = 1 TO ABS(id.arrayelements)
                    IF i2 <> 1 THEN PRINT #12, "*";
                    PRINT #12, n$ + "[" + str2(i2 * 4 - 4 + 5) + "]";
                NEXT
                PRINT #12, "*" + bytesperelement$ + ");"
            END IF
            PRINT #12, "}else{" 'array is dynamic
            '1. free memory & any allocated strings
            IF (id.arraytype AND ISSTRING) <> 0 AND (id.arraytype AND ISFIXEDLENGTH) = 0 THEN
                'free strings
                PRINT #12, "tmp_long=";
                FOR i2 = 1 TO ABS(id.arrayelements)
                    IF i2 <> 1 THEN PRINT #12, "*";
                    PRINT #12, n$ + "[" + str2(i2 * 4 - 4 + 5) + "]";
                NEXT
                PRINT #12, ";"
                PRINT #12, "while(tmp_long--){"
                PRINT #12, "qbs_free((qbs*)(((uint64*)(" + n$ + "[0]))[tmp_long]));"
                PRINT #12, "}"
                'free memory
                PRINT #12, "free((void*)(" + n$ + "[0]));"
            ELSE
                'free memory
                PRINT #12, "if (" + n$ + "[2]&4){" 'cmem array
                PRINT #12, "cmem_dynamic_free((uint8*)(" + n$ + "[0]));"
                PRINT #12, "}else{" 'non-cmem array
                PRINT #12, "free((void*)(" + n$ + "[0]));"
                PRINT #12, "}"
            END IF
            '2. set array (and its elements) as undefined
            PRINT #12, n$ + "[2]^=1;" 'remove defined flag, keeping other flags (such as cmem)
            'set dimensions as undefined
            FOR i2 = 1 TO ABS(id.arrayelements)
                B = i2 * 4
                PRINT #12, n$ + "[" + str2(B) + "]=2147483647;" 'base
                PRINT #12, n$ + "[" + str2(B + 1) + "]=0;" 'num. index
                PRINT #12, n$ + "[" + str2(B + 2) + "]=0;" 'multiplier
            NEXT
            IF (id.arraytype AND ISSTRING) <> 0 AND (id.arraytype AND ISFIXEDLENGTH) = 0 THEN
                PRINT #12, n$ + "[0]=(ptrszint)&nothingstring;"
            ELSE
                PRINT #12, n$ + "[0]=(ptrszint)nothingvalue;"
            END IF
            PRINT #12, "}" 'static/dynamic
            PRINT #12, "}" 'array is defined
            IF clearerasereturn = 1 THEN clearerasereturn = 0: GOTO clearerasereturned
            GOTO erasedarray
        END IF
        IF Error_Happened THEN GOTO errmes
        a$ = "Undefined array passed to ERASE": GOTO errmes

        erasedarray:
        IF i < n THEN
            i = i + 1: n$ = getelement$(a$, i): IF n$ <> "," THEN a$ = "Expected ,": GOTO errmes
            l$ = l$ + sp2 + ","
            i = i + 1: IF i > n THEN a$ = "Expected , ...": GOTO errmes
            GOTO erasenextarray
        END IF

        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline
    END IF
END IF


'DIM/REDIM/STATIC
IF n >= 2 THEN
    dimoption = 0: redimoption = 0: commonoption = 0
    IF firstelement$ = "DIM" THEN l$ = SCase$("Dim"): dimoption = 1
    IF firstelement$ = "REDIM" THEN
        l$ = SCase$("ReDim")
        dimoption = 2: redimoption = 1
        IF secondelement$ = "_PRESERVE" OR (secondelement$ = "PRESERVE" AND qbnexprefix_set = 1) THEN
            redimoption = 2
            IF secondelement$ = "_PRESERVE" THEN
                l$ = l$ + sp + SCase$("_Preserve")
            ELSE
                l$ = l$ + sp + SCase$("Preserve")
            END IF
            IF n = 2 THEN a$ = "Expected REDIM " + qbnexprefix$ + "PRESERVE ...": GOTO errmes
        END IF
    END IF
    IF firstelement$ = "STATIC" THEN l$ = SCase$("Static"): dimoption = 3
    IF firstelement$ = "COMMON" THEN l$ = SCase$("Common"): dimoption = 1: commonoption = 1
    IF dimoption THEN

        IF dimoption = 3 AND subfuncn = 0 THEN a$ = "STATIC must be used within a SUB/FUNCTION": GOTO errmes
        IF commonoption = 1 AND subfuncn <> 0 THEN a$ = "COMMON cannot be used within a SUB/FUNCTION": GOTO errmes

        i = 2
        IF redimoption = 2 THEN i = 3

        IF dimoption <> 3 THEN 'shared cannot be static
        a2$ = getelement(a$, i)
        IF a2$ = "SHARED" THEN
            IF subfuncn <> 0 THEN a$ = "DIM/REDIM SHARED invalid within a SUB/FUNCTION": GOTO errmes
            dimshared = 1
            i = i + 1
            l$ = l$ + sp + SCase$("Shared")
        END IF
    END IF

    IF dimoption = 3 THEN dimstatic = 1: AllowLocalName = 1

    'look for new dim syntax: DIM AS variabletype var1, var2, etc....
    e$ = getelement$(a$, i)
    IF e$ <> "AS" THEN
        'no "AS", so this is the traditional dim syntax
        dimnext:
        newDimSyntax = 0
        notype = 0
        listarray = 0


        'old chain code
        'chaincommonarray=0

        varname$ = getelement(ca$, i): i = i + 1
        IF varname$ = "" THEN a$ = "Expected " + firstelement$ + " variable-name or " + firstelement$ + " AS type variable-list": GOTO errmes

        'get the next element
        IF i >= n + 1 THEN e$ = "" ELSE e$ = getelement(a$, i): i = i + 1

        'check if next element is a ( to create an array
        elements$ = ""

        IF e$ = "(" THEN
            B = 1
            FOR i = i TO n
                e$ = getelement(ca$, i)
                IF e$ = "(" THEN B = B + 1
                IF e$ = ")" THEN B = B - 1
                IF B = 0 THEN EXIT FOR
                IF LEN(elements$) THEN elements$ = elements$ + sp + e$ ELSE elements$ = e$
            NEXT
            IF B <> 0 THEN a$ = "Expected )": GOTO errmes
            i = i + 1 'set i to point to the next element

            IF commonoption THEN elements$ = "?"


            IF Debug THEN PRINT #9, "DIM2:array:elements$:[" + elements$ + "]"

            'arrayname() means list array to it will automatically be static when it is formally dimensioned later
            'note: listed arrays are always created in dynamic memory, but their contents are not erased
            '      this differs from static arrays from SUB...STATIC and the unique QBNex method -> STATIC arrayname(100)
            IF dimoption = 3 THEN 'STATIC used
            IF LEN(elements$) = 0 THEN 'nothing between brackets
            listarray = 1 'add to static list
        END IF
    END IF

    'last element was ")"
    'get next element
    IF i >= n + 1 THEN e$ = "" ELSE e$ = getelement(a$, i): i = i + 1
END IF 'e$="("
d$ = e$

dimmethod = 0

appendname$ = "" 'the symbol to append to name returned by dim2
appendtype$ = "" 'eg. sp+AS+spINTEGER
dim2typepassback$ = ""

'does varname have an appended symbol?
s$ = removesymbol$(varname$)
IF Error_Happened THEN GOTO errmes
IF validname(varname$) = 0 THEN a$ = "Invalid variable name": GOTO errmes

IF s$ <> "" THEN
    typ$ = s$
    dimmethod = 1
    appendname$ = typ$
    GOTO dimgottyp
END IF

IF d$ = "AS" THEN
    appendtype$ = sp + SCase$("As")
    typ$ = ""
    FOR i = i TO n
        d$ = getelement(a$, i)
        IF d$ = "," THEN i = i + 1: EXIT FOR
        typ$ = typ$ + d$ + " "
        appendtype$ = appendtype$ + sp + d$
        d$ = ""
    NEXT
    appendtype$ = SCase2$(appendtype$) 'capitalise default types (udt override this later if necessary)
    typ$ = RTRIM$(typ$)
    GOTO dimgottyp
END IF

'auto-define type based on name
notype = 1
IF LEFT$(varname$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(varname$)) - 64
typ$ = defineaz(v)
dimmethod = 1
GOTO dimgottyp

dimgottyp:
IF d$ <> "" AND d$ <> "," THEN a$ = "DIM: Expected ,": GOTO errmes

'In QBASIC, if no type info is given it can refer to an expeicit/formally defined array
IF notype <> 0 AND dimoption <> 3 AND dimoption <> 1 THEN 'not DIM or STATIC which only create new content
IF LEN(elements$) THEN 'an array
IF FindArray(varname$) THEN
    IF LEN(RTRIM$(id.mayhave)) THEN 'explict/formally defined
    typ$ = id2fulltypename$ 'adopt type
    dimmethod = 0 'set as formally defined
END IF
END IF
END IF
END IF

NormalDimBlock:
IF dimoption = 3 AND LEN(elements$) THEN 'eg. STATIC a(100)
'does a conflicting array exist? (use findarray) if so again this should lead to duplicate definition
typ2$ = symbol2fulltypename$(typ$)
t = typname2typ(typ2$): ts = typname2typsize
'try name without any extension
IF FindArray(varname$) THEN 'name without any symbol
IF id.insubfuncn = subfuncn THEN 'global cannot conflict with static
IF LEN(RTRIM$(id.musthave)) THEN
    'if types match then fail
    IF (id.arraytype AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) = (t AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) THEN
        IF ts = id.tsize THEN
            a$ = "Name already in use (" + varname$ + ")": GOTO errmes
        END IF
    END IF
ELSE
    IF dimmethod = 0 THEN
        a$ = "Name already in use (" + varname$ + ")": GOTO errmes 'explicit over explicit
    ELSE
        'if types match then fail
        IF (id.arraytype AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) = (t AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) THEN
            IF ts = id.tsize THEN
                a$ = "Name already in use (" + varname$ + ")": GOTO errmes
            END IF
        END IF
    END IF
END IF
END IF
END IF
'add extension (if possible)
IF (t AND ISUDT) = 0 THEN
    s2$ = type2symbol$(typ2$)
    IF Error_Happened THEN GOTO errmes
    IF FindArray(varname$ + s2$) THEN
        IF id.insubfuncn = subfuncn THEN 'global cannot conflict with static
        IF LEN(RTRIM$(id.musthave)) THEN
            'if types match then fail
            IF (id.arraytype AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) = (t AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) THEN
                IF ts = id.tsize THEN
                    a$ = "Name already in use (" + varname$ + s2$ + ")": GOTO errmes
                END IF
            END IF
        ELSE
            IF dimmethod = 0 THEN
                a$ = "Name already in use (" + varname$ + s2$ + ")": GOTO errmes 'explicit over explicit
            ELSE
                'if types match then fail
                IF (id.arraytype AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) = (t AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) THEN
                    IF ts = id.tsize THEN
                        a$ = "Name already in use (" + varname$ + s2$ + ")": GOTO errmes
                    END IF
                END IF
            END IF
        END IF
    END IF
END IF
END IF 'not a UDT
END IF

IF listarray THEN 'eg. STATIC a()
'note: list is cleared by END SUB/FUNCTION

'is a conflicting array already listed? if so this should cause a duplicate definition error
'check for conflict within list:
xi = 1
FOR x = 1 TO staticarraylistn
    varname2$ = getelement$(staticarraylist, xi): xi = xi + 1
    typ2$ = getelement$(staticarraylist, xi): xi = xi + 1
    dimmethod2 = VAL(getelement$(staticarraylist, xi)): xi = xi + 1
    'check if they are similar
    IF UCASE$(varname$) = UCASE$(varname2$) THEN
        IF dimmethod2 = 1 THEN
            'old using symbol
            IF symbol2fulltypename$(typ$) = typ2$ THEN a$ = "Name already in use (" + varname$ + ")": GOTO errmes
        ELSE
            'old using AS
            IF dimmethod = 0 THEN
                a$ = "Name already in use (" + varname$ + ")": GOTO errmes
            ELSE
                IF symbol2fulltypename$(typ$) = typ2$ THEN a$ = "Name already in use (" + varname$ + ")": GOTO errmes
            END IF
        END IF
    END IF
NEXT

'does a conflicting array exist? (use findarray) if so again this should lead to duplicate definition
typ2$ = symbol2fulltypename$(typ$)
t = typname2typ(typ2$): ts = typname2typsize
'try name without any extension
IF FindArray(varname$) THEN 'name without any symbol
IF id.insubfuncn = subfuncn THEN 'global cannot conflict with static
IF LEN(RTRIM$(id.musthave)) THEN
    'if types match then fail
    IF (id.arraytype AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) = (t AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) THEN
        IF ts = id.tsize THEN
            a$ = "Name already in use (" + varname$ + ")": GOTO errmes
        END IF
    END IF
ELSE
    IF dimmethod = 0 THEN
        a$ = "Name already in use (" + varname$ + ")": GOTO errmes 'explicit over explicit
    ELSE
        'if types match then fail
        IF (id.arraytype AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) = (t AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) THEN
            IF ts = id.tsize THEN
                a$ = "Name already in use (" + varname$ + ")": GOTO errmes
            END IF
        END IF
    END IF
END IF
END IF
END IF
'add extension (if possible)
IF (t AND ISUDT) = 0 THEN
    s2$ = type2symbol$(typ2$)
    IF Error_Happened THEN GOTO errmes
    IF FindArray(varname$ + s2$) THEN
        IF id.insubfuncn = subfuncn THEN 'global cannot conflict with static
        IF LEN(RTRIM$(id.musthave)) THEN
            'if types match then fail
            IF (id.arraytype AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) = (t AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) THEN
                IF ts = id.tsize THEN
                    a$ = "Name already in use (" + varname$ + s2$ + ")": GOTO errmes
                END IF
            END IF
        ELSE
            IF dimmethod = 0 THEN
                a$ = "Name already in use (" + varname$ + s2$ + ")": GOTO errmes 'explicit over explicit
            ELSE
                'if types match then fail
                IF (id.arraytype AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) = (t AND (ISFLOAT + ISUDT + 511 + ISUNSIGNED + ISSTRING + ISFIXEDLENGTH)) THEN
                    IF ts = id.tsize THEN
                        a$ = "Name already in use (" + varname$ + s2$ + ")": GOTO errmes
                    END IF
                END IF
            END IF
        END IF
    END IF
END IF
END IF 'not a UDT

'note: static list arrays cannot be created until they are formally [or informally] (RE)DIM'd later
IF LEN(staticarraylist) THEN staticarraylist = staticarraylist + sp
staticarraylist = staticarraylist + varname$ + sp + symbol2fulltypename$(typ$) + sp + str2(dimmethod)
IF Error_Happened THEN GOTO errmes
staticarraylistn = staticarraylistn + 1
l$ = l$ + sp + varname$ + appendname$ + sp2 + "(" + sp2 + ")" + appendtype$
'note: none of the following code is run, dim2 call is also skipped

ELSE

    olddimstatic = dimstatic

    'check if varname is on the static list
    IF LEN(elements$) THEN 'it's an array
    IF subfuncn THEN 'it's in a sub/function
    xi = 1
    FOR x = 1 TO staticarraylistn
        varname2$ = getelement$(staticarraylist, xi): xi = xi + 1
        typ2$ = getelement$(staticarraylist, xi): xi = xi + 1
        dimmethod2 = VAL(getelement$(staticarraylist, xi)): xi = xi + 1
        'check if they are similar
        IF UCASE$(varname$) = UCASE$(varname2$) THEN
            IF symbol2fulltypename$(typ$) = typ2$ THEN
                IF Error_Happened THEN GOTO errmes
                IF dimmethod = dimmethod2 THEN
                    'match found!
                    varname$ = varname2$
                    dimstatic = 3
                    IF dimoption = 3 THEN a$ = "Array already listed as STATIC": GOTO errmes
                END IF
            END IF 'typ
        END IF 'varname
    NEXT
END IF
END IF

'COMMON exception
'note: COMMON alone does not imply SHARED
'      if either(or both) COMMON & later DIM have SHARED, variable becomes shared
IF commonoption THEN
    IF LEN(elements$) THEN

        'add array to list
        IF LEN(commonarraylist) THEN commonarraylist = commonarraylist + sp
        'note: dimmethod distinguishes between a%(...) vs a(...) AS INTEGER
        commonarraylist = commonarraylist + varname$ + sp + symbol2fulltypename$(typ$) + sp + str2(dimmethod) + sp + str2(dimshared)
        IF Error_Happened THEN GOTO errmes
        commonarraylistn = commonarraylistn + 1
        IF Debug THEN PRINT #9, "common listed:" + varname$ + sp + symbol2fulltypename$(typ$) + sp + str2(dimmethod) + sp + str2(dimshared)
        IF Error_Happened THEN GOTO errmes

        x = 0

        v$ = varname$
        IF dimmethod = 1 THEN v$ = v$ + typ$
        try = findid(v$)
        IF Error_Happened THEN GOTO errmes
        DO WHILE try
            IF id.arraytype THEN

                t = typname2typ(typ$)
                IF Error_Happened THEN GOTO errmes
                s = typname2typsize
                match = 1
                'note: dimmethod 2 is already matched
                IF dimmethod = 0 THEN
                    t2 = id.arraytype
                    s2 = id.tsize
                    IF (t AND ISFLOAT) <> (t2 AND ISFLOAT) THEN match = 0
                    IF (t AND ISUNSIGNED) <> (t2 AND ISUNSIGNED) THEN match = 0
                    IF (t AND ISSTRING) <> (t2 AND ISSTRING) THEN match = 0
                    IF (t AND ISFIXEDLENGTH) <> (t2 AND ISFIXEDLENGTH) THEN match = 0
                    IF (t AND ISOFFSETINBITS) <> (t2 AND ISOFFSETINBITS) THEN match = 0
                    IF (t AND ISUDT) <> (t2 AND ISUDT) THEN match = 0
                    IF (t AND 511) <> (t2 AND 511) THEN match = 0
                    IF s <> s2 THEN match = 0
                    'check for implicit/explicit declaration match
                    oldmethod = 0: IF LEN(RTRIM$(id.musthave)) THEN oldmethod = 1
                    IF oldmethod <> dimmethod THEN match = 0
                END IF

                IF match THEN
                    x = currentid
                    IF dimshared THEN ids(x).share = 1 'share if necessary
                    tlayout$ = RTRIM$(id.cn) + sp + "(" + sp2 + ")"

                    IF dimmethod = 0 THEN
                        IF t AND ISUDT THEN
                            dim2typepassback$ = RTRIM$(udtxcname(t AND 511))
                            IF UCASE$(typ$) = "MEM" AND qbnexprefix_set = 1 AND RTRIM$(udtxcname(t AND 511)) = "_MEM" THEN
                                dim2typepassback$ = MID$(RTRIM$(udtxcname(t AND 511)), 2)
                            END IF
                        ELSE
                            dim2typepassback$ = typ$
                            DO WHILE INSTR(dim2typepassback$, " ")
                                ASC(dim2typepassback$, INSTR(dim2typepassback$, " ")) = ASC(sp)
                            LOOP
                            dim2typepassback$ = SCase2$(dim2typepassback$)
                        END IF
                    END IF 'method 0

                    EXIT DO
                END IF 'match

            END IF 'arraytype
            IF try = 2 THEN findanotherid = 1: try = findid(v$) ELSE try = 0
            IF Error_Happened THEN GOTO errmes
        LOOP

        IF x = 0 THEN x = idn + 1

        'note: the following code only adds include directives, everything else is defered
        OPEN tmpdir$ + "chain.txt" FOR APPEND AS #22
        'include directive
        PRINT #22, "#include " + CHR$(34) + "chain" + str2$(x) + ".txt" + CHR$(34)
        CLOSE #22
        'create/clear include file
        OPEN tmpdir$ + "chain" + str2$(x) + ".txt" FOR OUTPUT AS #22: CLOSE #22

        OPEN tmpdir$ + "inpchain.txt" FOR APPEND AS #22
        'include directive
        PRINT #22, "#include " + CHR$(34) + "inpchain" + str2$(x) + ".txt" + CHR$(34)
        CLOSE #22
        'create/clear include file
        OPEN tmpdir$ + "inpchain" + str2$(x) + ".txt" FOR OUTPUT AS #22: CLOSE #22

        'note: elements$="?"
        IF x <> idn + 1 THEN GOTO skipdim 'array already exists
        GOTO dimcommonarray

    END IF
END IF

'is varname on common list?
'******
IF LEN(elements$) THEN 'it's an array
IF subfuncn = 0 THEN 'not in a sub/function

IF Debug THEN PRINT #9, "common checking:" + varname$

xi = 1
FOR x = 1 TO commonarraylistn
    varname2$ = getelement$(commonarraylist, xi): xi = xi + 1
    typ2$ = getelement$(commonarraylist, xi): xi = xi + 1
    dimmethod2 = VAL(getelement$(commonarraylist, xi)): xi = xi + 1
    dimshared2 = VAL(getelement$(commonarraylist, xi)): xi = xi + 1
    IF Debug THEN PRINT #9, "common checking against:" + varname2$ + sp + typ2$ + sp + str2(dimmethod2) + sp + str2(dimshared2)
    'check if they are similar
    IF varname$ = varname2$ THEN
        IF symbol2fulltypename$(typ$) = typ2$ THEN
            IF Error_Happened THEN GOTO errmes
            IF dimmethod = dimmethod2 THEN

                'match found!
                'enforce shared status (if necessary)
                IF dimshared2 THEN dimshared = dimshared OR 2 'temp force SHARED

                'old chain code
                'chaincommonarray=x

            END IF 'method
        END IF 'typ
    END IF 'varname
NEXT
END IF
END IF

dimcommonarray:
retval = dim2(varname$, typ$, dimmethod, elements$)
IF Error_Happened THEN GOTO errmes
skipdim:
IF dimshared >= 2 THEN dimshared = dimshared - 2

'non-array COMMON variable
IF commonoption <> 0 AND LEN(elements$) = 0 THEN

    'CHAIN.TXT (save)

    use_global_byte_elements = 1

    'switch output from main.txt to chain.txt
    CLOSE #12
    OPEN tmpdir$ + "chain.txt" FOR APPEND AS #12
    l2$ = tlayout$

    PRINT #12, "int32val=1;" 'simple variable
    PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"

    t = id.t
    bits = t AND 511
    IF t AND ISUDT THEN bits = udtxsize(t AND 511)
    IF t AND ISSTRING THEN
        IF t AND ISFIXEDLENGTH THEN
            bits = id.tsize * 8
        ELSE
            PRINT #12, "int64val=__STRING_" + RTRIM$(id.n) + "->len*8;"
            bits = 0
        END IF
    END IF

    IF bits THEN
        PRINT #12, "int64val=" + str2$(bits) + ";" 'size in bits
    END IF
    PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);"

    'put the variable
    e$ = RTRIM$(id.n)

    IF (t AND ISUDT) = 0 THEN
        IF t AND ISFIXEDLENGTH THEN
            e$ = e$ + "$" + str2$(id.tsize)
        ELSE
            e$ = e$ + typevalue2symbol$(t)
            IF Error_Happened THEN GOTO errmes
        END IF
    END IF
    e$ = evaluatetotyp(fixoperationorder$(e$), -4)
    IF Error_Happened THEN GOTO errmes

    PRINT #12, "sub_put(FF,NULL," + e$ + ",0);"

    tlayout$ = l2$
    'revert output to main.txt
    CLOSE #12
    OPEN tmpdir$ + "main.txt" FOR APPEND AS #12


    'INPCHAIN.TXT (load)

    'switch output from main.txt to chain.txt
    CLOSE #12
    OPEN tmpdir$ + "inpchain.txt" FOR APPEND AS #12
    l2$ = tlayout$


    PRINT #12, "if (int32val==1){"
    'get the size in bits
    PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);"
    '***assume correct size***

    e$ = RTRIM$(id.n)
    t = id.t
    IF (t AND ISUDT) = 0 THEN
        IF t AND ISFIXEDLENGTH THEN
            e$ = e$ + "$" + str2$(id.tsize)
        ELSE
            e$ = e$ + typevalue2symbol$(t)
            IF Error_Happened THEN GOTO errmes
        END IF
    END IF

    IF t AND ISSTRING THEN
        IF (t AND ISFIXEDLENGTH) = 0 THEN
            PRINT #12, "tqbs=qbs_new(int64val>>3,1);"
            PRINT #12, "qbs_set(__STRING_" + RTRIM$(id.n) + ",tqbs);"
            'now that the string is the correct size, the following GET command will work correctly...
        END IF
    END IF

    e$ = evaluatetotyp(fixoperationorder$(e$), -4)
    IF Error_Happened THEN GOTO errmes
    PRINT #12, "sub_get(FF,NULL," + e$ + ",0);"

    PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);" 'get next command
    PRINT #12, "}"

    tlayout$ = l2$
    'revert output to main.txt
    CLOSE #12
    OPEN tmpdir$ + "main.txt" FOR APPEND AS #12

    use_global_byte_elements = 0

END IF

commonarraylisted:

IF LEN(appendtype$) > 0 AND newDimSyntax = -1 THEN
    IF LEN(dim2typepassback$) THEN appendtype$ = sp + SCase$("As") + sp + dim2typepassback$
    IF newDimSyntaxTypePassBack = 0 THEN
        newDimSyntaxTypePassBack = -1
        l$ = l$ + appendtype$
    END IF
END IF

n2 = numelements(tlayout$)
l$ = l$ + sp + getelement$(tlayout$, 1) + appendname$
IF n2 > 1 THEN
    l$ = l$ + sp2 + getelements$(tlayout$, 2, n2)
END IF

IF LEN(appendtype$) > 0 AND newDimSyntax = 0 THEN
    IF LEN(dim2typepassback$) THEN appendtype$ = sp + SCase$("As") + sp + dim2typepassback$
    l$ = l$ + appendtype$
END IF

'modify first element name to include symbol

dimstatic = olddimstatic

END IF 'listarray=0

IF newDimSyntax THEN RETURN

IF d$ = "," THEN l$ = l$ + sp2 + ",": GOTO dimnext

dimoption = 0
dimshared = 0
redimoption = 0
IF dimstatic = 1 THEN dimstatic = 0
AllowLocalName = 0

layoutdone = 1
IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$

GOTO finishedline
ELSE
    'yes, this is the new dim syntax.
    i = i + 1 'skip "AS"
    newDimSyntaxTypePassBack = 0

    'estabilish the data type:
    appendname$ = ""
    appendtype$ = sp + SCase$("As")
    typ$ = ""
    varname$ = ""
    previousElement$ = ""
    FOR i = i TO n
        d$ = getelement(a$, i)
        IF d$ = "," OR d$ = "(" THEN EXIT FOR
        varname$ = getelement(ca$, i)
        IF LEN(previousElement$) THEN
            typ$ = typ$ + previousElement$ + " "
            appendtype$ = appendtype$ + sp + previousElement$
        END IF
        previousElement$ = d$
        d$ = ""
    NEXT
    appendtype$ = SCase2$(appendtype$) 'capitalise default types (udt override this later if necessary)
    typ$ = RTRIM$(typ$)

    dimnext2:
    notype = 0
    listarray = 0

    IF typ$ = "" OR varname$ = "" THEN a$ = "Expected " + firstelement$ + " AS type variable-list or " + firstelement$ + " variable-name AS type": GOTO errmes

    'get the next element
    IF i >= n + 1 THEN e$ = "" ELSE e$ = getelement(a$, i): i = i + 1

    'check if next element is a ( to create an array
    elements$ = ""

    IF e$ = "(" THEN
        B = 1
        FOR i = i TO n
            e$ = getelement(ca$, i)
            IF e$ = "(" THEN B = B + 1
            IF e$ = ")" THEN B = B - 1
            IF B = 0 THEN EXIT FOR
            IF LEN(elements$) THEN elements$ = elements$ + sp + e$ ELSE elements$ = e$
        NEXT
        IF B <> 0 THEN a$ = "Expected )": GOTO errmes
        i = i + 1 'set i to point to the next element

        IF commonoption THEN elements$ = "?"


        IF Debug THEN PRINT #9, "DIM2:array:elements$:[" + elements$ + "]"

        'arrayname() means list array to it will automatically be static when it is formally dimensioned later
        'note: listed arrays are always created in dynamic memory, but their contents are not erased
        '      this differs from static arrays from SUB...STATIC and the unique QBNex method -> STATIC arrayname(100)
        IF dimoption = 3 THEN 'STATIC used
        IF LEN(elements$) = 0 THEN 'nothing between brackets
        listarray = 1 'add to static list
    END IF
END IF

'last element was ")"
'get next element
IF i >= n + 1 THEN e$ = "" ELSE e$ = getelement(a$, i): i = i + 1
END IF 'e$="("
d$ = e$

dimmethod = 0

dim2typepassback$ = ""

'does varname have an appended symbol?
s$ = removesymbol$(varname$)
IF Error_Happened THEN GOTO errmes
IF validname(varname$) = 0 THEN a$ = "Invalid variable name": GOTO errmes

IF s$ <> "" THEN
    a$ = "Cannot use type symbol with " + firstelement$ + " AS type variable-list (" + s$ + ")"
    GOTO errmes
END IF

IF d$ <> "" AND d$ <> "," THEN a$ = "DIM: Expected ,": GOTO errmes

newDimSyntax = -1
GOSUB NormalDimBlock
newDimSyntax = 0

IF d$ = "," THEN
    l$ = l$ + sp2 + ","
    varname$ = getelement(ca$, i): i = i + 1
    GOTO dimnext2
END IF

dimoption = 0
dimshared = 0
redimoption = 0
IF dimstatic = 1 THEN dimstatic = 0
AllowLocalName = 0

layoutdone = 1
IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$

GOTO finishedline
END IF
END IF
END IF











'THEN [GOTO] linenumber?
IF THENGOTO = 1 THEN
    IF n = 1 THEN
        l$ = ""
        a = ASC(LEFT$(firstelement$, 1))
        IF a = 46 OR (a >= 48 AND a <= 57) THEN a2$ = ca$: GOTO THENGOTO
    END IF
END IF

'goto
IF n = 2 THEN
    IF getelement$(a$, 1) = "GOTO" THEN
        l$ = SCase$("GoTo")
        a2$ = getelement$(ca$, 2)
        THENGOTO:
        IF validlabel(a2$) = 0 THEN a$ = "Invalid label!": GOTO errmes

        v = HashFind(a2$, HASHFLAG_LABEL, ignore, r)
        x = 1
        labchk2:
        IF v THEN
            s = Labels(r).Scope
            IF s = subfuncn OR s = -1 THEN 'same scope?
            IF s = -1 THEN Labels(r).Scope = subfuncn 'acquire scope
            x = 0 'already defined
            tlayout$ = RTRIM$(Labels(r).cn)
        ELSE
            IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk2
        END IF
    END IF
    IF x THEN
        'does not exist
        nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
        Labels(nLabels) = Empty_Label
        HashAdd a2$, HASHFLAG_LABEL, nLabels
        r = nLabels
        Labels(r).State = 0
        Labels(r).cn = a2$
        Labels(r).Scope = subfuncn
        Labels(r).Error_Line = linenumber
    END IF 'x

    IF LEN(l$) THEN l$ = l$ + sp + tlayout$ ELSE l$ = tlayout$
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    PRINT #12, "goto LABEL_" + a2$ + ";"
    GOTO finishedline
END IF
END IF

IF n = 1 THEN
    IF firstelement$ = "_CONTINUE" OR (firstelement$ = "CONTINUE" AND qbnexprefix_set = 1) THEN
        IF firstelement$ = "_CONTINUE" THEN l$ = SCase$("_Continue") ELSE l$ = SCase$("Continue")
        'scan backwards until previous control level reached
        FOR i = controllevel TO 1 STEP -1
            t = controltype(i)
            IF t = 2 THEN 'for...next
            PRINT #12, "goto fornext_continue_" + str2$(controlid(i)) + ";"
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        ELSEIF t = 3 OR t = 4 THEN 'do...loop
            PRINT #12, "goto dl_continue_" + str2$(controlid(i)) + ";"
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        ELSEIF t = 5 THEN 'while...wend
            PRINT #12, "goto ww_continue_" + str2$(controlid(i)) + ";"
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        END IF
    NEXT
    a$ = qbnexprefix$ + "CONTINUE outside DO..LOOP/FOR..NEXT/WHILE..WEND block": GOTO errmes
END IF
END IF

IF firstelement$ = "CHAIN" THEN
    IF vWatchOn THEN
        addWarning linenumber, inclevel, inclinenumber(inclevel), incname$(inclevel), "Feature incompatible with $Debug mode", "CHAIN"
    END IF
END IF

IF firstelement$ = "RUN" THEN 'RUN
IF vWatchOn THEN
    addWarning linenumber, inclevel, inclinenumber(inclevel), incname$(inclevel), "Feature incompatible with $Debug mode", "RUN"
END IF
l$ = SCase$("Run")
IF n = 1 THEN
    'no parameters
    PRINT #12, "sub_run_init();" 'note: called first to free up screen-locked image handles
    PRINT #12, "sub_clear(NULL,NULL,NULL,NULL);" 'use functionality of CLEAR
    IF LEN(subfunc$) THEN
        PRINT #12, "QBMAIN(NULL);"
    ELSE
        PRINT #12, "goto S_0;"
    END IF
ELSE
    'parameter passed
    e$ = getelements$(ca$, 2, n)
    e$ = fixoperationorder$(e$)
    IF Error_Happened THEN GOTO errmes
    l2$ = tlayout$
    ignore$ = evaluate(e$, typ)
    IF Error_Happened THEN GOTO errmes
    IF n = 2 AND ((typ AND ISSTRING) = 0) THEN
        'assume it's a label or line number
        lbl$ = getelement$(ca$, 2)
        IF validlabel(lbl$) = 0 THEN a$ = "Invalid label!": GOTO errmes 'invalid label

        v = HashFind(lbl$, HASHFLAG_LABEL, ignore, r)
        x = 1
        labchk501:
        IF v THEN
            s = Labels(r).Scope
            IF s = 0 OR s = -1 THEN 'main scope?
            IF s = -1 THEN Labels(r).Scope = 0 'acquire scope
            x = 0 'already defined
            tlayout$ = RTRIM$(Labels(r).cn)
            Labels(r).Scope_Restriction = subfuncn
            Labels(r).Error_Line = linenumber
        ELSE
            IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk501
        END IF
    END IF
    IF x THEN
        'does not exist
        nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
        Labels(nLabels) = Empty_Label
        HashAdd lbl$, HASHFLAG_LABEL, nLabels
        r = nLabels
        Labels(r).State = 0
        Labels(r).cn = lbl$
        Labels(r).Scope = 0
        Labels(r).Error_Line = linenumber
        Labels(r).Scope_Restriction = subfuncn
    END IF 'x

    l$ = l$ + sp + tlayout$
    PRINT #12, "sub_run_init();" 'note: called first to free up screen-locked image handles
    PRINT #12, "sub_clear(NULL,NULL,NULL,NULL);" 'use functionality of CLEAR
    IF LEN(subfunc$) THEN
        PRINT #21, "if (run_from_line==" + str2(nextrunlineindex) + "){run_from_line=0;goto LABEL_" + lbl$ + ";}"
        PRINT #12, "run_from_line=" + str2(nextrunlineindex) + ";"
        nextrunlineindex = nextrunlineindex + 1
        PRINT #12, "QBMAIN(NULL);"
    ELSE
        PRINT #12, "goto LABEL_" + lbl$ + ";"
    END IF
ELSE
    'assume it's a string containing a filename to execute
    e$ = evaluatetotyp(e$, ISSTRING)
    IF Error_Happened THEN GOTO errmes
    PRINT #12, "sub_run(" + e$ + ");"
    l$ = l$ + sp + l2$
END IF 'isstring
END IF 'n=1
layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
GOTO finishedline
END IF 'run





IF firstelement$ = "END" THEN
    l$ = SCase$("End")
    IF n > 1 THEN
        e$ = getelements$(ca$, 2, n)
        e$ = fixoperationorder$(e$): IF Error_Happened THEN GOTO errmes
        l2$ = tlayout$
        e$ = evaluatetotyp(e$, ISINTEGER64): IF Error_Happened THEN GOTO errmes
        inclinenump$ = ""
        IF inclinenumber(inclevel) THEN
            inclinenump$ = "," + str2$(inclinenumber(inclevel))
            thisincname$ = getfilepath$(incname$(inclevel))
            thisincname$ = MID$(incname$(inclevel), LEN(thisincname$) + 1)
            inclinenump$ = inclinenump$ + "," + CHR$(34) + thisincname$ + CHR$(34)
        END IF
        IF vWatchOn AND inclinenumber(inclevel) = 0 THEN temp$ = vWatchErrorCall$ ELSE temp$ = ""
        PRINT #12, "if(qbevent){" + temp$ + "evnt(" + str2$(linenumber) + inclinenump$ + ");}" 'non-resumable error check (cannot exit without handling errors)
        PRINT #12, "exit_code=" + e$ + ";"
        l$ = l$ + sp + l2$
    END IF
    xend
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    GOTO finishedline
END IF

IF firstelement$ = "SYSTEM" THEN
    l$ = SCase$("System")
    IF n > 1 THEN
        e$ = getelements$(ca$, 2, n)
        e$ = fixoperationorder$(e$): IF Error_Happened THEN GOTO errmes
        l2$ = tlayout$
        e$ = evaluatetotyp(e$, ISINTEGER64): IF Error_Happened THEN GOTO errmes
        inclinenump$ = ""
        IF inclinenumber(inclevel) THEN
            inclinenump$ = "," + str2$(inclinenumber(inclevel))
            thisincname$ = getfilepath$(incname$(inclevel))
            thisincname$ = MID$(incname$(inclevel), LEN(thisincname$) + 1)
            inclinenump$ = inclinenump$ + "," + CHR$(34) + thisincname$ + CHR$(34)
        END IF
        IF vWatchOn = 1 AND NoChecks = 0 AND inclinenumber(inclevel) = 0 THEN temp$ = vWatchErrorCall$ ELSE temp$ = ""
        PRINT #12, "if(qbevent){" + temp$ + "evnt(" + str2$(linenumber) + inclinenump$ + ");}" 'non-resumable error check (cannot exit without handling errors)
        PRINT #12, "exit_code=" + e$ + ";"
        l$ = l$ + sp + l2$
    END IF


    IF vWatchOn = 1 THEN
        IF inclinenumber(inclevel) = 0 THEN
            vWatchAddLabel linenumber, 0
        END IF
        PRINT #12, "*__LONG_VWATCH_LINENUMBER= 0; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);"
    END IF
    PRINT #12, "if (sub_gl_called) error(271);"
    PRINT #12, "close_program=1;"
    PRINT #12, "end();"
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    GOTO finishedline
END IF

IF n >= 1 THEN
    IF firstelement$ = "STOP" THEN
        l$ = SCase$("Stop")
        IF n > 1 THEN
            e$ = getelements$(ca$, 2, n)
            e$ = fixoperationorder$(e$)
            IF Error_Happened THEN GOTO errmes
            l$ = SCase$("Stop") + sp + tlayout$
            e$ = evaluatetotyp(e$, 64)
            IF Error_Happened THEN GOTO errmes
            'note: this value is currently ignored but evaluated for checking reasons
        END IF
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        IF vWatchOn = 1 AND NoChecks = 0 AND inclinenumber(inclevel) = 0 THEN
            PRINT #12, "*__LONG_VWATCH_LINENUMBER=-3; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars); if (*__LONG_VWATCH_GOTO>0) goto VWATCH_SETNEXTLINE; if (*__LONG_VWATCH_GOTO<0) goto VWATCH_SKIPLINE;"
            vWatchAddLabel linenumber, 0
        ELSE
            PRINT #12, "close_program=1;"
            PRINT #12, "end();"
        END IF
        GOTO finishedline
    END IF
END IF

IF n = 2 THEN
    IF firstelement$ = "GOSUB" THEN
        xgosub ca$
        IF Error_Happened THEN GOTO errmes
        'note: layout implemented in xgosub
        GOTO finishedline
    END IF
END IF

IF n >= 1 THEN
    IF firstelement$ = "RETURN" THEN
        IF n = 1 THEN
            PRINT #12, "#include " + CHR$(34) + "ret" + str2$(subfuncn) + ".txt" + CHR$(34)
            l$ = SCase$("Return")
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        ELSE
            'label/linenumber follows
            IF subfuncn <> 0 THEN a$ = "RETURN linelabel/linenumber invalid within a SUB/FUNCTION": GOTO errmes
            IF n > 2 THEN a$ = "Expected linelabel/linenumber after RETURN": GOTO errmes
            PRINT #12, "if (!next_return_point) error(3);" 'check return point available
            PRINT #12, "next_return_point--;" 'destroy return point
            a2$ = getelement$(ca$, 2)
            IF validlabel(a2$) = 0 THEN a$ = "Invalid label!": GOTO errmes

            v = HashFind(a2$, HASHFLAG_LABEL, ignore, r)
            x = 1
            labchk505:
            IF v THEN
                s = Labels(r).Scope
                IF s = subfuncn OR s = -1 THEN 'same scope?
                IF s = -1 THEN Labels(r).Scope = subfuncn 'acquire scope
                x = 0 'already defined
                tlayout$ = RTRIM$(Labels(r).cn)
            ELSE
                IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk505
            END IF
        END IF
        IF x THEN
            'does not exist
            nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
            Labels(nLabels) = Empty_Label
            HashAdd a2$, HASHFLAG_LABEL, nLabels
            r = nLabels
            Labels(r).State = 0
            Labels(r).cn = a2$
            Labels(r).Scope = subfuncn
            Labels(r).Error_Line = linenumber
        END IF 'x

        PRINT #12, "goto LABEL_" + a2$ + ";"
        l$ = SCase$("Return") + sp + tlayout$
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline
    END IF
END IF
END IF

IF n >= 1 THEN
    IF firstelement$ = "RESUME" THEN
        l$ = SCase$("Resume")
        IF n = 1 THEN
            resumeprev:


            PRINT #12, "if (!error_handling){error(20);}else{error_retry=1; qbevent=1; error_handling=0; error_err=0; return;}"

            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        END IF
        IF n > 2 THEN a$ = "Too many parameters": GOTO errmes
        s$ = getelement$(ca$, 2)
        IF UCASE$(s$) = "NEXT" THEN


            PRINT #12, "if (!error_handling){error(20);}else{error_handling=0; error_err=0; return;}"

            l$ = l$ + sp + SCase$("Next")
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        END IF
        IF s$ = "0" THEN l$ = l$ + sp + "0": GOTO resumeprev
        IF validlabel(s$) = 0 THEN a$ = "Invalid label passed to RESUME": GOTO errmes

        v = HashFind(s$, HASHFLAG_LABEL, ignore, r)
        x = 1
        labchk506:
        IF v THEN
            s = Labels(r).Scope
            IF s = subfuncn OR s = -1 THEN 'same scope?
            IF s = -1 THEN Labels(r).Scope = subfuncn 'acquire scope
            x = 0 'already defined
            tlayout$ = RTRIM$(Labels(r).cn)
        ELSE
            IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk506
        END IF
    END IF
    IF x THEN
        'does not exist
        nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
        Labels(nLabels) = Empty_Label
        HashAdd s$, HASHFLAG_LABEL, nLabels
        r = nLabels
        Labels(r).State = 0
        Labels(r).cn = s$
        Labels(r).Scope = subfuncn
        Labels(r).Error_Line = linenumber
    END IF 'x

    l$ = l$ + sp + tlayout$
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    PRINT #12, "if (!error_handling){error(20);}else{error_handling=0; error_err=0; goto LABEL_" + s$ + ";}"
    GOTO finishedline
END IF
END IF

IF n = 4 THEN
    IF getelements(a$, 1, 3) = "ON" + sp + "ERROR" + sp + "GOTO" THEN
        l$ = SCase$("On" + sp + "Error" + sp + "GoTo")
        lbl$ = getelement$(ca$, 4)
        IF lbl$ = "0" THEN
            PRINT #12, "error_goto_line=0;"
            l$ = l$ + sp + "0"
            layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
            GOTO finishedline
        END IF
        IF validlabel(lbl$) = 0 THEN a$ = "Invalid label": GOTO errmes

        v = HashFind(lbl$, HASHFLAG_LABEL, ignore, r)
        x = 1
        labchk6:
        IF v THEN
            s = Labels(r).Scope
            IF s = 0 OR s = -1 THEN 'main scope?
            IF s = -1 THEN Labels(r).Scope = 0 'acquire scope
            x = 0 'already defined
            tlayout$ = RTRIM$(Labels(r).cn)
            Labels(r).Scope_Restriction = subfuncn
            Labels(r).Error_Line = linenumber
        ELSE
            IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk6
        END IF
    END IF
    IF x THEN
        'does not exist
        nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
        Labels(nLabels) = Empty_Label
        HashAdd lbl$, HASHFLAG_LABEL, nLabels
        r = nLabels
        Labels(r).State = 0
        Labels(r).cn = lbl$
        Labels(r).Scope = 0
        Labels(r).Error_Line = linenumber
        Labels(r).Scope_Restriction = subfuncn
    END IF 'x


    l$ = l$ + sp + tlayout$
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    errorlabels = errorlabels + 1
    PRINT #12, "error_goto_line=" + str2(errorlabels) + ";"
    PRINT #14, "if (error_goto_line==" + str2(errorlabels) + "){error_handling=1; goto LABEL_" + lbl$ + ";}"
    GOTO finishedline
END IF
END IF

IF n >= 1 THEN
    IF firstelement$ = "RESTORE" THEN
        l$ = SCase$("Restore")
        IF n = 1 THEN
            PRINT #12, "data_offset=0;"
        ELSE
            IF n > 2 THEN a$ = "Syntax error - too many parameters (expected RESTORE label/line number)": GOTO errmes
            lbl$ = getelement$(ca$, 2)
            IF validlabel(lbl$) = 0 THEN a$ = "Invalid label": GOTO errmes

            'rule: a RESTORE label has no scope, therefore, only one instance of that label may exist
            'how: enforced by a post check for duplicates
            v = HashFind(lbl$, HASHFLAG_LABEL, ignore, r)
            x = 1
            IF v THEN 'already defined
            x = 0
            tlayout$ = RTRIM$(Labels(r).cn)
            Labels(r).Data_Referenced = 1 'make sure the data referenced flag is set
            IF Labels(r).Error_Line = 0 THEN Labels(r).Error_Line = linenumber
        END IF
        IF x THEN
            nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
            Labels(nLabels) = Empty_Label
            HashAdd lbl$, HASHFLAG_LABEL, nLabels
            r = nLabels
            Labels(r).State = 0
            Labels(r).cn = lbl$
            Labels(r).Scope = -1 'modifyable scope
            Labels(r).Error_Line = linenumber
            Labels(r).Data_Referenced = 1
        END IF 'x

        l$ = l$ + sp + tlayout$
        PRINT #12, "data_offset=data_at_LABEL_" + lbl$ + ";"
    END IF
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    GOTO finishedline
END IF
END IF



'ON ... GOTO/GOSUB
IF n >= 1 THEN
    IF firstelement$ = "ON" THEN
        xongotogosub a$, ca$, n
        IF Error_Happened THEN GOTO errmes
        GOTO finishedline
    END IF
END IF


'(_MEM) _MEMPUT _MEMGET
IF n >= 1 THEN
    IF firstelement$ = "_MEMGET" OR (firstelement$ = "MEMGET" AND qbnexprefix_set = 1) THEN
        'get expressions
        e$ = ""
        B = 0
        ne = 0
        FOR i2 = 2 TO n
            e2$ = getelement$(ca$, i2)
            IF e2$ = "(" THEN B = B + 1
            IF e2$ = ")" THEN B = B - 1
            IF e2$ = "," AND B = 0 THEN
                ne = ne + 1
                IF ne = 1 THEN blk$ = e$: e$ = ""
                IF ne = 2 THEN offs$ = e$: e$ = ""
                IF ne = 3 THEN a$ = "Syntax error - too many parameters (Expected " + qbnexprefix$ + "MEMGET mem-reference, offset, variable)": GOTO errmes
            ELSE
                IF LEN(e$) = 0 THEN e$ = e2$ ELSE e$ = e$ + sp + e2$
            END IF
        NEXT
        var$ = e$
        IF e$ = "" OR ne <> 2 THEN a$ = "Expected " + qbnexprefix$ + "MEMGET mem-reference, offset, variable": GOTO errmes

        IF firstelement$ = "_MEMGET" THEN l$ = SCase$("_MemGet") + sp ELSE l$ = SCase$("MemGet") + sp

        e$ = fixoperationorder$(blk$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + tlayout$

        test$ = evaluate(e$, typ): IF Error_Happened THEN GOTO errmes
        IF (typ AND ISUDT) = 0 OR (typ AND 511) <> 1 THEN a$ = "Expected " + qbnexprefix$ + "MEM type": GOTO errmes
        blkoffs$ = evaluatetotyp(e$, -6)

        '            IF typ AND ISREFERENCE THEN e$ = refer(e$, typ, 0)


        'PRINT #12, blkoffs$ '???

        e$ = fixoperationorder$(offs$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + "," + sp + tlayout$
        e$ = evaluatetotyp(e$, OFFSETTYPE - ISPOINTER): IF Error_Happened THEN GOTO errmes
        offs$ = e$
        'PRINT #12, e$ '???

        e$ = fixoperationorder$(var$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + "," + sp + tlayout$
        varsize$ = evaluatetotyp(e$, -5): IF Error_Happened THEN GOTO errmes
        varoffs$ = evaluatetotyp(e$, -6): IF Error_Happened THEN GOTO errmes


        'PRINT #12, varoffs$ '???
        'PRINT #12, varsize$ '???

        'what do we do next
        'need to know offset of variable and its size

        'known sizes will be handled by designated command casts, otherwise use memmove
        s = 0
        IF varsize$ = "1" THEN s = 1: st$ = "int8"
        IF varsize$ = "2" THEN s = 2: st$ = "int16"
        IF varsize$ = "4" THEN s = 4: st$ = "int32"
        IF varsize$ = "8" THEN s = 8: st$ = "int64"

        IF NoChecks THEN
            'fast version:
            IF s THEN
                PRINT #12, "*(" + st$ + "*)" + varoffs$ + "=*(" + st$ + "*)(" + offs$ + ");"
            ELSE
                PRINT #12, "memmove(" + varoffs$ + ",(void*)" + offs$ + "," + varsize$ + ");"
            END IF
        ELSE
            'safe version:
            PRINT #12, "tmp_long=" + offs$ + ";"
            'is mem block init?
            PRINT #12, "if ( ((mem_block*)(" + blkoffs$ + "))->lock_offset ){"
            'are region and id valid?
            PRINT #12, "if ("
            PRINT #12, "tmp_long < ((mem_block*)(" + blkoffs$ + "))->offset  ||"
            PRINT #12, "(tmp_long+(" + varsize$ + ")) > ( ((mem_block*)(" + blkoffs$ + "))->offset + ((mem_block*)(" + blkoffs$ + "))->size)  ||"
            PRINT #12, "((mem_lock*)((mem_block*)(" + blkoffs$ + "))->lock_offset)->id != ((mem_block*)(" + blkoffs$ + "))->lock_id  ){"
            'diagnose error
            PRINT #12, "if (" + "((mem_lock*)((mem_block*)(" + blkoffs$ + "))->lock_offset)->id != ((mem_block*)(" + blkoffs$ + "))->lock_id" + ") error(308); else error(300);"
            PRINT #12, "}else{"
            IF s THEN
                PRINT #12, "*(" + st$ + "*)" + varoffs$ + "=*(" + st$ + "*)tmp_long;"
            ELSE
                PRINT #12, "memmove(" + varoffs$ + ",(void*)tmp_long," + varsize$ + ");"
            END IF
            PRINT #12, "}"
            PRINT #12, "}else error(309);"
        END IF

        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline

    END IF
END IF




IF n >= 1 THEN
    IF firstelement$ = "_MEMPUT" OR (firstelement$ = "MEMPUT" AND qbnexprefix_set = 1) THEN
        'get expressions
        typ$ = ""
        e$ = ""
        B = 0
        ne = 0
        FOR i2 = 2 TO n
            e2$ = getelement$(ca$, i2)
            IF e2$ = "(" THEN B = B + 1
            IF e2$ = ")" THEN B = B - 1
            IF (e2$ = "," OR UCASE$(e2$) = "AS") AND B = 0 THEN
                ne = ne + 1
                IF ne = 1 THEN blk$ = e$: e$ = ""
                IF ne = 2 THEN offs$ = e$: e$ = ""
                IF ne = 3 THEN var$ = e$: e$ = ""
                IF (UCASE$(e2$) = "AS" AND ne <> 3) OR (ne = 3 AND UCASE$(e2$) <> "AS") OR ne = 4 THEN a$ = "Expected _MEMPUT mem-reference,offset,variable|value[AS type]": GOTO errmes
            ELSE
                IF LEN(e$) = 0 THEN e$ = e2$ ELSE e$ = e$ + sp + e2$
            END IF
        NEXT
        IF ne < 2 OR e$ = "" THEN a$ = "Expected " + qbnexprefix$ + "MEMPUT mem-reference, offset, variable|value[AS type]": GOTO errmes
        IF ne = 2 THEN var$ = e$ ELSE typ$ = UCASE$(e$)

        IF firstelement$ = "_MEMPUT" THEN l$ = SCase$("_MemPut") + sp ELSE l$ = SCase$("MemPut") + sp

        e$ = fixoperationorder$(blk$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + tlayout$

        test$ = evaluate(e$, typ): IF Error_Happened THEN GOTO errmes
        IF (typ AND ISUDT) = 0 OR (typ AND 511) <> 1 THEN a$ = "Expected " + qbnexprefix$ + "MEM type": GOTO errmes
        blkoffs$ = evaluatetotyp(e$, -6)

        e$ = fixoperationorder$(offs$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + "," + sp + tlayout$
        e$ = evaluatetotyp(e$, OFFSETTYPE - ISPOINTER): IF Error_Happened THEN GOTO errmes
        offs$ = e$

        IF ne = 2 THEN
            e$ = fixoperationorder$(var$): IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp2 + "," + sp + tlayout$

            test$ = evaluate(e$, t): IF Error_Happened THEN GOTO errmes
            IF (t AND ISREFERENCE) = 0 AND (t AND ISSTRING) THEN
                PRINT #12, "g_tmp_str=" + test$ + ";"
                varsize$ = "g_tmp_str->len"
                varoffs$ = "g_tmp_str->chr"
            ELSE
                varsize$ = evaluatetotyp(e$, -5): IF Error_Happened THEN GOTO errmes
                varoffs$ = evaluatetotyp(e$, -6): IF Error_Happened THEN GOTO errmes
            END IF

            'known sizes will be handled by designated command casts, otherwise use memmove
            s = 0
            IF varsize$ = "1" THEN s = 1: st$ = "int8"
            IF varsize$ = "2" THEN s = 2: st$ = "int16"
            IF varsize$ = "4" THEN s = 4: st$ = "int32"
            IF varsize$ = "8" THEN s = 8: st$ = "int64"

            IF NoChecks THEN
                'fast version:
                IF s THEN
                    PRINT #12, "*(" + st$ + "*)(" + offs$ + ")=*(" + st$ + "*)" + varoffs$ + ";"
                ELSE
                    PRINT #12, "memmove((void*)" + offs$ + "," + varoffs$ + "," + varsize$ + ");"
                END IF
            ELSE
                'safe version:
                PRINT #12, "tmp_long=" + offs$ + ";"
                'is mem block init?
                PRINT #12, "if ( ((mem_block*)(" + blkoffs$ + "))->lock_offset ){"
                'are region and id valid?
                PRINT #12, "if ("
                PRINT #12, "tmp_long < ((mem_block*)(" + blkoffs$ + "))->offset  ||"
                PRINT #12, "(tmp_long+(" + varsize$ + ")) > ( ((mem_block*)(" + blkoffs$ + "))->offset + ((mem_block*)(" + blkoffs$ + "))->size)  ||"
                PRINT #12, "((mem_lock*)((mem_block*)(" + blkoffs$ + "))->lock_offset)->id != ((mem_block*)(" + blkoffs$ + "))->lock_id  ){"
                'diagnose error
                PRINT #12, "if (" + "((mem_lock*)((mem_block*)(" + blkoffs$ + "))->lock_offset)->id != ((mem_block*)(" + blkoffs$ + "))->lock_id" + ") error(308); else error(300);"
                PRINT #12, "}else{"
                IF s THEN
                    PRINT #12, "*(" + st$ + "*)tmp_long=*(" + st$ + "*)" + varoffs$ + ";"
                ELSE
                    PRINT #12, "memmove((void*)tmp_long," + varoffs$ + "," + varsize$ + ");"
                END IF
                PRINT #12, "}"
                PRINT #12, "}else error(309);"
            END IF

        ELSE

            '... AS type method
            'FUNCTION typname2typ& (t2$)
            'typname2typsize = 0 'the default
            t = typname2typ(typ$)
            IF t = 0 THEN a$ = "Invalid type": GOTO errmes
            IF (t AND ISOFFSETINBITS) <> 0 OR (t AND ISUDT) <> 0 OR (t AND ISSTRING) THEN a$ = qbnexprefix$ + "MEMPUT requires numeric type": GOTO errmes
            IF (t AND ISPOINTER) THEN t = t - ISPOINTER
            'attempt conversion...
            e$ = fixoperationorder$(var$): IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp2 + "," + sp + tlayout$ + sp + SCase$("As") + sp + typ$
            e$ = evaluatetotyp(e$, t): IF Error_Happened THEN GOTO errmes
            st$ = typ2ctyp$(t, "")
            varsize$ = str2((t AND 511) \ 8)
            IF NoChecks THEN
                'fast version:
                PRINT #12, "*(" + st$ + "*)(" + offs$ + ")=" + e$ + ";"
            ELSE
                'safe version:
                PRINT #12, "tmp_long=" + offs$ + ";"
                'is mem block init?
                PRINT #12, "if ( ((mem_block*)(" + blkoffs$ + "))->lock_offset ){"
                'are region and id valid?
                PRINT #12, "if ("
                PRINT #12, "tmp_long < ((mem_block*)(" + blkoffs$ + "))->offset  ||"
                PRINT #12, "(tmp_long+(" + varsize$ + ")) > ( ((mem_block*)(" + blkoffs$ + "))->offset + ((mem_block*)(" + blkoffs$ + "))->size)  ||"
                PRINT #12, "((mem_lock*)((mem_block*)(" + blkoffs$ + "))->lock_offset)->id != ((mem_block*)(" + blkoffs$ + "))->lock_id  ){"
                'diagnose error
                PRINT #12, "if (" + "((mem_lock*)((mem_block*)(" + blkoffs$ + "))->lock_offset)->id != ((mem_block*)(" + blkoffs$ + "))->lock_id" + ") error(308); else error(300);"
                PRINT #12, "}else{"
                PRINT #12, "*(" + st$ + "*)tmp_long=" + e$ + ";"
                PRINT #12, "}"
                PRINT #12, "}else error(309);"
            END IF

        END IF

        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline

    END IF
END IF





IF n >= 1 THEN
    IF firstelement$ = "_MEMFILL" OR (firstelement$ = "MEMFILL" AND qbnexprefix_set = 1) THEN
        'get expressions
        typ$ = ""
        e$ = ""
        B = 0
        ne = 0
        FOR i2 = 2 TO n
            e2$ = getelement$(ca$, i2)
            IF e2$ = "(" THEN B = B + 1
            IF e2$ = ")" THEN B = B - 1
            IF (e2$ = "," OR UCASE$(e2$) = "AS") AND B = 0 THEN
                ne = ne + 1
                IF ne = 1 THEN blk$ = e$: e$ = ""
                IF ne = 2 THEN offs$ = e$: e$ = ""
                IF ne = 3 THEN bytes$ = e$: e$ = ""
                IF ne = 4 THEN var$ = e$: e$ = ""
                IF (UCASE$(e2$) = "AS" AND ne <> 4) OR (ne = 4 AND UCASE$(e2$) <> "AS") OR ne = 5 THEN a$ = "Expected _MEMFILL mem-reference,offset,bytes,variable|value[AS type]": GOTO errmes
            ELSE
                IF LEN(e$) = 0 THEN e$ = e2$ ELSE e$ = e$ + sp + e2$
            END IF
        NEXT
        IF ne < 3 OR e$ = "" THEN a$ = "Expected " + qbnexprefix$ + "MEMFILL mem-reference, offset, bytes, variable|value[AS type]": GOTO errmes
        IF ne = 3 THEN var$ = e$ ELSE typ$ = UCASE$(e$)

        IF firstelement$ = "_MEMFILL" THEN l$ = SCase$("_MemFill") + sp ELSE l$ = SCase$("MemFill") + sp

        e$ = fixoperationorder$(blk$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + tlayout$

        test$ = evaluate(e$, typ): IF Error_Happened THEN GOTO errmes
        IF (typ AND ISUDT) = 0 OR (typ AND 511) <> 1 THEN a$ = "Expected " + qbnexprefix$ + "MEM type": GOTO errmes
        blkoffs$ = evaluatetotyp(e$, -6)

        e$ = fixoperationorder$(offs$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + "," + sp + tlayout$
        e$ = evaluatetotyp(e$, OFFSETTYPE - ISPOINTER): IF Error_Happened THEN GOTO errmes
        offs$ = e$

        e$ = fixoperationorder$(bytes$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + "," + sp + tlayout$
        e$ = evaluatetotyp(e$, OFFSETTYPE - ISPOINTER): IF Error_Happened THEN GOTO errmes
        bytes$ = e$

        IF ne = 3 THEN 'no AS
        e$ = fixoperationorder$(var$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + "," + sp + tlayout$
        test$ = evaluate(e$, t)
        IF (t AND ISREFERENCE) = 0 AND (t AND ISSTRING) THEN
            PRINT #12, "tmp_long=(ptrszint)" + test$ + ";"
            varsize$ = "((qbs*)tmp_long)->len"
            varoffs$ = "((qbs*)tmp_long)->chr"
        ELSE
            varsize$ = evaluatetotyp(e$, -5): IF Error_Happened THEN GOTO errmes
            varoffs$ = evaluatetotyp(e$, -6): IF Error_Happened THEN GOTO errmes
        END IF

        IF NoChecks THEN
            PRINT #12, "sub__memfill_nochecks(" + offs$ + "," + bytes$ + ",(ptrszint)" + varoffs$ + "," + varsize$ + ");"
        ELSE
            PRINT #12, "sub__memfill((mem_block*)" + blkoffs$ + "," + offs$ + "," + bytes$ + ",(ptrszint)" + varoffs$ + "," + varsize$ + ");"
        END IF

    ELSE

        '... AS type method
        t = typname2typ(typ$)
        IF t = 0 THEN a$ = "Invalid type": GOTO errmes
        IF (t AND ISOFFSETINBITS) <> 0 OR (t AND ISUDT) <> 0 OR (t AND ISSTRING) THEN a$ = qbnexprefix$ + "MEMFILL requires numeric type": GOTO errmes
        IF (t AND ISPOINTER) THEN t = t - ISPOINTER
        'attempt conversion...
        e$ = fixoperationorder$(var$): IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + "," + sp + tlayout$ + sp + SCase$("As") + sp + typ$
        e$ = evaluatetotyp(e$, t): IF Error_Happened THEN GOTO errmes

        c$ = "sub__memfill_"
        IF NoChecks THEN c$ = "sub__memfill_nochecks_"
        IF t AND ISOFFSET THEN
            c$ = c$ + "OFFSET"
        ELSE
            IF t AND ISFLOAT THEN
                IF (t AND 511) = 32 THEN c$ = c$ + "SINGLE"
                IF (t AND 511) = 64 THEN c$ = c$ + "DOUBLE"
                IF (t AND 511) = 256 THEN c$ = c$ + "FLOAT" 'padded variable
            ELSE
                c$ = c$ + str2((t AND 511) \ 8)
            END IF
        END IF
        c$ = c$ + "("
        IF NoChecks = 0 THEN c$ = c$ + "(mem_block*)" + blkoffs$ + ","
        PRINT #12, c$ + offs$ + "," + bytes$ + "," + e$ + ");"
    END IF

    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    GOTO finishedline

END IF
END IF













'note: ABSOLUTE cannot be used without CALL
cispecial = 0
IF n > 1 THEN
    IF firstelement$ = "INTERRUPT" OR firstelement$ = "INTERRUPTX" THEN
        a$ = "CALL" + sp + firstelement$ + sp + "(" + sp + getelements$(a$, 2, n) + sp + ")"
        ca$ = "CALL" + sp + firstelement$ + sp + "(" + sp + getelements$(ca$, 2, n) + sp + ")"
        n = n + 3
        firstelement$ = "CALL"
        cispecial = 1
        'fall through
    END IF
END IF

usecall = 0
IF firstelement$ = "CALL" THEN
    usecall = 1
    IF n = 1 THEN a$ = "Expected CALL sub-name [(...)]": GOTO errmes
    cn$ = getelement$(ca$, 2): n$ = UCASE$(cn$)

    IF n > 2 THEN

        IF n <= 4 THEN a$ = "Expected CALL sub-name (...)": GOTO errmes
        IF getelement$(a$, 3) <> "(" OR getelement$(a$, n) <> ")" THEN a$ = "Expected CALL sub-name (...)": GOTO errmes
        a$ = n$ + sp + getelements$(a$, 4, n - 1)
        ca$ = cn$ + sp + getelements$(ca$, 4, n - 1)


        IF n$ = "INTERRUPT" OR n$ = "INTERRUPTX" THEN 'assume CALL INTERRUPT[X] request
        'print "CI: call interrupt command reached":sleep 1
        IF n$ = "INTERRUPT" THEN PRINT #12, "call_interrupt("; ELSE PRINT #12, "call_interruptx(";
        argn = 0
        n = numelements(a$)
        B = 0
        e$ = ""
        FOR i = 2 TO n
            e2$ = getelement$(ca$, i)
            IF e2$ = "(" THEN B = B + 1
            IF e2$ = ")" THEN B = B - 1
            IF (e2$ = "," AND B = 0) OR i = n THEN
                IF i = n THEN
                    IF e$ = "" THEN e$ = e2$ ELSE e$ = e$ + sp + e2$
                END IF
                argn = argn + 1
                IF argn = 1 THEN 'interrupt number
                e$ = fixoperationorder$(e$)
                IF Error_Happened THEN GOTO errmes
                l$ = SCase$("Call") + sp + n$ + sp2 + "(" + sp2 + tlayout$
                IF cispecial = 1 THEN l$ = n$ + sp + tlayout$
                e$ = evaluatetotyp(e$, 64&)
                IF Error_Happened THEN GOTO errmes
                'print "CI: evaluated interrupt number as ["+e$+"]":sleep 1
                PRINT #12, e$;
            END IF
            IF argn = 2 OR argn = 3 THEN 'inregs, outregs
            e$ = fixoperationorder$(e$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp2 + "," + sp + tlayout$
            e2$ = e$
            e$ = evaluatetotyp(e$, -2) 'offset+size
            IF Error_Happened THEN GOTO errmes
            'print "CI: evaluated in/out regs ["+e2$+"] as ["+e$+"]":sleep 1
            PRINT #12, "," + e$;
        END IF
        e$ = ""
    ELSE
        IF e$ = "" THEN e$ = e2$ ELSE e$ = e$ + sp + e2$
    END IF
NEXT
IF argn <> 3 THEN a$ = "Expected CALL INTERRUPT (interrupt-no, inregs, outregs)": GOTO errmes
PRINT #12, ");"
IF cispecial = 0 THEN l$ = l$ + sp2 + ")"
layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
'print "CI: done":sleep 1
GOTO finishedline
END IF 'call interrupt








'call to CALL ABSOLUTE beyond reasonable doubt
IF n$ = "ABSOLUTE" THEN
    l$ = SCase$("Call" + sp + "Absolute" + sp2 + "(" + sp2)
    argn = 0
    n = numelements(a$)
    B = 0
    e$ = ""
    FOR i = 2 TO n
        e2$ = getelement$(ca$, i)
        IF e2$ = "(" THEN B = B + 1
        IF e2$ = ")" THEN B = B - 1
        IF (e2$ = "," AND B = 0) OR i = n THEN
            IF i < n THEN
                IF e$ = "" THEN a$ = "Expected expression before , or )": GOTO errmes
                '1. variable or value?
                e$ = fixoperationorder$(e$)
                IF Error_Happened THEN GOTO errmes
                l$ = l$ + tlayout$ + sp2 + "," + sp
                ignore$ = evaluate(e$, typ)
                IF Error_Happened THEN GOTO errmes

                IF (typ AND ISPOINTER) <> 0 AND (typ AND ISREFERENCE) <> 0 THEN

                    'assume standard variable
                    'assume not string/array/udt/etc
                    e$ = "VARPTR" + sp + "(" + sp + e$ + sp + ")"
                    e$ = evaluatetotyp(e$, UINTEGERTYPE - ISPOINTER)
                    IF Error_Happened THEN GOTO errmes

                ELSE

                    'assume not string
                    'single, double or integer64?
                    IF typ AND ISFLOAT THEN
                        IF (typ AND 511) = 32 THEN
                            e$ = evaluatetotyp(e$, SINGLETYPE - ISPOINTER)
                            IF Error_Happened THEN GOTO errmes
                            v$ = "pass" + str2$(uniquenumber)
                            PRINT #defdatahandle, "float *" + v$ + "=NULL;"
                            PRINT #13, "if(" + v$ + "==NULL){"
                            PRINT #13, "cmem_sp-=4;"
                            PRINT #13, v$ + "=(float*)(dblock+cmem_sp);"
                            PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
                            PRINT #13, "}"
                            e$ = "(uint16)(((uint8*)&(*" + v$ + "=" + e$ + "))-((uint8*)dblock))"
                        ELSE
                            e$ = evaluatetotyp(e$, DOUBLETYPE - ISPOINTER)
                            IF Error_Happened THEN GOTO errmes
                            v$ = "pass" + str2$(uniquenumber)
                            PRINT #defdatahandle, "double *" + v$ + "=NULL;"
                            PRINT #13, "if(" + v$ + "==NULL){"
                            PRINT #13, "cmem_sp-=8;"
                            PRINT #13, v$ + "=(double*)(dblock+cmem_sp);"
                            PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
                            PRINT #13, "}"
                            e$ = "(uint16)(((uint8*)&(*" + v$ + "=" + e$ + "))-((uint8*)dblock))"
                        END IF
                    ELSE
                        e$ = evaluatetotyp(e$, INTEGER64TYPE - ISPOINTER)
                        IF Error_Happened THEN GOTO errmes
                        v$ = "pass" + str2$(uniquenumber)
                        PRINT #defdatahandle, "int64 *" + v$ + "=NULL;"
                        PRINT #13, "if(" + v$ + "==NULL){"
                        PRINT #13, "cmem_sp-=8;"
                        PRINT #13, v$ + "=(int64*)(dblock+cmem_sp);"
                        PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
                        PRINT #13, "}"
                        e$ = "(uint16)(((uint8*)&(*" + v$ + "=" + e$ + "))-((uint8*)dblock))"
                    END IF

                END IF

                PRINT #12, "call_absolute_offsets[" + str2$(argn) + "]=" + e$ + ";"
            ELSE
                IF e$ = "" THEN e$ = e2$ ELSE e$ = e$ + sp + e2$
                e$ = fixoperationorder(e$)
                IF Error_Happened THEN GOTO errmes
                l$ = l$ + tlayout$ + sp2 + ")"
                e$ = evaluatetotyp(e$, UINTEGERTYPE - ISPOINTER)
                IF Error_Happened THEN GOTO errmes
                PRINT #12, "call_absolute(" + str2$(argn) + "," + e$ + ");"
            END IF
            argn = argn + 1
            e$ = ""
        ELSE
            IF e$ = "" THEN e$ = e2$ ELSE e$ = e$ + sp + e2$
        END IF
    NEXT
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    GOTO finishedline
END IF

ELSE 'n>2

    a$ = n$
    ca$ = cn$
    usecall = 2

END IF 'n>2

n = numelements(a$)
firstelement$ = getelement$(a$, 1)

'valid SUB name
validsub = 0
findidsecondarg = "": IF n >= 2 THEN findidsecondarg = getelement$(a$, 2)
try = findid(firstelement$)
IF Error_Happened THEN GOTO errmes
DO WHILE try
    IF id.subfunc = 2 THEN validsub = 1: EXIT DO
    IF try = 2 THEN
        findidsecondarg = "": IF n >= 2 THEN findidsecondarg = getelement$(a$, 2)
        findanotherid = 1
        try = findid(firstelement$)
        IF Error_Happened THEN GOTO errmes
    ELSE
        try = 0
    END IF
LOOP
IF validsub = 0 THEN a$ = "Expected CALL sub-name [(...)]": GOTO errmes
END IF

'sub?
IF n >= 1 THEN

    IF firstelement$ = "?" THEN firstelement$ = "PRINT"

    findidsecondarg = "": IF n >= 2 THEN findidsecondarg = getelement$(a$, 2)
    try = findid(firstelement$)
    IF Error_Happened THEN GOTO errmes
    DO WHILE try
        IF id.subfunc = 2 THEN

            'check symbol
            s$ = removesymbol$(firstelement$ + "")
            IF Error_Happened THEN GOTO errmes
            IF ASC(id.musthave) = 36 THEN '="$"
            IF s$ <> "$" THEN GOTO notsubcall 'missing musthave "$"
        ELSE
            IF LEN(s$) THEN GOTO notsubcall 'unrequired symbol added
        END IF
        'check for variable assignment
        IF n > 1 THEN
            IF ASC(id.specialformat) <> 61 THEN '<>"="
            IF ASC(getelement$(a$, 2)) = 61 THEN GOTO notsubcall 'assignment, not sub call
        END IF
    END IF
    'check for array assignment
    IF n > 2 THEN
        IF firstelement$ <> "PRINT" AND firstelement$ <> "LPRINT" THEN
            IF getelement$(a$, 2) = "(" THEN
                B = 1
                FOR i = 3 TO n
                    e$ = getelement$(a$, i)
                    IF e$ = "(" THEN B = B + 1
                    IF e$ = ")" THEN
                        B = B - 1
                        IF B = 0 THEN
                            IF i = n THEN EXIT FOR
                            IF getelement$(a$, i + 1) = "=" THEN GOTO notsubcall
                        END IF
                    END IF
                NEXT
            END IF
        END IF
    END IF


    'generate error on driect _GL call
    IF firstelement$ = "_GL" THEN
        a$ = "Cannot call SUB _GL directly": GOTO errmes
    END IF

    IF firstelement$ = "VWATCH" THEN
        a$ = "Cannot call SUB VWATCH directly": GOTO errmes
    END IF

    IF firstelement$ = "OPEN" THEN
        'gwbasic or qbasic version?
        B = 0
        FOR x = 2 TO n
            a2$ = getelement$(a$, x)
            IF a2$ = "(" THEN B = B + 1
            IF a2$ = ")" THEN B = B - 1
            IF a2$ = "FOR" OR a2$ = "AS" THEN EXIT FOR 'qb style open verified
            IF B = 0 AND a2$ = "," THEN 'the gwbasic version includes a comma after the first string expression
            findanotherid = 1
            try = findid(firstelement$) 'id of sub_open_gwbasic
            IF Error_Happened THEN GOTO errmes
            EXIT FOR
        END IF
    NEXT
END IF


'IF findid(firstelement$) THEN
'IF id.subfunc = 2 THEN


IF firstelement$ = "CLOSE" OR firstelement$ = "RESET" THEN
    IF firstelement$ = "RESET" THEN
        IF n > 1 THEN a$ = "Syntax error - RESET takes no parameters": GOTO errmes
        l$ = SCase$("Reset")
    ELSE
        l$ = SCase$("Close")
    END IF

    IF n = 1 THEN
        PRINT #12, "sub_close(NULL,0);" 'closes all files
    ELSE
        l$ = l$ + sp
        B = 0
        s = 0
        a3$ = ""
        FOR x = 2 TO n
            a2$ = getelement$(ca$, x)
            IF a2$ = "(" THEN B = B + 1
            IF a2$ = ")" THEN B = B - 1
            IF a2$ = "#" AND B = 0 THEN
                IF s = 0 THEN s = 1 ELSE a$ = "Unexpected #": GOTO errmes
                l$ = l$ + "#" + sp2
                GOTO closenexta
            END IF

            IF a2$ = "," AND B = 0 THEN
                IF s = 2 THEN
                    e$ = fixoperationorder$(a3$)
                    IF Error_Happened THEN GOTO errmes
                    l$ = l$ + tlayout$ + sp2 + "," + sp
                    e$ = evaluatetotyp(e$, 64&)
                    IF Error_Happened THEN GOTO errmes
                    PRINT #12, "sub_close(" + e$ + ",1);"
                    a3$ = ""
                    s = 0
                    GOTO closenexta
                ELSE
                    a$ = "Expected expression before ,": GOTO errmes
                END IF
            END IF

            s = 2
            IF a3$ = "" THEN a3$ = a2$ ELSE a3$ = a3$ + sp + a2$

            closenexta:
        NEXT

        IF s = 2 THEN
            e$ = fixoperationorder$(a3$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + tlayout$
            e$ = evaluatetotyp(e$, 64&)
            IF Error_Happened THEN GOTO errmes
            PRINT #12, "sub_close(" + e$ + ",1);"
        ELSE
            l$ = LEFT$(l$, LEN(l$) - 1)
        END IF

    END IF
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    GOTO finishedline
END IF 'close
















'data, restore, read
IF firstelement$ = "READ" THEN 'file input
xread ca$, n
IF Error_Happened THEN GOTO errmes
'note: layout done in xread sub
GOTO finishedline
END IF 'read





































lineinput = 0
IF n >= 2 THEN
    IF firstelement$ = "LINE" AND secondelement$ = "INPUT" THEN
        lineinput = 1
        a$ = RIGHT$(a$, LEN(a$) - 5): ca$ = RIGHT$(ca$, LEN(ca$) - 5): n = n - 1 'remove "LINE"
        firstelement$ = "INPUT"
    END IF
END IF

IF firstelement$ = "INPUT" THEN 'file input
IF n > 1 THEN
    IF getelement$(a$, 2) = "#" THEN
        l$ = SCase$("Input") + sp + "#": IF lineinput THEN l$ = SCase$("Line") + sp + l$

        u$ = str2$(uniquenumber)
        'which file?
        IF n = 2 THEN a$ = "Expected # ... , ...": GOTO errmes
        a3$ = ""
        B = 0
        FOR i = 3 TO n
            a2$ = getelement$(ca$, i)
            IF a2$ = "(" THEN B = B + 1
            IF a2$ = ")" THEN B = B - 1
            IF a2$ = "," AND B = 0 THEN
                IF a3$ = "" THEN a$ = "Expected # ... , ...": GOTO errmes
                GOTO inputgotfn
            END IF
            IF a3$ = "" THEN a3$ = a2$ ELSE a3$ = a3$ + sp + a2$
        NEXT
        inputgotfn:
        e$ = fixoperationorder$(a3$)
        IF Error_Happened THEN GOTO errmes
        l$ = l$ + sp2 + tlayout$
        e$ = evaluatetotyp(e$, 64&)
        IF Error_Happened THEN GOTO errmes
        PRINT #12, "tmp_fileno=" + e$ + ";"
        PRINT #12, "if (new_error) goto skip" + u$ + ";"
        i = i + 1
        IF i > n THEN a$ = "Expected , ...": GOTO errmes
        a3$ = ""
        B = 0
        FOR i = i TO n
            a2$ = getelement$(ca$, i)
            IF a2$ = "(" THEN B = B + 1
            IF a2$ = ")" THEN B = B - 1
            IF i = n THEN
                IF a3$ = "" THEN a3$ = a2$ ELSE a3$ = a3$ + sp + a2$
                a2$ = ",": B = 0
            END IF
            IF a2$ = "," AND B = 0 THEN
                IF a3$ = "" THEN a$ = "Expected , ...": GOTO errmes
                e$ = fixoperationorder$(a3$)
                IF Error_Happened THEN GOTO errmes
                l$ = l$ + sp2 + "," + sp + tlayout$
                e$ = evaluate(e$, t)
                IF Error_Happened THEN GOTO errmes
                IF (t AND ISREFERENCE) = 0 THEN a$ = "Expected variable-name": GOTO errmes
                IF (t AND ISSTRING) THEN
                    e$ = refer(e$, t, 0)
                    IF Error_Happened THEN GOTO errmes
                    IF lineinput THEN
                        PRINT #12, "sub_file_line_input_string(tmp_fileno," + e$ + ");"
                        PRINT #12, "if (new_error) goto skip" + u$ + ";"
                    ELSE
                        PRINT #12, "sub_file_input_string(tmp_fileno," + e$ + ");"
                        PRINT #12, "if (new_error) goto skip" + u$ + ";"
                    END IF
                    stringprocessinghappened = 1
                ELSE
                    IF lineinput THEN a$ = "Expected string-variable": GOTO errmes

                    'numeric variable
                    IF (t AND ISFLOAT) <> 0 OR (t AND 511) <> 64 THEN
                        IF (t AND ISOFFSETINBITS) THEN
                            setrefer e$, t, "((int64)func_file_input_float(tmp_fileno," + str2(t) + "))", 1
                            IF Error_Happened THEN GOTO errmes
                        ELSE
                            setrefer e$, t, "func_file_input_float(tmp_fileno," + str2(t) + ")", 1
                            IF Error_Happened THEN GOTO errmes
                        END IF
                    ELSE
                        IF t AND ISUNSIGNED THEN
                            setrefer e$, t, "func_file_input_uint64(tmp_fileno)", 1
                            IF Error_Happened THEN GOTO errmes
                        ELSE
                            setrefer e$, t, "func_file_input_int64(tmp_fileno)", 1
                            IF Error_Happened THEN GOTO errmes
                        END IF
                    END IF

                    PRINT #12, "if (new_error) goto skip" + u$ + ";"

                END IF
                IF i = n THEN EXIT FOR
                IF lineinput THEN a$ = "Too many variables": GOTO errmes
                a3$ = "": a2$ = ""
            END IF
            IF a3$ = "" THEN a3$ = a2$ ELSE a3$ = a3$ + sp + a2$
        NEXT
        PRINT #12, "skip" + u$ + ":"
        IF stringprocessinghappened THEN PRINT #12, cleanupstringprocessingcall$ + "0);"
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline
    END IF
END IF
END IF 'input#


IF firstelement$ = "INPUT" THEN
    l$ = SCase$("Input"): IF lineinput THEN l$ = SCase$("Line") + sp + l$
    commaneeded = 0
    i = 2

    newline = 1: IF getelement$(a$, i) = ";" THEN newline = 0: i = i + 1: l$ = l$ + sp + ";"

    a2$ = getelement$(ca$, i)
    IF LEFT$(a2$, 1) = CHR$(34) THEN
        e$ = fixoperationorder$(a2$): l$ = l$ + sp + tlayout$
        IF Error_Happened THEN GOTO errmes
        PRINT #12, "qbs_print(qbs_new_txt_len(" + a2$ + "),0);"
        i = i + 1
        'MUST be followed by a ; or ,
        a2$ = getelement$(ca$, i)
        i = i + 1
        l$ = l$ + sp2 + a2$
        IF a2$ = ";" THEN
            IF lineinput THEN GOTO finishedpromptstring
            PRINT #12, "qbs_print(qbs_new_txt(" + CHR$(34) + "? " + CHR$(34) + "),0);"
            GOTO finishedpromptstring
        END IF
        IF a2$ = "," THEN
            GOTO finishedpromptstring
        END IF
        a$ = "Syntax error - Reference: INPUT [;] " + CHR$(34) + "[Question or statement text]" + CHR$(34) + "{,|;} variable[, ...] or INPUT ; variable[, ...]": GOTO errmes
    END IF
    'there was no promptstring, so print a ?
    IF lineinput = 0 THEN PRINT #12, "qbs_print(qbs_new_txt(" + CHR$(34) + "? " + CHR$(34) + "),0);"
    finishedpromptstring:
    numvar = 0
    FOR i = i TO n
        IF commaneeded = 1 THEN
            a2$ = getelement$(ca$, i)
            IF a2$ <> "," THEN a$ = "Syntax error - comma expected": GOTO errmes
        ELSE

            B = 0
            e$ = ""
            FOR i2 = i TO n
                e2$ = getelement$(ca$, i2)
                IF e2$ = "(" THEN B = B + 1
                IF e2$ = ")" THEN B = B - 1
                IF e2$ = "," AND B = 0 THEN i2 = i2 - 1: EXIT FOR
                e$ = e$ + sp + e2$
            NEXT
            i = i2: IF i > n THEN i = n
            IF e$ = "" THEN a$ = "Expected variable": GOTO errmes
            e$ = RIGHT$(e$, LEN(e$) - 1)
            e$ = fixoperationorder$(e$)
            IF Error_Happened THEN GOTO errmes
            l$ = l$ + sp + tlayout$: IF i <> n THEN l$ = l$ + sp2 + ","
            e$ = evaluate(e$, t)
            IF Error_Happened THEN GOTO errmes
            IF (t AND ISREFERENCE) = 0 THEN a$ = "Expected variable": GOTO errmes

            IF (t AND ISSTRING) THEN
                e$ = refer(e$, t, 0)
                IF Error_Happened THEN GOTO errmes
                numvar = numvar + 1
                IF lineinput THEN
                    PRINT #12, "qbs_input_variabletypes[" + str2(numvar) + "]=ISSTRING+512;"
                ELSE
                    PRINT #12, "qbs_input_variabletypes[" + str2(numvar) + "]=ISSTRING;"
                END IF
                PRINT #12, "qbs_input_variableoffsets[" + str2(numvar) + "]=" + e$ + ";"
                GOTO gotinputvar
            END IF

            IF lineinput THEN a$ = "Expected string variable": GOTO errmes
            IF (t AND ISARRAY) THEN
                IF (t AND ISOFFSETINBITS) THEN
                    a$ = "INPUT cannot handle BIT array elements": GOTO errmes
                END IF
            END IF
            e$ = "&(" + refer(e$, t, 0) + ")"
            IF Error_Happened THEN GOTO errmes

            'remove assumed/unnecessary flags
            IF (t AND ISPOINTER) THEN t = t - ISPOINTER
            IF (t AND ISINCONVENTIONALMEMORY) THEN t = t - ISINCONVENTIONALMEMORY
            IF (t AND ISREFERENCE) THEN t = t - ISREFERENCE

            'IF (t AND ISOFFSETINBITS) THEN
            'numvar = numvar + 1
            'consider storing the bit offset in unused bits of t
            'PRINT #12, "qbs_input_variabletypes[" + str2(numvar) + "]=" + str2(t) + ";"
            'PRINT #12, "qbs_input_variableoffsets[" + str2(numvar) + "]=" + refer(ref$, typ, 1) + ";"
            'GOTO gotinputvar
            'END IF

            'assume it is a regular variable
            numvar = numvar + 1
            PRINT #12, "qbs_input_variabletypes[" + str2(numvar) + "]=" + str2$(t) + ";"
            PRINT #12, "qbs_input_variableoffsets[" + str2(numvar) + "]=" + e$ + ";"
            GOTO gotinputvar

        END IF
        gotinputvar:
        commaneeded = commaneeded + 1: IF commaneeded = 2 THEN commaneeded = 0
    NEXT
    IF numvar = 0 THEN a$ = "Syntax error - Reference: INPUT [;] " + CHR$(34) + "[Question or statement text]" + CHR$(34) + "{,|;} variable[, ...] or INPUT ; variable[, ...]": GOTO errmes
    IF lineinput = 1 AND numvar > 1 THEN a$ = "Too many variables": GOTO errmes
    IF vWatchOn = 1 THEN
        PRINT #12, "*__LONG_VWATCH_LINENUMBER= -4; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);"
    END IF
    PRINT #12, "qbs_input(" + str2(numvar) + "," + str2$(newline) + ");"
    PRINT #12, "if (stop_program) end();"
    IF vWatchOn = 1 THEN
        PRINT #12, "*__LONG_VWATCH_LINENUMBER= -5; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);"
    END IF
    PRINT #12, cleanupstringprocessingcall$ + "0);"
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    GOTO finishedline
END IF



IF firstelement$ = "WRITE" THEN 'file write
IF n > 1 THEN
    IF getelement$(a$, 2) = "#" THEN
        xfilewrite ca$, n
        IF Error_Happened THEN GOTO errmes
        GOTO finishedline
    END IF '#
END IF 'n>1
END IF '"write"

IF firstelement$ = "WRITE" THEN 'write
xwrite ca$, n
IF Error_Happened THEN GOTO errmes
GOTO finishedline
END IF '"write"

IF firstelement$ = "PRINT" THEN 'file print
IF n > 1 THEN
    IF getelement$(a$, 2) = "#" THEN
        xfileprint a$, ca$, n
        IF Error_Happened THEN GOTO errmes
        l$ = tlayout$
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline
    END IF '#
END IF 'n>1
END IF '"print"

IF firstelement$ = "PRINT" OR firstelement$ = "LPRINT" THEN
    IF secondelement$ <> "USING" THEN 'check to see if we need to auto-add semicolons
    elementon = 2
    redosemi:
    FOR i = elementon TO n - 1
        nextchar$ = getelement$(a$, i + 1)
        IF nextchar$ <> ";" AND nextchar$ <> "," AND nextchar$ <> "+" AND nextchar$ <> ")" THEN
            temp1$ = getelement$(a$, i)
            beginpoint = INSTR(beginpoint, temp1$, CHR$(34))
            endpoint = INSTR(beginpoint + 1, temp1$, CHR$(34) + ",")
            IF beginpoint <> 0 AND endpoint <> 0 THEN 'if we have both positions
            'Quote without semicolon check (like PRINT "abc"123)
            textlength = endpoint - beginpoint - 1
            textvalue$ = MID$(temp1$, endpoint + 2, LEN(LTRIM$(STR$(textlength))))
            IF VAL(textvalue$) = textlength THEN
                insertelements a$, i, ";"
                insertelements ca$, i, ";"
                n = n + 1
                elementon = i + 2 'just a easy way to reduce redundant calls to the routine
                GOTO redosemi
            END IF
        END IF
        IF temp1$ <> "USING" THEN
            IF LEFT$(LTRIM$(nextchar$), 1) = CHR$(34) THEN
                IF temp1$ <> ";" AND temp1$ <> "," AND temp1$ <> "+" AND temp1$ <> "(" THEN
                    insertelements a$, i, ";"
                    insertelements ca$, i, ";"
                    n = n + 1
                    elementon = i + 2 'just a easy way to reduce redundant calls to the routine
                    GOTO redosemi
                END IF
            END IF
        END IF
    END IF
NEXT
END IF

xprint a$, ca$, n
IF Error_Happened THEN GOTO errmes
l$ = tlayout$
layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
GOTO finishedline
END IF



IF firstelement$ = "CLEAR" THEN
    IF subfunc$ <> "" THEN a$ = "CLEAR cannot be used inside a SUB/FUNCTION": GOTO errmes
END IF

'LSET/RSET
IF firstelement$ = "LSET" OR firstelement$ = "RSET" THEN
    IF n = 1 THEN a$ = "Expected " + firstelement$ + " ...": GOTO errmes
    IF firstelement$ = "LSET" THEN l$ = SCase$("LSet") ELSE l$ = SCase$("RSet")
    dest$ = ""
    source$ = ""
    part = 1
    i = 2
    a3$ = ""
    B = 0
    DO
        IF i > n THEN
            IF part <> 2 OR a3$ = "" THEN a$ = "Expected LSET/RSET stringvariable=string": GOTO errmes
            source$ = a3$
            EXIT DO
        END IF
        a2$ = getelement$(ca$, i)
        IF a2$ = "(" THEN B = B + 1
        IF a2$ = ")" THEN B = B - 1
        IF a2$ = "=" AND B = 0 THEN
            IF part = 1 THEN dest$ = a3$: part = 2: a3$ = "": GOTO lrsetgotpart
        END IF
        IF LEN(a3$) THEN a3$ = a3$ + sp + a2$ ELSE a3$ = a2$
        lrsetgotpart:
        i = i + 1
    LOOP
    IF dest$ = "" THEN a$ = "Expected LSET/RSET stringvariable=string": GOTO errmes
    'check if it is a valid source string
    f$ = fixoperationorder$(dest$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + sp + tlayout$ + sp + "="
    e$ = evaluate(f$, sourcetyp)
    IF Error_Happened THEN GOTO errmes
    IF (sourcetyp AND ISREFERENCE) = 0 OR (sourcetyp AND ISSTRING) = 0 THEN a$ = "LSET/RSET expects a string variable/array-element as its first argument": GOTO errmes
    dest$ = evaluatetotyp(f$, ISSTRING)
    IF Error_Happened THEN GOTO errmes
    source$ = fixoperationorder$(source$)
    IF Error_Happened THEN GOTO errmes
    l$ = l$ + sp + tlayout$
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    source$ = evaluatetotyp(source$, ISSTRING)
    IF Error_Happened THEN GOTO errmes
    IF firstelement$ = "LSET" THEN
        PRINT #12, "sub_lset(" + dest$ + "," + source$ + ");"
    ELSE
        PRINT #12, "sub_rset(" + dest$ + "," + source$ + ");"
    END IF
    GOTO finishedline
END IF

'SWAP
IF firstelement$ = "SWAP" THEN
    IF n < 4 THEN a$ = "Expected SWAP ... , ...": GOTO errmes
    B = 0
    ele = 1
    e1$ = ""
    e2$ = ""
    FOR i = 2 TO n
        e$ = getelement$(ca$, i)
        IF e$ = "(" THEN B = B + 1
        IF e$ = ")" THEN B = B - 1
        IF e$ = "," AND B = 0 THEN
            IF ele = 2 THEN a$ = "Expected SWAP ... , ...": GOTO errmes
            ele = 2
        ELSE
            IF ele = 1 THEN e1$ = e1$ + sp + e$ ELSE e2$ = e2$ + sp + e$
        END IF
    NEXT
    IF e2$ = "" THEN a$ = "Expected SWAP ... , ...": GOTO errmes
    e1$ = RIGHT$(e1$, LEN(e1$) - 1): e2$ = RIGHT$(e2$, LEN(e2$) - 1)

    e1$ = fixoperationorder(e1$)
    IF Error_Happened THEN GOTO errmes
    e1l$ = tlayout$
    e2$ = fixoperationorder(e2$)
    IF Error_Happened THEN GOTO errmes
    e2l$ = tlayout$
    e1$ = evaluate(e1$, e1typ): e2$ = evaluate(e2$, e2typ)
    IF Error_Happened THEN GOTO errmes
    IF (e1typ AND ISREFERENCE) = 0 OR (e2typ AND ISREFERENCE) = 0 THEN a$ = "Expected variable": GOTO errmes

    layoutdone = 1
    l$ = SCase$("Swap") + sp + e1l$ + sp2 + "," + sp + e2l$
    IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$

    'swap strings?
    IF (e1typ AND ISSTRING) THEN
        IF (e2typ AND ISSTRING) = 0 THEN a$ = "Type mismatch": GOTO errmes
        e1$ = refer(e1$, e1typ, 0): e2$ = refer(e2$, e2typ, 0)
        IF Error_Happened THEN GOTO errmes
        PRINT #12, "swap_string(" + e1$ + "," + e2$ + ");"
        GOTO finishedline
    END IF

    'swap UDT?
    'note: entire UDTs, unlike thier elements cannot be swapped like standard variables
    '      as UDT sizes may vary, and to avoid a malloc operation, QBNex should allocate a buffer
    '      in global.txt for the purpose of swapping each UDT type

    IF e1typ AND ISUDT THEN
        a$ = e1$
        'retrieve ID
        i = INSTR(a$, sp3)
        IF i THEN
            idnumber = VAL(LEFT$(a$, i - 1)): a$ = RIGHT$(a$, LEN(a$) - i)
            getid idnumber
            IF Error_Happened THEN GOTO errmes
            u = VAL(a$)
            i = INSTR(a$, sp3): a$ = RIGHT$(a$, LEN(a$) - i): E = VAL(a$)
            i = INSTR(a$, sp3): o$ = RIGHT$(a$, LEN(a$) - i)
            n$ = "UDT_" + RTRIM$(id.n): IF id.t = 0 THEN n$ = "ARRAY_" + n$ + "[0]"
            IF E = 0 THEN 'not an element of UDT u
            lhsscope$ = scope$
            e$ = e2$: t2 = e2typ
            IF (t2 AND ISUDT) = 0 THEN a$ = "Expected SWAP with similar user defined type": GOTO errmes
            idnumber2 = VAL(e$)
            getid idnumber2
            IF Error_Happened THEN GOTO errmes
            n2$ = "UDT_" + RTRIM$(id.n): IF id.t = 0 THEN n2$ = "ARRAY_" + n2$ + "[0]"
            i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i): u2 = VAL(e$)
            i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i): e2 = VAL(e$)

            i = INSTR(e$, sp3): o2$ = RIGHT$(e$, LEN(e$) - i)
            'WARNING: u2 may need minor modifications based on e to see if they are the same
            IF u <> u2 OR e2 <> 0 THEN a$ = "Expected SWAP with similar user defined type": GOTO errmes
            dst$ = "(((char*)" + lhsscope$ + n$ + ")+(" + o$ + "))"
            src$ = "(((char*)" + scope$ + n2$ + ")+(" + o2$ + "))"
            B = udtxsize(u) \ 8
            siz$ = str2$(B)
            IF B = 1 THEN PRINT #12, "swap_8(" + src$ + "," + dst$ + ");"
            IF B = 2 THEN PRINT #12, "swap_16(" + src$ + "," + dst$ + ");"
            IF B = 4 THEN PRINT #12, "swap_32(" + src$ + "," + dst$ + ");"
            IF B = 8 THEN PRINT #12, "swap_64(" + src$ + "," + dst$ + ");"
            IF B <> 1 AND B <> 2 AND B <> 4 AND B <> 8 THEN PRINT #12, "swap_block(" + src$ + "," + dst$ + "," + siz$ + ");"
            GOTO finishedline
        END IF 'e=0
    END IF 'i
END IF 'isudt

'cull irrelavent flags to make comparison possible
e1typc = e1typ
IF e1typc AND ISPOINTER THEN e1typc = e1typc - ISPOINTER
IF e1typc AND ISINCONVENTIONALMEMORY THEN e1typc = e1typc - ISINCONVENTIONALMEMORY
IF e1typc AND ISARRAY THEN e1typc = e1typc - ISARRAY
IF e1typc AND ISUNSIGNED THEN e1typc = e1typc - ISUNSIGNED
IF e1typc AND ISUDT THEN e1typc = e1typc - ISUDT
e2typc = e2typ
IF e2typc AND ISPOINTER THEN e2typc = e2typc - ISPOINTER
IF e2typc AND ISINCONVENTIONALMEMORY THEN e2typc = e2typc - ISINCONVENTIONALMEMORY
IF e2typc AND ISARRAY THEN e2typc = e2typc - ISARRAY
IF e2typc AND ISUNSIGNED THEN e2typc = e2typc - ISUNSIGNED
IF e2typc AND ISUDT THEN e2typc = e2typc - ISUDT
IF e1typc <> e2typc THEN a$ = "Type mismatch": GOTO errmes
t = e1typ
IF t AND ISOFFSETINBITS THEN a$ = "Cannot SWAP bit-length variables": GOTO errmes
B = t AND 511
t$ = str2$(B): IF B > 64 THEN t$ = "longdouble"
PRINT #12, "swap_" + t$ + "(&" + refer(e1$, e1typ, 0) + ",&" + refer(e2$, e2typ, 0) + ");"
IF Error_Happened THEN GOTO errmes
GOTO finishedline
END IF

IF firstelement$ = "OPTION" THEN
    IF optionexplicit = 0 THEN e$ = " or OPTION " + qbnexprefix$ + "EXPLICIT" ELSE e$ = ""
    IF optionexplicitarray = 0 THEN e$ = e$ + " or OPTION " + qbnexprefix$ + "EXPLICITARRAY"
    IF n = 1 THEN a$ = "Expected OPTION BASE" + e$: GOTO errmes
    e$ = getelement$(a$, 2)
    SELECT CASE e$
    CASE "BASE"
        l$ = getelement$(a$, 3)
        IF l$ <> "0" AND l$ <> "1" THEN a$ = "Expected OPTION BASE 0 or 1": GOTO errmes
        IF l$ = "1" THEN optionbase = 1 ELSE optionbase = 0
        l$ = SCase$("Option" + sp + "Base") + sp + l$
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline
    CASE "EXPLICIT", "_EXPLICIT"
        IF e$ = "EXPLICIT" AND qbnexprefix$ = "_" THEN
            IF optionexplicit = 0 THEN e$ = " or OPTION " + qbnexprefix$ + "EXPLICIT" ELSE e$ = ""
            IF optionexplicitarray = 0 THEN e$ = e$ + " or OPTION " + qbnexprefix$ + "EXPLICITARRAY"
            a$ = "Expected OPTION BASE" + e$: GOTO errmes
        END IF

        opex_desiredState = -1
        IF optionexplicit = 0 THEN
            IF opex_recompileAttempts = 0 THEN
                opex_recompileAttempts = opex_recompileAttempts + 1
                GOTO do_recompile
            END IF
        END IF

        l$ = SCase$("Option") + sp
        IF e$ = "EXPLICIT" THEN l$ = l$ + SCase$("Explicit") ELSE l$ = l$ + SCase$("_Explicit")
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline
    CASE "EXPLICITARRAY", "_EXPLICITARRAY"
        IF e$ = "EXPLICITARRAY" AND qbnexprefix$ = "_" THEN
            IF optionexplicit = 0 THEN e$ = " or OPTION " + qbnexprefix$ + "EXPLICIT" ELSE e$ = ""
            IF optionexplicitarray = 0 THEN e$ = e$ + " or OPTION " + qbnexprefix$ + "EXPLICITARRAY"
            a$ = "Expected OPTION BASE" + e$: GOTO errmes
        END IF

        opexarray_desiredState = -1
        IF optionexplicitarray = 0 THEN
            IF opexarray_recompileAttempts = 0 THEN
                opexarray_recompileAttempts = opexarray_recompileAttempts + 1
                GOTO do_recompile
            END IF
        END IF

        l$ = SCase$("Option") + sp
        IF e$ = "EXPLICITARRAY" THEN l$ = l$ + SCase$("ExplicitArray") ELSE l$ = l$ + SCase$("_ExplicitArray")
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        GOTO finishedline
    CASE ELSE
        IF optionexplicit = 0 THEN e$ = " or OPTION " + qbnexprefix$ + "EXPLICIT" ELSE e$ = ""
        IF optionexplicitarray = 0 THEN e$ = e$ + " or OPTION " + qbnexprefix$ + "EXPLICITARRAY"
        a$ = "Expected OPTION BASE" + e$: GOTO errmes
    END SELECT
END IF

'any other "unique" subs can be processed above

id2 = id

targetid = currentid

IF RTRIM$(id2.callname) = "sub_stub" THEN a$ = "Command not implemented": GOTO errmes

IF n > 1 THEN
    IF id2.args = 0 THEN a$ = "SUB does not require any arguments": GOTO errmes
END IF

SetDependency id2.Dependency

seperateargs_error = 0
passedneeded = seperateargs(getelements(a$, 2, n), getelements(ca$, 2, n), passed&)
IF seperateargs_error THEN a$ = seperateargs_error_message: GOTO errmes

'backup args to local string array space before calling evaluate
FOR i = 1 TO id2.args
    separgs_local$(i) = separgs$(i)
NEXT
FOR i = 1 TO id2.args + 1
    separgslayout_local$(i) = separgslayout$(i)
NEXT



IF Debug THEN
    PRINT #9, "separgs:"
    FOR i = 1 TO id2.args
        PRINT #9, i, separgs_local$(i)
    NEXT
    PRINT #9, "separgslayout:"
    FOR i = 1 TO id2.args + 1
        PRINT #9, i, separgslayout_local$(i)
    NEXT
END IF



'note: seperateargs finds the arguments to pass and sets passed& as necessary
'      FIXOPERTIONORDER is not called on these args yet
'      what we need it to do is build a second array of layout info at the same time
'   ref:DIM SHARED separgslayout(100) AS STRING
'   the above array stores what layout info (if any) goes BEFORE the arg in question
'       it has one extra index which is the arg after

IF usecall THEN
    IF id.internal_subfunc THEN
        IF usecall = 1 THEN l$ = SCase$("Call") + sp + SCase$(RTRIM$(id.cn)) + RTRIM$(id.musthave) + sp2 + "(" + sp2
        IF usecall = 2 THEN l$ = SCase$("Call") + sp + SCase$(RTRIM$(id.cn)) + RTRIM$(id.musthave) + sp 'sp at end for easy parsing
    ELSE
        IF usecall = 1 THEN l$ = SCase$("Call") + sp + RTRIM$(id.cn) + RTRIM$(id.musthave) + sp2 + "(" + sp2
        IF usecall = 2 THEN l$ = SCase$("Call") + sp + RTRIM$(id.cn) + RTRIM$(id.musthave) + sp 'sp at end for easy parsing
    END IF
ELSE
    IF id.internal_subfunc THEN
        l$ = SCase$(RTRIM$(id.cn)) + RTRIM$(id.musthave) + sp
    ELSE
        l$ = RTRIM$(id.cn) + RTRIM$(id.musthave) + sp
    END IF
END IF

subcall$ = RTRIM$(id.callname) + "("
addedlayout = 0

fieldcall = 0
'GET/PUT field exception
IF RTRIM$(id2.callname) = "sub_get" OR RTRIM$(id2.callname) = "sub_put" THEN
    IF passed AND 2 THEN
        'regular GET/PUT call with variable provided
        passed = passed - 2 'for complience with existing methods, remove 'passed' flag for the passing of a variable
    ELSE
        'FIELD GET/PUT call with variable omited
        IF RTRIM$(id2.callname) = "sub_get" THEN
            fieldcall = 1
            subcall$ = "field_get("
        ELSE
            fieldcall = 2
            subcall$ = "field_put("
        END IF
    END IF
END IF 'field exception

IF RTRIM$(id2.callname) = "sub_timer" OR RTRIM$(id2.callname) = "sub_key" THEN 'spacing exception
IF usecall = 0 THEN
    l$ = LEFT$(l$, LEN(l$) - 1) + sp2
END IF
END IF

FOR i = 1 TO id2.args
    targettyp = CVL(MID$(id2.arg, -3 + i * 4, 4))
    nele = ASC(MID$(id2.nele, i, 1))
    nelereq = ASC(MID$(id2.nelereq, i, 1))

    addlayout = 1 'omits option values in layout (eg. BINARY="2")
    convertspacing = 0 'if an 'equation' is next, it will be preceeded by a space
    x$ = separgslayout_local$(i)
    DO WHILE LEN(x$)
        x = ASC(x$)
        IF x THEN
            convertspacing = 0
            x2$ = MID$(x$, 2, x)
            x$ = RIGHT$(x$, LEN(x$) - x - 1)

            s = 0
            an = 0
            x3$ = RIGHT$(l$, 1)
            IF x3$ = sp THEN s = 1
            IF x3$ = sp2 THEN
                s = 2
                IF alphanumeric(ASC(RIGHT$(l$, 2))) THEN an = 1
            ELSE
                IF alphanumeric(ASC(x3$)) THEN an = 1
            END IF
            s1 = s

            IF alphanumeric(ASC(x2$)) THEN convertspacing = 1


            IF x2$ = "LPRINT" THEN

                'x2$="LPRINT"
                'x$=CHR$(0)
                'x3$=[sp] from WIDTH[sp]
                'therefore...
                's=1
                'an=0
                'convertspacing=1


                'if debug=1 then
                'print #9,"LPRINT:"
                'print #9,s
                'print #9,an
                'print #9,l$
                'print #9,x2$
                'end if

            END IF




            IF (an = 1 OR addedlayout = 1) AND alphanumeric(ASC(x2$)) <> 0 THEN



                s = 1 'force space
                x2$ = x2$ + sp2
                GOTO customlaychar
            END IF

            IF x2$ = "=" THEN
                s = 1
                x2$ = x2$ + sp
                GOTO customlaychar
            END IF

            IF x2$ = "#" THEN
                s = 1
                x2$ = x2$ + sp2
                GOTO customlaychar
            END IF

            IF x2$ = "," THEN x2$ = x2$ + sp: GOTO customlaychar


            IF x$ = CHR$(0) THEN 'substitution
            IF x2$ = "STEP" THEN x2$ = x2$ + sp2: GOTO customlaychar
            x2$ = x2$ + sp: GOTO customlaychar
        END IF

        'default solution sp2+?+sp2
        x2$ = x2$ + sp2





        customlaychar:
        IF s = 0 THEN s = 2
        IF s <> s1 THEN
            IF s1 THEN l$ = LEFT$(l$, LEN(l$) - 1)
            IF s = 1 THEN l$ = l$ + sp
            IF s = 2 THEN l$ = l$ + sp2
        END IF

        IF (RTRIM$(id2.callname) = "sub_timer" OR RTRIM$(id2.callname) = "sub_key") AND i = id2.args THEN 'spacing exception
        IF x2$ <> ")" + sp2 THEN
            l$ = LEFT$(l$, LEN(l$) - 1) + sp
        END IF
    END IF

    l$ = l$ + x2$

ELSE
    addlayout = 0
    x$ = RIGHT$(x$, LEN(x$) - 1)
END IF
addedlayout = 0
LOOP



'---better sub syntax checking begins here---



IF targettyp = -3 THEN
    IF separgs_local$(i) = "N-LL" THEN a$ = "Expected array name": GOTO errmes
    'names of numeric arrays have ( ) automatically appended (nothing else)
    e$ = separgs_local$(i)

    IF INSTR(e$, sp) = 0 THEN 'one element only
    try_string$ = e$
    try = findid(try_string$)
    IF Error_Happened THEN GOTO errmes
    DO
        IF try THEN
            IF id.arraytype THEN
                IF (id.arraytype AND ISSTRING) = 0 THEN
                    e$ = e$ + sp + "(" + sp + ")"
                    EXIT DO
                END IF
            END IF
            '---
            IF try = 2 THEN findanotherid = 1: try = findid(try_string$) ELSE try = 0
            IF Error_Happened THEN GOTO errmes
        END IF 'if try
        IF try = 0 THEN 'add symbol?
        IF LEN(removesymbol$(try_string$)) = 0 THEN
            IF Error_Happened THEN GOTO errmes
            a = ASC(try_string$)
            IF a >= 97 AND a <= 122 THEN a = a - 32
            IF a = 95 THEN a = 91
            a = a - 64
            IF LEN(defineextaz(a)) THEN try_string$ = try_string$ + defineextaz(a): try = findid(try_string$)
            IF Error_Happened THEN GOTO errmes
        END IF
    END IF 'try=0
LOOP UNTIL try = 0
END IF 'one element only



e$ = fixoperationorder$(e$)
IF Error_Happened THEN GOTO errmes
IF convertspacing = 1 AND addlayout = 1 THEN l$ = LEFT$(l$, LEN(l$) - 1) + sp
IF addlayout THEN l$ = l$ + tlayout$: addedlayout = 1
e$ = evaluatetotyp(e$, -2)
IF Error_Happened THEN GOTO errmes
GOTO sete
END IF '-3


IF targettyp = -2 THEN
    e$ = fixoperationorder$(e$)
    IF Error_Happened THEN GOTO errmes
    IF convertspacing = 1 AND addlayout = 1 THEN l$ = LEFT$(l$, LEN(l$) - 1) + sp
    IF addlayout THEN l$ = l$ + tlayout$: addedlayout = 1
    e$ = evaluatetotyp(e$, -2)
    IF Error_Happened THEN GOTO errmes
    GOTO sete
END IF '-2

IF targettyp = -4 THEN

    IF fieldcall THEN
        i = id2.args + 1
        EXIT FOR
    END IF

    IF separgs_local$(i) = "N-LL" THEN a$ = "Expected variable name/array element": GOTO errmes
    e$ = fixoperationorder$(separgs_local$(i))
    IF Error_Happened THEN GOTO errmes
    IF convertspacing = 1 AND addlayout = 1 THEN l$ = LEFT$(l$, LEN(l$) - 1) + sp
    IF addlayout THEN l$ = l$ + tlayout$: addedlayout = 1

    'GET/PUT RANDOM-ACCESS override
    IF firstelement$ = "GET" OR firstelement$ = "PUT" THEN
        e2$ = e$ 'backup
        e$ = evaluate(e$, sourcetyp)
        IF Error_Happened THEN GOTO errmes
        IF (sourcetyp AND ISSTRING) THEN
            IF (sourcetyp AND ISFIXEDLENGTH) = 0 THEN
                'replace name of sub to call
                subcall$ = RIGHT$(subcall$, LEN(subcall$) - 7) 'delete original name
                'note: GET2 & PUT2 take differing input, following code is correct
                IF firstelement$ = "GET" THEN
                    subcall$ = "sub_get2" + subcall$
                    e$ = refer(e$, sourcetyp, 0) 'pass a qbs pointer instead
                    IF Error_Happened THEN GOTO errmes
                    GOTO sete
                ELSE
                    subcall$ = "sub_put2" + subcall$
                    'no goto sete required, fall through
                END IF
            END IF
        END IF
        e$ = e2$ 'restore
    END IF 'override

    e$ = evaluatetotyp(e$, -4)
    IF Error_Happened THEN GOTO errmes
    GOTO sete
END IF '-4

IF separgs_local$(i) = "N-LL" THEN
    e$ = "NULL"
ELSE

    e2$ = fixoperationorder$(separgs_local$(i))
    IF Error_Happened THEN GOTO errmes
    IF convertspacing = 1 AND addlayout = 1 THEN l$ = LEFT$(l$, LEN(l$) - 1) + sp
    IF addlayout THEN l$ = l$ + tlayout$: addedlayout = 1

    e$ = evaluate(e2$, sourcetyp)
    IF Error_Happened THEN GOTO errmes

    IF sourcetyp AND ISOFFSET THEN
        IF (targettyp AND ISOFFSET) = 0 THEN
            IF id2.internal_subfunc = 0 THEN a$ = "Cannot convert _OFFSET type to other types": GOTO errmes
        END IF
    END IF

    IF RTRIM$(id2.callname) = "sub_paint" THEN
        IF i = 3 THEN
            IF (sourcetyp AND ISSTRING) THEN
                targettyp = ISSTRING
            END IF
        END IF
    END IF

    IF LEFT$(separgs_local$(i), 2) = "(" + sp THEN dereference = 1 ELSE dereference = 0

    'pass by reference
    IF (targettyp AND ISPOINTER) THEN
        IF dereference = 0 THEN 'check deferencing wasn't used

        'note: array pointer
        IF (targettyp AND ISARRAY) THEN
            IF (sourcetyp AND ISREFERENCE) = 0 THEN a$ = "Expected arrayname()": GOTO errmes
            IF (sourcetyp AND ISARRAY) = 0 THEN a$ = "Expected arrayname()": GOTO errmes
            IF Debug THEN PRINT #9, "sub:array reference:[" + e$ + "]"

            'check arrays are of same type
            targettyp2 = targettyp: sourcetyp2 = sourcetyp
            targettyp2 = targettyp2 AND (511 + ISOFFSETINBITS + ISUDT + ISSTRING + ISFIXEDLENGTH + ISFLOAT)
            sourcetyp2 = sourcetyp2 AND (511 + ISOFFSETINBITS + ISUDT + ISSTRING + ISFIXEDLENGTH + ISFLOAT)
            IF sourcetyp2 <> targettyp2 THEN a$ = "Incorrect array type passed to sub": GOTO errmes

            'check arrayname was followed by '()'
            IF targettyp AND ISUDT THEN
                IF Debug THEN PRINT #9, "sub:array reference:udt reference:[" + e$ + "]"
                'get UDT info
                udtrefid = VAL(e$)
                getid udtrefid
                IF Error_Happened THEN GOTO errmes
                udtrefi = INSTR(e$, sp3) 'end of id
                udtrefi2 = INSTR(udtrefi + 1, e$, sp3) 'end of u
                udtrefu = VAL(MID$(e$, udtrefi + 1, udtrefi2 - udtrefi - 1))
                udtrefi3 = INSTR(udtrefi2 + 1, e$, sp3) 'skip e
                udtrefe = VAL(MID$(e$, udtrefi2 + 1, udtrefi3 - udtrefi2 - 1))
                o$ = RIGHT$(e$, LEN(e$) - udtrefi3)
                'note: most of the UDT info above is not required
                IF LEFT$(o$, 4) <> "(0)*" THEN a$ = "Expected arrayname()": GOTO errmes
            ELSE
                IF RIGHT$(e$, 2) <> sp3 + "0" THEN a$ = "Expected arrayname()": GOTO errmes
            END IF

            idnum = VAL(LEFT$(e$, INSTR(e$, sp3) - 1))
            getid idnum
            IF Error_Happened THEN GOTO errmes

            IF targettyp AND ISFIXEDLENGTH THEN
                targettypsize = CVL(MID$(id2.argsize, i * 4 - 4 + 1, 4))
                IF id.tsize <> targettypsize THEN a$ = "Incorrect array type passed to sub": GOTO errmes
            END IF

            IF MID$(sfcmemargs(targetid), i, 1) = CHR$(1) THEN 'cmem required?
            IF cmemlist(idnum) = 0 THEN
                cmemlist(idnum) = 1
                recompile = 1
            END IF
        END IF

        IF id.linkid = 0 THEN
            'if id.linkid is 0, it means the number of array elements is definietly
            'known of the array being passed, this is not some "fake"/unknown array.
            'using the numer of array elements of a fake array would be dangerous!


            IF nelereq = 0 THEN
                'only continue if the number of array elements required is unknown
                'and it needs to be set

                IF id.arrayelements > 0 THEN '2009

                nelereq = id.arrayelements
                MID$(id2.nelereq, i, 1) = CHR$(nelereq)

            END IF

            'print rtrim$(id2.n)+">nelereq=";nelereq

            ids(targetid) = id2

        ELSE

            'the number of array elements required is known AND
            'the number of elements in the array to be passed is known

            IF id.arrayelements <> nelereq THEN a$ = "Passing arrays with a differing number of elements to a SUB/FUNCTION is not supported": GOTO errmes


        END IF
    END IF

    e$ = refer(e$, sourcetyp, 1)
    IF Error_Happened THEN GOTO errmes
    GOTO sete

END IF 'target is an array

'note: not an array...
'target is not an array

IF (targettyp AND ISSTRING) = 0 THEN
    IF (sourcetyp AND ISREFERENCE) THEN
        idnum = VAL(LEFT$(e$, INSTR(e$, sp3) - 1)) 'id# of sourcetyp

        targettyp2 = targettyp: sourcetyp2 = sourcetyp

        'get info about source/target
        arr = 0: IF (sourcetyp2 AND ISARRAY) THEN arr = 1
        passudtelement = 0: IF (targettyp2 AND ISUDT) = 0 AND (sourcetyp2 AND ISUDT) <> 0 THEN passudtelement = 1: sourcetyp2 = sourcetyp2 - ISUDT

        'remove flags irrelevant for comparison... ISPOINTER,ISREFERENCE,ISINCONVENTIONALMEMORY,ISARRAY
        targettyp2 = targettyp2 AND (511 + ISOFFSETINBITS + ISUDT + ISFLOAT + ISSTRING)
        sourcetyp2 = sourcetyp2 AND (511 + ISOFFSETINBITS + ISUDT + ISFLOAT + ISSTRING)

        'compare types
        IF sourcetyp2 = targettyp2 THEN

            IF sourcetyp AND ISUDT THEN
                'udt/udt array

                'get info
                udtrefid = VAL(e$)
                getid udtrefid
                IF Error_Happened THEN GOTO errmes
                udtrefi = INSTR(e$, sp3) 'end of id
                udtrefi2 = INSTR(udtrefi + 1, e$, sp3) 'end of u
                udtrefu = VAL(MID$(e$, udtrefi + 1, udtrefi2 - udtrefi - 1))
                udtrefi3 = INSTR(udtrefi2 + 1, e$, sp3) 'skip e
                udtrefe = VAL(MID$(e$, udtrefi2 + 1, udtrefi3 - udtrefi2 - 1))
                o$ = RIGHT$(e$, LEN(e$) - udtrefi3)
                'note: most of the UDT info above is not required

                IF arr THEN
                    n$ = scope$ + "ARRAY_UDT_" + RTRIM$(id.n) + "[0]"
                ELSE
                    n$ = scope$ + "UDT_" + RTRIM$(id.n)
                END IF

                e$ = "(void*)( ((char*)(" + n$ + ")) + (" + o$ + ") )"

                'convert void* to target type*
                IF passudtelement THEN e$ = "(" + typ2ctyp$(targettyp2 + (targettyp AND ISUNSIGNED), "") + "*)" + e$
                IF Error_Happened THEN GOTO errmes

            ELSE
                'not a udt
                IF arr THEN
                    IF (sourcetyp2 AND ISOFFSETINBITS) THEN a$ = "Cannot pass BIT array offsets": GOTO errmes
                    e$ = "(&(" + refer(e$, sourcetyp, 0) + "))"
                    IF Error_Happened THEN GOTO errmes
                ELSE
                    e$ = refer(e$, sourcetyp, 1)
                    IF Error_Happened THEN GOTO errmes
                END IF

                'note: signed/unsigned mismatch requires casting
                IF (sourcetyp AND ISUNSIGNED) <> (targettyp AND ISUNSIGNED) THEN
                    e$ = "(" + typ2ctyp$(targettyp2 + (targettyp AND ISUNSIGNED), "") + "*)" + e$
                    IF Error_Happened THEN GOTO errmes
                END IF

            END IF 'udt?

            IF MID$(sfcmemargs(targetid), i, 1) = CHR$(1) THEN 'cmem required?
            IF cmemlist(idnum) = 0 THEN
                cmemlist(idnum) = 1
                recompile = 1
            END IF
        END IF

        GOTO sete
    END IF 'similar
END IF 'reference
ELSE 'not a string
    'its a string
    IF (sourcetyp AND ISREFERENCE) THEN
        idnum = VAL(LEFT$(e$, INSTR(e$, sp3) - 1)) 'id# of sourcetyp
        IF MID$(sfcmemargs(targetid), i, 1) = CHR$(1) THEN 'cmem required?
        IF cmemlist(idnum) = 0 THEN
            cmemlist(idnum) = 1
            recompile = 1
        END IF
    END IF
END IF 'reference
END IF 'its a string

END IF 'dereference check
END IF 'target is a pointer

'note: Target is not a pointer...

'String-numeric mismatch?
IF targettyp AND ISSTRING THEN
    IF (sourcetyp AND ISSTRING) = 0 THEN
        nth = i
        IF ids(targetid).args = 1 THEN a$ = "String required for sub": GOTO errmes
        a$ = str_nth$(nth) + " sub argument requires a string": GOTO errmes
    END IF
END IF
IF (targettyp AND ISSTRING) = 0 THEN
    IF sourcetyp AND ISSTRING THEN
        nth = i
        IF ids(targetid).args = 1 THEN a$ = "Number required for sub": GOTO errmes
        a$ = str_nth$(nth) + " sub argument requires a number": GOTO errmes
    END IF
END IF

'change to "non-pointer" value
IF (sourcetyp AND ISREFERENCE) THEN
    e$ = refer(e$, sourcetyp, 0)
    IF Error_Happened THEN GOTO errmes
END IF

IF explicitreference = 0 THEN
    IF targettyp AND ISUDT THEN
        nth = i
        IF qbnexprefix_set AND udtxcname(targettyp AND 511) = "_MEM" THEN
            x$ = "'" + MID$(RTRIM$(udtxcname(targettyp AND 511)), 2) + "'"
        ELSE
            x$ = "'" + RTRIM$(udtxcname(targettyp AND 511)) + "'"
        END IF
        IF ids(targetid).args = 1 THEN a$ = "TYPE " + x$ + " required for sub": GOTO errmes
        a$ = str_nth$(nth) + " sub argument requires TYPE " + x$: GOTO errmes
    END IF
ELSE
    IF sourcetyp AND ISUDT THEN a$ = "Number required for sub": GOTO errmes
END IF

'round to integer if required
IF (sourcetyp AND ISFLOAT) THEN
    IF (targettyp AND ISFLOAT) = 0 THEN
        '**32 rounding fix
        bits = targettyp AND 511
        IF bits <= 16 THEN e$ = "qbr_float_to_long(" + e$ + ")"
        IF bits > 16 AND bits < 32 THEN e$ = "qbr_double_to_long(" + e$ + ")"
        IF bits >= 32 THEN e$ = "qbr(" + e$ + ")"
    END IF
END IF

IF (targettyp AND ISPOINTER) THEN 'pointer required
IF (targettyp AND ISSTRING) THEN GOTO sete 'no changes required
t$ = typ2ctyp$(targettyp, "")
IF Error_Happened THEN GOTO errmes
v$ = "pass" + str2$(uniquenumber)
'assume numeric type
IF MID$(sfcmemargs(targetid), i, 1) = CHR$(1) THEN 'cmem required?
bytesreq = ((targettyp AND 511) + 7) \ 8
PRINT #defdatahandle, t$ + " *" + v$ + "=NULL;"
PRINT #13, "if(" + v$ + "==NULL){"
PRINT #13, "cmem_sp-=" + str2(bytesreq) + ";"
PRINT #13, v$ + "=(" + t$ + "*)(dblock+cmem_sp);"
PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
PRINT #13, "}"
e$ = "&(*" + v$ + "=" + e$ + ")"
ELSE
    PRINT #13, t$ + " " + v$ + ";"
    e$ = "&(" + v$ + "=" + e$ + ")"
END IF
GOTO sete
END IF

END IF 'not "NULL"

sete:

IF RTRIM$(id2.callname) = "sub_paint" THEN
    IF i = 3 THEN
        IF (sourcetyp AND ISSTRING) THEN
            e$ = "(qbs*)" + e$
        ELSE
            e$ = "(uint32)" + e$
        END IF
    END IF
END IF

IF id2.ccall THEN

    'if a forced cast from a returned ccall function is in e$, remove it
    IF LEFT$(e$, 3) = "(  " THEN
        e$ = removecast$(e$)
    END IF

    IF targettyp AND ISSTRING THEN
        e$ = "(char*)(" + e$ + ")->chr"
    END IF

    IF LTRIM$(RTRIM$(e$)) = "0" THEN e$ = "NULL"

END IF

IF i <> 1 THEN subcall$ = subcall$ + ","
subcall$ = subcall$ + e$
NEXT

'note: i=id.args+1
x$ = separgslayout_local$(i)
DO WHILE LEN(x$)
    x = ASC(x$)
    IF x THEN
        x2$ = MID$(x$, 2, x)
        x$ = RIGHT$(x$, LEN(x$) - x - 1)

        s = 0
        an = 0
        x3$ = RIGHT$(l$, 1)
        IF x3$ = sp THEN s = 1
        IF x3$ = sp2 THEN
            s = 2
            IF alphanumeric(ASC(RIGHT$(l$, 2))) THEN an = 1
            'if asc(right$(l$,2))=34 then an=1
        ELSE
            IF alphanumeric(ASC(x3$)) THEN an = 1
            'if asc(x3$)=34 then an=1
        END IF
        s1 = s

        IF (an = 1 OR addedlayout = 1) AND alphanumeric(ASC(x2$)) <> 0 THEN
            s = 1 'force space
            x2$ = x2$ + sp2
            GOTO customlaychar2
        END IF

        IF x2$ = "=" THEN
            s = 1
            x2$ = x2$ + sp
            GOTO customlaychar2
        END IF

        IF x2$ = "#" THEN
            s = 1
            x2$ = x2$ + sp2
            GOTO customlaychar2
        END IF

        IF x2$ = "," THEN x2$ = x2$ + sp: GOTO customlaychar2

        IF x$ = CHR$(0) THEN 'substitution
        IF x2$ = "STEP" THEN x2$ = SCase$("Step") + sp2: GOTO customlaychar2
        x2$ = x2$ + sp: GOTO customlaychar2
    END IF

    'default solution sp2+?+sp2
    x2$ = x2$ + sp2
    customlaychar2:
    IF s = 0 THEN s = 2
    IF s <> s1 THEN
        IF s1 THEN l$ = LEFT$(l$, LEN(l$) - 1)
        IF s = 1 THEN l$ = l$ + sp
        IF s = 2 THEN l$ = l$ + sp2
    END IF
    l$ = l$ + x2$

ELSE
    addlayout = 0
    x$ = RIGHT$(x$, LEN(x$) - 1)
END IF
addedlayout = 0
LOOP






IF passedneeded THEN
    subcall$ = subcall$ + "," + str2$(passed&)
END IF
subcall$ = subcall$ + ");"

IF firstelement$ = "SLEEP" THEN
    IF vWatchOn = 1 THEN
        PRINT #12, "*__LONG_VWATCH_LINENUMBER= -4; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);"
    END IF
END IF

PRINT #12, subcall$

IF firstelement$ = "SLEEP" THEN
    IF vWatchOn = 1 THEN
        PRINT #12, "*__LONG_VWATCH_LINENUMBER= -5; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);"
    END IF
END IF

subcall$ = ""
IF stringprocessinghappened THEN PRINT #12, cleanupstringprocessingcall$ + "0);"

layoutdone = 1
x$ = RIGHT$(l$, 1): IF x$ = sp OR x$ = sp2 THEN l$ = LEFT$(l$, LEN(l$) - 1)
IF usecall = 1 THEN l$ = l$ + sp2 + ")"
IF Debug THEN PRINT #9, "SUB layout:[" + l$ + "]"
IF LEN(layout$) = 0 THEN layout$ = l$ ELSE layout$ = layout$ + sp + l$
GOTO finishedline


END IF

IF try = 2 THEN
    findidsecondarg = "": IF n >= 2 THEN findidsecondarg = getelement$(a$, 2)
    findanotherid = 1
    try = findid(firstelement$)
    IF Error_Happened THEN GOTO errmes
ELSE
    try = 0
END IF
LOOP

END IF

notsubcall:

IF n >= 1 THEN
    IF firstelement$ = "LET" THEN
        IF n = 1 THEN a$ = "Syntax error - Reference: LET variable = expression (tip: LET is entirely optional)": GOTO errmes
        ca$ = RIGHT$(ca$, LEN(ca$) - 4)
        n = n - 1
        l$ = SCase$("Let")
        IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
        'note: layoutdone=1 will be set later
        GOTO letused
    END IF
END IF

'LET ???=???
IF n >= 3 THEN
    IF INSTR(a$, sp + "=" + sp) THEN
        letused:
        assign ca$, n
        IF Error_Happened THEN GOTO errmes
        layoutdone = 1
        IF LEN(layout$) = 0 THEN layout$ = tlayout$ ELSE layout$ = layout$ + sp + tlayout$
        GOTO finishedline
    END IF
END IF '>=3
IF RIGHT$(a$, 2) = sp + "=" THEN a$ = "Expected ... = expression": GOTO errmes

'Syntax error
a$ = "Syntax error": GOTO errmes

finishedline:
THENGOTO = 0
finishedline2:

IF inputfunctioncalled THEN
    inputfunctioncalled = 0
    IF vWatchOn = 1 THEN
        PRINT #12, "*__LONG_VWATCH_LINENUMBER= -5; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);"
    END IF
END IF

IF arrayprocessinghappened = 1 THEN arrayprocessinghappened = 0

inclinenump$ = ""
IF inclinenumber(inclevel) THEN
    inclinenump$ = "," + str2$(inclinenumber(inclevel))
    thisincname$ = getfilepath$(incname$(inclevel))
    thisincname$ = MID$(incname$(inclevel), LEN(thisincname$) + 1)
    inclinenump$ = inclinenump$ + "," + CHR$(34) + thisincname$ + CHR$(34)
END IF
IF NoChecks = 0 THEN
    IF vWatchOn AND inclinenumber(inclevel) = 0 THEN temp$ = vWatchErrorCall$ ELSE temp$ = ""
    IF dynscope THEN
        dynscope = 0
        PRINT #12, "if(qbevent){" + temp$ + "evnt(" + str2$(linenumber) + inclinenump$ + ");if(r)goto S_" + str2$(statementn) + ";}"
    ELSE
        PRINT #12, "if(!qbevent)break;" + temp$ + "evnt(" + str2$(linenumber) + inclinenump$ + ");}while(r);"
    END IF
END IF

finishednonexec:

firstLine = 0

IF layoutdone = 0 THEN layoutok = 0 'invalidate layout if not handled

IF continuelinefrom = 0 THEN 'note: manager #2 requires this condition

'Include Manager #2 '***
IF LEN(addmetainclude$) THEN

    IF inclevel = 0 THEN
        'backup line formatting
        layoutcomment_backup$ = layoutcomment$
        layoutok_backup = layoutok
        layout_backup$ = layout$
    END IF

    a$ = addmetainclude$: addmetainclude$ = "" 'read/clear message

    IF inclevel = 0 THEN
        includingFromRoot = 0
        forceIncludingFile = 0
        forceInclude:
        IF forceIncludeFromRoot$ <> "" THEN
            a$ = forceIncludeFromRoot$
            forceIncludeFromRoot$ = ""
            forceIncludingFile = 1
            includingFromRoot = 1
        END IF
    END IF

    IF inclevel = 100 THEN a$ = "Too many indwelling INCLUDE files": GOTO errmes
    '1. Verify file exists (location is either (a)relative to source file or (b)absolute)
    fh = 99 + inclevel + 1

    firstTryMethod = 1
    IF includingFromRoot <> 0 AND inclevel = 0 THEN firstTryMethod = 2
    FOR try = firstTryMethod TO 2 'if including file from root, do not attempt including from relative location
        IF try = 1 THEN
            IF inclevel = 0 THEN
                p$ = getfilepath$(sourcefile$)
            ELSE
                p$ = getfilepath$(incname(inclevel))
            END IF
            f$ = p$ + a$
        END IF
        IF try = 2 THEN f$ = a$
        IF _FILEEXISTS(f$) THEN
            qberrorhappened = -2 '***
            OPEN f$ FOR BINARY AS #fh
            qberrorhappened2: '***
            IF qberrorhappened = -2 THEN EXIT FOR '***
        END IF
        qberrorhappened = 0
    NEXT
    IF qberrorhappened <> -2 THEN qberrorhappened = 0: a$ = "File " + a$ + " not found": GOTO errmes
    inclevel = inclevel + 1: incname$(inclevel) = f$: inclinenumber(inclevel) = 0
END IF 'fall through to next section...
'--------------------
DO WHILE inclevel
    fh = 99 + inclevel
    '2. Feed next line
    IF LEN(classSyntaxQueue$) THEN
        a3$ = ClassSyntax_DequeueLine$
        a3$ = TopLevelRuntime_ProcessLine$(a3$)
        continuelinefrom = 0
        linenumber = linenumber - 1 'lower official linenumber to counter later increment
        GOTO includeline
    END IF
    IF EOF(fh) = 0 THEN
        LINE INPUT #fh, x$
        a3$ = ClassSyntax_ProcessLine$(x$)
        a3$ = TopLevelRuntime_ProcessLine$(a3$)
        continuelinefrom = 0
        inclinenumber(inclevel) = inclinenumber(inclevel) + 1
        'create extended error string 'incerror$'
        errorLineInInclude = inclinenumber(inclevel)
        e$ = " in line " + str2(inclinenumber(inclevel)) + " of " + incname$(inclevel) + " included"
        IF inclevel > 1 THEN
            e$ = e$ + " (through "
            FOR x = 1 TO inclevel - 1 STEP 1
                e$ = e$ + incname$(x)
                IF x < inclevel - 1 THEN 'a sep is req
                IF x = inclevel - 2 THEN
                    e$ = e$ + " then "
                ELSE
                    e$ = e$ + ", "
                END IF
            END IF
        NEXT
        e$ = e$ + ")"
    END IF
    incerror$ = e$
    linenumber = linenumber - 1 'lower official linenumber to counter later increment
    GOTO includeline
END IF
IF LEN(classSyntaxDeferredQueue$) THEN
    a3$ = ClassSyntax_DequeueDeferredLine$
    continuelinefrom = 0
    linenumber = linenumber - 1
    GOTO includeline
END IF
'3. Close & return control
CLOSE #fh
inclevel = inclevel - 1
IF inclevel = 0 THEN
    IF forceIncludingFile = 1 THEN
        forceIncludingFile = 0
        GOTO forceIncludeCompleted
    END IF
    'restore line formatting
    layoutok = layoutok_backup
    layout$ = layout_backup$
    layoutcomment$ = layoutcomment_backup$
END IF
LOOP 'fall through to next section...
'(end manager)



END IF 'continuelinefrom=0


IF Debug THEN
    PRINT #9, "[layout check]"
    PRINT #9, "[" + layoutoriginal$ + "]"
    PRINT #9, "[" + layout$ + "]"
    PRINT #9, layoutok
    PRINT #9, "[end layout check]"
END IF




'layout is not currently used by the compiler, if it was it would be used here
nextmainpassline:
LOOP

'add final line
IF lastLineReturn = 0 THEN
    lastLineReturn = 1
    lastLine = 1
    wholeline$ = ""
    GOTO mainpassLastLine
END IF

IF NOT QuietMode THEN
    IF percentage <> 100 THEN
        percentage = 100
        UpdateCompilerProgress percentage
    END IF
    FinishCompilerProgress
END IF

linenumber = 0

IF closedmain = 0 THEN closemain

IF definingtype THEN linenumber = definingtypeerror: a$ = "TYPE without END TYPE": GOTO errmes

'check for open controls (copy #1)
IF controllevel THEN
    a$ = "Unidentified open control block"
    SELECT CASE controltype(controllevel)
    CASE 1: a$ = "IF without END IF"
    CASE 2: a$ = "FOR without NEXT"
    CASE 3, 4: a$ = "DO without LOOP"
    CASE 5: a$ = "WHILE without WEND"
    CASE 6: a$ = "$IF without $END IF"
    CASE 10 TO 19: a$ = "SELECT CASE without END SELECT"
    CASE 32: a$ = "SUB/FUNCTION without END SUB/FUNCTION"
    END SELECT
    linenumber = controlref(controllevel)
    GOTO errmes
END IF

IF ideindentsubs = 0 THEN
    IF LEN(subfunc) THEN a$ = "SUB/FUNCTION without END SUB/FUNCTION": GOTO errmes
END IF

'close the error handler (cannot be put in 'closemain' because subs/functions can also add error jumps to this file)
PRINT #14, "exit(99);" 'in theory this line should never be run!
PRINT #14, "}" 'close error jump handler

'create CLEAR method "CLEAR"
CLOSE #12 'close code handle
CALL TopLevelRuntime_InjectMainHook(tmpdir$ + "main.txt")
OPEN tmpdir$ + "clear.txt" FOR OUTPUT AS #12 'direct code to clear.txt

FOR i = 1 TO idn

    IF ids(i).staticscope THEN 'static scope?
    subfunc = RTRIM$(ids(i).insubfunc) 'set static scope
    GOTO clearstaticscope
END IF

a = ASC(ids(i).insubfunc)
IF a = 0 OR a = 32 THEN 'global scope?
subfunc = "" 'set global scope
clearstaticscope:

IF ids(i).arraytype THEN 'an array
getid i
IF Error_Happened THEN GOTO errmes
IF id.arrayelements = -1 THEN GOTO clearerasereturned 'cannot erase non-existant array
IF INSTR(vWatchVariableExclusions$, "@" + RTRIM$(id.callname) + "@") > 0 THEN
    GOTO clearerasereturned
END IF
clearerasereturn = 1: GOTO clearerase
END IF 'array

IF ids(i).t THEN 'non-array variable
getid i
IF Error_Happened THEN GOTO errmes
bytes$ = variablesize$(-1)
IF Error_Happened THEN GOTO errmes
'create a reference
typ = id.t + ISREFERENCE
IF typ AND ISUDT THEN
    e$ = str2(i) + sp3 + str2(typ AND 511) + sp3 + "0" + sp3 + "0"
ELSE
    e$ = str2(i)
END IF
e$ = refer$(e$, typ, 1)
IF Error_Happened THEN GOTO errmes
IF typ AND ISSTRING THEN
    IF typ AND ISFIXEDLENGTH THEN
        PRINT #12, "memset((void*)(" + e$ + "->chr),0," + bytes$ + ");"
        GOTO cleared
    ELSE
        IF INSTR(vWatchVariableExclusions$, "@" + e$ + "@") = 0 AND LEFT$(e$, 12) <> "_SUB_VWATCH_" THEN
            PRINT #12, e$ + "->len=0;"
        END IF
        GOTO cleared
    END IF
END IF
IF typ AND ISUDT THEN
    IF udtxvariable(typ AND 511) THEN
        'this next procedure resets values of UDT variables with variable-length strings
        clear_udt_with_varstrings e$, typ AND 511, 12, 0
    ELSE
        PRINT #12, "memset((void*)" + e$ + ",0," + bytes$ + ");"
    END IF
ELSE
    IF INSTR(vWatchVariableExclusions$, "@" + e$ + "@") = 0 AND LEFT$(e$, 12) <> "_SUB_VWATCH_" THEN
        PRINT #12, "*" + e$ + "=0;"
    END IF
END IF
GOTO cleared
END IF 'non-array variable

END IF 'scope

cleared:
clearerasereturned:
NEXT
CLOSE #12

IF Debug THEN
    PRINT #9, "finished making program!"
    PRINT #9, "recompile="; recompile
END IF

'Set cmem flags for subs/functions requiring data passed in cmem
FOR i = 1 TO idn
    IF cmemlist(i) THEN 'must be in cmem

    getid i
    IF Error_Happened THEN GOTO errmes

    IF Debug THEN PRINT #9, "recompiling cmem sf! checking:"; RTRIM$(id.n)

    IF id.sfid THEN 'it is an argument of a sub/function

    IF Debug THEN PRINT #9, "recompiling cmem sf! It's a sub/func arg!"

    i2 = id.sfid
    x = id.sfarg

    IF Debug THEN PRINT #9, "recompiling cmem sf! values:"; i2; x

    'check if cmem flag is set, if not then set it & force recompile
    IF MID$(sfcmemargs(i2), x, 1) <> CHR$(1) THEN
        MID$(sfcmemargs(i2), x, 1) = CHR$(1)


        IF Debug THEN PRINT #9, "recompiling cmem sf! setting:"; i2; x


        recompile = 1
    END IF
END IF
END IF
NEXT i

unresolved = 0
FOR i = 1 TO idn
    getid i
    IF Error_Happened THEN GOTO errmes

    IF Debug THEN PRINT #9, "checking id named:"; id.n

    IF id.subfunc THEN
        FOR i2 = 1 TO id.args
            t = CVL(MID$(id.arg, i2 * 4 - 3, 4))
            IF t > 0 THEN
                IF (t AND ISPOINTER) THEN
                    IF (t AND ISARRAY) THEN

                        IF Debug THEN PRINT #9, "checking argument "; i2; " of "; id.args

                        nele = ASC(MID$(id.nele, i2, 1))
                        nelereq = ASC(MID$(id.nelereq, i2, 1))

                        IF Debug THEN PRINT #9, "nele="; nele
                        IF Debug THEN PRINT #9, "nelereq="; nelereq

                        IF nele <> nelereq THEN

                            IF Debug THEN PRINT #9, "mismatch detected!"

                            unresolved = unresolved + 1
                            sflistn = sflistn + 1
                            sfidlist(sflistn) = i
                            sfarglist(sflistn) = i2
                            sfelelist(sflistn) = nelereq '0 means still unknown
                        END IF
                    END IF
                END IF
            END IF
        NEXT
    END IF
NEXT

'is recompilation required to resolve this?
IF unresolved > 0 THEN
    IF lastunresolved = -1 THEN
        'first pass
        recompile = 1
        IF Debug THEN
            PRINT #9, "recompiling to resolve array elements (first time)"
            PRINT #9, "sflistn="; sflistn
            PRINT #9, "oldsflistn="; oldsflistn
        END IF
    ELSE
        'not first pass
        IF unresolved < lastunresolved THEN
            recompile = 1
            IF Debug THEN
                PRINT #9, "recompiling to resolve array elements (not first time)"
                PRINT #9, "sflistn="; sflistn
                PRINT #9, "oldsflistn="; oldsflistn
            END IF
        END IF
    END IF
END IF 'unresolved
lastunresolved = unresolved

'IDEA!
'have a flag to record if anything gets resolved in a pass
'if not then it's time to stop
'the problem is the same amount of new problems may be created by a
'resolve as those that get fixed
'also/or.. could it be that previous fixes are overridden in a recompile
'          by a new fix? if so, it would give these effects



'could recompilation resolve this?
'IF sflistn <> -1 THEN
'IF sflistn <> oldsflistn THEN
'recompile = 1
'
'if debug then
'print #9,"recompile set to 1 to resolve array elements"
'print #9,"sflistn=";sflistn
'print #9,"oldsflistn=";oldsflistn
'end if
'
'END IF
'END IF

IF Debug THEN PRINT #9, "Beginning COMMON array list check..."
xi = 1
FOR x = 1 TO commonarraylistn
    varname$ = getelement$(commonarraylist, xi): xi = xi + 1
    typ$ = getelement$(commonarraylist, xi): xi = xi + 1
    dimmethod2 = VAL(getelement$(commonarraylist, xi)): xi = xi + 1
    dimshared2 = VAL(getelement$(commonarraylist, xi)): xi = xi + 1
    'find the array ID (try method)
    t = typname2typ(typ$)
    IF Error_Happened THEN GOTO errmes
    IF (t AND ISUDT) = 0 THEN varname$ = varname$ + type2symbol$(typ$)
    IF Error_Happened THEN GOTO errmes

    IF Debug THEN PRINT #9, "Checking for array '" + varname$ + "'..."

    try = findid(varname$)
    IF Error_Happened THEN GOTO errmes
    DO WHILE try
        IF id.arraytype THEN GOTO foundcommonarray2
        IF try = 2 THEN findanotherid = 1: try = findid(varname$) ELSE try = 0
        IF Error_Happened THEN GOTO errmes
    LOOP
    foundcommonarray2:

    IF Debug THEN PRINT #9, "Found array '" + varname$ + "!"

    IF id.arrayelements = -1 THEN
        IF arrayelementslist(currentid) <> 0 THEN recompile = 1
        IF Debug THEN PRINT #9, "Recompiling to resolve elements of:" + varname$
    END IF
NEXT
IF Debug THEN PRINT #9, "Finished COMMON array list check!"

IF vWatchDesiredState <> vWatchOn THEN
    vWatchRecompileAttempts = vWatchRecompileAttempts + 1
    recompile = 1
END IF

IF recompile THEN
    do_recompile:
    IF Debug THEN PRINT #9, "Recompile required!"
    recompile = 0
    FOR closeall = 1 TO 255: CLOSE closeall: NEXT
        OPEN tmpdir$ + "temp.bin" FOR OUTPUT LOCK WRITE AS #26 'relock
        GOTO recompile
    END IF

    labelValidationResult = ValidatePendingLabels%
    IF labelValidationResult = 1 THEN GOTO do_recompile
    IF labelValidationResult = 2 THEN GOTO errmes


    'if targettyp=-4 or targettyp=-5 then '? -> byte_element(offset,element size in bytes)
    ' IF (sourcetyp AND ISREFERENCE) = 0 THEN a$ = "Expected variable name/array element": GOTO errmes


    'create include files for COMMON arrays

    CLOSE #12

    'return to 'main'
    subfunc$ = ""
    defdatahandle = 18
    CLOSE #13: OPEN tmpdir$ + "maindata.txt" FOR APPEND AS #13
    CLOSE #19: OPEN tmpdir$ + "mainfree.txt" FOR APPEND AS #19

    IF Console THEN
        PRINT #18, "int32 console=1;"
    ELSE
        PRINT #18, "int32 console=0;"
    END IF

    IF ScreenHide THEN
        PRINT #18, "int32 screen_hide_startup=1;"
    ELSE
        PRINT #18, "int32 screen_hide_startup=0;"
    END IF

    IF Asserts THEN
        PRINT #18, "int32 asserts=1;"
    ELSE
        PRINT #18, "int32 asserts=0;"
    END IF

    IF vWatchOn THEN
        PRINT #18, "int32 vwatch=-1;"
    ELSE
        PRINT #18, "int32 vwatch=0;"
    END IF

    fh = FREEFILE
    OPEN tmpdir$ + "dyninfo.txt" FOR APPEND AS #fh
    IF Resize THEN
        PRINT #fh, "ScreenResize=1;"
    END IF
    IF Resize_Scale THEN
        PRINT #fh, "ScreenResizeScale=" + str2(Resize_Scale) + ";"
    END IF
    CLOSE #fh

    IF vWatchOn = 1 THEN
        vWatchVariable "", 1
    END IF


    'DATA_finalize
    PRINT #18, "ptrszint data_size=" + str2(DataOffset) + ";"
    IF DataOffset = 0 THEN

        PRINT #18, "uint8 *data=(uint8*)calloc(1,1);"

    ELSE

        IF inline_DATA = 0 THEN
            IF os$ = "WIN" THEN
                IF OS_BITS = 32 THEN
                    x$ = CHR$(0): PUT #16, , x$
                    PRINT #18, "extern " + CHR$(34) + "C" + CHR$(34) + "{"
                    PRINT #18, "extern char *binary_____temp" + tempfolderindexstr2$ + "__data_bin_start;"
                    PRINT #18, "}"
                    PRINT #18, "uint8 *data=(uint8*)&binary_____temp" + tempfolderindexstr2$ + "__data_bin_start;"
                ELSE
                    x$ = CHR$(0): PUT #16, , x$
                    PRINT #18, "extern " + CHR$(34) + "C" + CHR$(34) + "{"
                    PRINT #18, "extern char *_binary_____temp" + tempfolderindexstr2$ + "__data_bin_start;"
                    PRINT #18, "}"
                    PRINT #18, "uint8 *data=(uint8*)&_binary_____temp" + tempfolderindexstr2$ + "__data_bin_start;"
                END IF
            END IF
            IF os$ = "LNX" THEN
                x$ = CHR$(0): PUT #16, , x$
                PRINT #18, "extern " + CHR$(34) + "C" + CHR$(34) + "{"
                PRINT #18, "extern char *_binary____temp" + tempfolderindexstr2$ + "_data_bin_start;"
                PRINT #18, "}"
                PRINT #18, "uint8 *data=(uint8*)&_binary____temp" + tempfolderindexstr2$ + "_data_bin_start;"
            END IF
        ELSE
            'inline data
            CLOSE #16
            ff = FREEFILE
            OPEN tmpdir$ + "data.bin" FOR BINARY AS #ff
            x$ = SPACE$(LOF(ff))
            GET #ff, , x$
            CLOSE #ff
            x2$ = "uint8 inline_data[]={"
            FOR i = 1 TO LEN(x$)
                x2$ = x2$ + inlinedatastr$(ASC(x$, i))
            NEXT
            x2$ = x2$ + "0};"
            PRINT #18, x2$
            PRINT #18, "uint8 *data=&inline_data[0];"
            x$ = "": x2$ = ""
        END IF
    END IF

    IF Debug THEN PRINT #9, "Beginning generation of code for saving/sharing common array data..."
    use_global_byte_elements = 1
    ncommontmp = 0
    xi = 1
    FOR x = 1 TO commonarraylistn
        varname$ = getelement$(commonarraylist, xi): xi = xi + 1
        typ$ = getelement$(commonarraylist, xi): xi = xi + 1
        dimmethod2 = VAL(getelement$(commonarraylist, xi)): xi = xi + 1
        dimshared2 = VAL(getelement$(commonarraylist, xi)): xi = xi + 1

        'find the array ID (try method)
        purevarname$ = varname$
        t = typname2typ(typ$)
        IF Error_Happened THEN GOTO errmes
        IF (t AND ISUDT) = 0 THEN varname$ = varname$ + type2symbol$(typ$)
        IF Error_Happened THEN GOTO errmes
        try = findid(varname$)
        IF Error_Happened THEN GOTO errmes
        DO WHILE try
            IF id.arraytype THEN GOTO foundcommonarray
            IF try = 2 THEN findanotherid = 1: try = findid(varname$) ELSE try = 0
            IF Error_Happened THEN GOTO errmes
        LOOP
        a$ = "COMMON array unlocatable": GOTO errmes 'should never happen
        foundcommonarray:
        IF Debug THEN PRINT #9, "Found common array '" + varname$ + "'!"

        i = currentid
        arraytype = id.arraytype
        arrayelements = id.arrayelements
        e$ = RTRIM$(id.n)
        IF (t AND ISUDT) = 0 THEN e$ = e$ + typevalue2symbol$(t)
        IF Error_Happened THEN GOTO errmes
        n$ = e$
        n2$ = RTRIM$(id.callname)
        tsize = id.tsize

        'select command
        command = 3 'fixed length elements
        IF t AND ISSTRING THEN
            IF (t AND ISFIXEDLENGTH) = 0 THEN
                command = 4 'var-len elements
            END IF
        END IF


        'if...
        'i) array elements are still undefined (ie. arrayelements=-1) pass the input content along
        '   if any existed or an array-placeholder
        'ii) if the array's elements were defined, any input content would have been loaded so the
        '    array (in whatever state it currently is) should be passed. If it is currently erased
        '    then it should be passed as a placeholder

        IF arrayelements = -1 THEN

            'load array (copies the array, if any, into a buffer for later)



            OPEN tmpdir$ + "inpchain" + str2$(i) + ".txt" FOR OUTPUT AS #12
            PRINT #12, "if (int32val==2){" 'array place-holder
            'create buffer to store array as-is in global.txt
            x$ = str2$(uniquenumber)
            x1$ = "chainarraybuf" + x$
            x2$ = "chainarraybufsiz" + x$
            PRINT #18, "static uint8 *" + x1$ + "=(uint8*)malloc(1);"
            PRINT #18, "static int64 " + x2$ + "=0;"
            'read next command
            PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"

            IF command = 3 THEN PRINT #12, "if (int32val==3){" 'fixed-length-element array
            IF command = 4 THEN PRINT #12, "if (int32val==4){" 'var-length-element array
            PRINT #12, x2$ + "+=4; " + x1$ + "=(uint8*)realloc(" + x1$ + "," + x2$ + "); *(int32*)(" + x1$ + "+" + x2$ + "-4)=int32val;"

            IF command = 3 THEN
                'read size in bits of one element, convert it to bytes
                PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);"
                PRINT #12, x2$ + "+=8; " + x1$ + "=(uint8*)realloc(" + x1$ + "," + x2$ + "); *(int64*)(" + x1$ + "+" + x2$ + "-8)=int64val;"
                PRINT #12, "bytes=int64val>>3;"
            END IF 'com=3

            IF command = 4 THEN PRINT #12, "bytes=1;" 'bytes used to calculate number of elements

            'read number of dimensions
            PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"
            PRINT #12, x2$ + "+=4; " + x1$ + "=(uint8*)realloc(" + x1$ + "," + x2$ + "); *(int32*)(" + x1$ + "+" + x2$ + "-4)=int32val;"

            'read size of dimensions & calculate the size of the array in bytes
            PRINT #12, "while(int32val--){"
            PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);" 'lbound
            PRINT #12, x2$ + "+=8; " + x1$ + "=(uint8*)realloc(" + x1$ + "," + x2$ + "); *(int64*)(" + x1$ + "+" + x2$ + "-8)=int64val;"
            PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int64val2,8," + NewByteElement$ + "),0);" 'ubound
            PRINT #12, x2$ + "+=8; " + x1$ + "=(uint8*)realloc(" + x1$ + "," + x2$ + "); *(int64*)(" + x1$ + "+" + x2$ + "-8)=int64val2;"
            PRINT #12, "bytes*=(int64val2-int64val+1);"
            PRINT #12, "}"

            IF command = 3 THEN
                'read the array data
                PRINT #12, x2$ + "+=bytes; " + x1$ + "=(uint8*)realloc(" + x1$ + "," + x2$ + ");"
                PRINT #12, "sub_get(FF,NULL,byte_element((uint64)(" + x1$ + "+" + x2$ + "-bytes),bytes," + NewByteElement$ + "),0);"
            END IF 'com=3

            IF command = 4 THEN
                PRINT #12, "bytei=0;"
                PRINT #12, "while(bytei<bytes){"
                PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);" 'get size
                PRINT #12, x2$ + "+=8; " + x1$ + "=(uint8*)realloc(" + x1$ + "," + x2$ + "); *(int64*)(" + x1$ + "+" + x2$ + "-8)=int64val;"
                PRINT #12, x2$ + "+=(int64val>>3); " + x1$ + "=(uint8*)realloc(" + x1$ + "," + x2$ + ");"
                PRINT #12, "sub_get(FF,NULL,byte_element((uint64)(" + x1$ + "+" + x2$ + "-(int64val>>3)),(int64val>>3)," + NewByteElement$ + "),0);"
                PRINT #12, "bytei++;"
                PRINT #12, "}"
            END IF

            'get next command
            PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"
            PRINT #12, "}" 'command=3 or 4

            PRINT #12, "}" 'array place-holder
            CLOSE #12


            'save array (saves the buffered data, if any, for later)

            OPEN tmpdir$ + "chain" + str2$(i) + ".txt" FOR OUTPUT AS #12
            PRINT #12, "int32val=2;" 'placeholder
            PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"

            PRINT #12, "sub_put(FF,NULL,byte_element((uint64)" + x1$ + "," + x2$ + "," + NewByteElement$ + "),0);"
            CLOSE #12




        ELSE
            'note: arrayelements<>-1

            'load array

            OPEN tmpdir$ + "inpchain" + str2$(i) + ".txt" FOR OUTPUT AS #12

            PRINT #12, "if (int32val==2){" 'array place-holder
            PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"

            IF command = 3 THEN PRINT #12, "if (int32val==3){" 'fixed-length-element array
            IF command = 4 THEN PRINT #12, "if (int32val==4){" 'var-length-element array

            IF command = 3 THEN
                'get size in bits
                PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);"
                '***assume correct***
            END IF

            'get number of elements
            PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"
            '***assume correct***

            e$ = ""
            IF command = 4 THEN PRINT #12, "bytes=1;" 'bytes counts the number of total elements
            FOR x2 = 1 TO arrayelements

                'create 'secret' variables to assist in passing common arrays
                IF x2 > ncommontmp THEN
                    ncommontmp = ncommontmp + 1

                    IF Debug THEN PRINT #9, "Calling DIM2(...)..."
                    IF Error_Happened THEN GOTO errmes
                    retval = dim2("___RESERVED_COMMON_LBOUND" + str2$(ncommontmp), "_INTEGER64", 0, "")
                    IF Error_Happened THEN GOTO errmes
                    retval = dim2("___RESERVED_COMMON_UBOUND" + str2$(ncommontmp), "_INTEGER64", 0, "")
                    IF Error_Happened THEN GOTO errmes
                    IF Debug THEN PRINT #9, "Finished calling DIM2(...)!"
                    IF Error_Happened THEN GOTO errmes


                END IF

                PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);"
                PRINT #12, "*__INTEGER64____RESERVED_COMMON_LBOUND" + str2$(x2) + "=int64val;"
                PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int64val2,8," + NewByteElement$ + "),0);"
                PRINT #12, "*__INTEGER64____RESERVED_COMMON_UBOUND" + str2$(x2) + "=int64val2;"
                IF command = 4 THEN PRINT #12, "bytes*=(int64val2-int64val+1);"
                IF x2 > 1 THEN e$ = e$ + sp + "," + sp
                e$ = e$ + "___RESERVED_COMMON_LBOUND" + str2$(x2) + sp + "TO" + sp + "___RESERVED_COMMON_UBOUND" + str2$(x2)
            NEXT

            IF Debug THEN PRINT #9, "Calling DIM2(" + purevarname$ + "," + typ$ + ",0," + e$ + ")..."
            IF Error_Happened THEN GOTO errmes
            'Note: purevarname$ is simply varname$ without the type symbol after it
            redimoption = 1
            retval = dim2(purevarname$, typ$, 0, e$)
            IF Error_Happened THEN GOTO errmes
            redimoption = 0
            IF Debug THEN PRINT #9, "Finished calling DIM2(" + purevarname$ + "," + typ$ + ",0," + e$ + ")!"
            IF Error_Happened THEN GOTO errmes

            IF command = 3 THEN
                'use get to load in the array data
                varname$ = varname$ + sp + "(" + sp + ")"
                e$ = evaluatetotyp(fixoperationorder$(varname$), -4)
                IF Error_Happened THEN GOTO errmes
                PRINT #12, "sub_get(FF,NULL," + e$ + ",0);"
            END IF

            IF command = 4 THEN
                PRINT #12, "bytei=0;"
                PRINT #12, "while(bytei<bytes){"
                PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);" 'get size
                PRINT #12, "tqbs=((qbs*)(((uint64*)(" + n2$ + "[0]))[bytei]));" 'get element
                PRINT #12, "qbs_set(tqbs,qbs_new(int64val>>3,1));" 'change string size
                PRINT #12, "sub_get(FF,NULL,byte_element((uint64)tqbs->chr,int64val>>3," + NewByteElement$ + "),0);" 'get size
                PRINT #12, "bytei++;"
                PRINT #12, "}"
            END IF

            'get next command
            PRINT #12, "sub_get(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"
            PRINT #12, "}"
            PRINT #12, "}"
            CLOSE #12

            'save array

            OPEN tmpdir$ + "chain" + str2$(i) + ".txt" FOR OUTPUT AS #12

            PRINT #12, "int32val=2;" 'placeholder
            PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"

            PRINT #12, "if (" + n2$ + "[2]&1){" 'don't add unless defined

            IF command = 3 THEN PRINT #12, "int32val=3;"
            IF command = 4 THEN PRINT #12, "int32val=4;"
            PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"

            IF command = 3 THEN
                'size of each element in bits
                bits = t AND 511
                IF t AND ISUDT THEN bits = udtxsize(t AND 511)
                IF t AND ISSTRING THEN bits = tsize * 8
                PRINT #12, "int64val=" + str2$(bits) + ";" 'size in bits
                PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);"
            END IF 'com=3

            PRINT #12, "int32val=" + str2$(arrayelements) + ";" 'number of dimensions
            PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int32val,4," + NewByteElement$ + "),0);"

            IF command = 3 THEN

                FOR x2 = 1 TO arrayelements
                    'simulate calls to lbound/ubound
                    e$ = "LBOUND" + sp + "(" + sp + n$ + sp + "," + sp + str2$(x2) + sp + ")"
                    e$ = evaluatetotyp(fixoperationorder$(e$), 64)
                    IF Error_Happened THEN GOTO errmes
                    PRINT #12, "int64val=" + e$ + ";"
                    PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);"
                    e$ = "UBOUND" + sp + "(" + sp + n$ + sp + "," + sp + str2$(x2) + sp + ")"
                    e$ = evaluatetotyp(fixoperationorder$(e$), 64)
                    IF Error_Happened THEN GOTO errmes
                    PRINT #12, "int64val=" + e$ + ";"
                    PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);"
                NEXT

                'array data
                e$ = evaluatetotyp(fixoperationorder$(n$ + sp + "(" + sp + ")"), -4)
                IF Error_Happened THEN GOTO errmes
                PRINT #12, "sub_put(FF,NULL," + e$ + ",0);"

            END IF 'com=3

            IF command = 4 THEN

                'store LBOUND/UBOUND values and calculate number of total elements/strings
                PRINT #12, "bytes=1;" 'note: bytes is actually the total number of elements
                FOR x2 = 1 TO arrayelements
                    e$ = "LBOUND" + sp + "(" + sp + n$ + sp + "," + sp + str2$(x2) + sp + ")"
                    e$ = evaluatetotyp(fixoperationorder$(e$), 64)
                    IF Error_Happened THEN GOTO errmes
                    PRINT #12, "int64val=" + e$ + ";"
                    PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);"
                    e$ = "UBOUND" + sp + "(" + sp + n$ + sp + "," + sp + str2$(x2) + sp + ")"
                    e$ = evaluatetotyp(fixoperationorder$(e$), 64)
                    IF Error_Happened THEN GOTO errmes
                    PRINT #12, "int64val2=" + e$ + ";"
                    PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int64val2,8," + NewByteElement$ + "),0);"
                    PRINT #12, "bytes*=(int64val2-int64val+1);"
                NEXT

                PRINT #12, "bytei=0;"
                PRINT #12, "while(bytei<bytes){"
                PRINT #12, "tqbs=((qbs*)(((uint64*)(" + n2$ + "[0]))[bytei]));" 'get element
                PRINT #12, "int64val=tqbs->len; int64val<<=3;"
                PRINT #12, "sub_put(FF,NULL,byte_element((uint64)&int64val,8," + NewByteElement$ + "),0);" 'size of element
                PRINT #12, "sub_put(FF,NULL,byte_element((uint64)tqbs->chr,tqbs->len," + NewByteElement$ + "),0);" 'element data
                PRINT #12, "bytei++;"
                PRINT #12, "}"

            END IF 'com=4

            PRINT #12, "}" 'don't add unless defined

            CLOSE #12




            'if chaincommonarray then
            'l2$=tlayout$
            'x=chaincommonarray
            '
            ''chain???.txt
            'open tmpdir$ + "chain" + str2$(x) + ".txt" for append as #22
            'if lof(22) then close #22: goto chaindone 'only add this once
            ''***assume non-var-len-string array***
            'print #22,"int32val=3;" 'non-var-len-element array
            'print #22,"sub_put(FF,NULL,byte_element((uint64)&int32val,4,"+NewByteElement$+"),0);"
            't=id.arraytype
            ''***check for UDT size if necessary***
            ''***check for string length if necessary***
            'bits=t and 511
            'print #22,"int64val="+str2$(bits)+";" 'size in bits
            'print #22,"sub_put(FF,NULL,byte_element((uint64)&int64val,8,"+NewByteElement$+"),0);"
            'print #22,"int32val="+str2$(id.arrayelements)+";" 'number of elements
            'print #22,"sub_put(FF,NULL,byte_element((uint64)&int32val,4,"+NewByteElement$+"),0);"
            'e$=rtrim$(id.n)
            'if (t and ISUDT)=0 then e$=e$+typevalue2symbol$(t)
            'n$=e$
            'for x2=1 to id.arrayelements
            ''simulate calls to lbound/ubound
            'e$="LBOUND"+sp+"("+sp+n$+sp+","+sp+str2$(x2)+sp+")"
            'e$=evaluatetotyp(fixoperationorder$(e$),64)
            'print #22,"int64val="+e$+";"'LBOUND
            'print #22,"sub_put(FF,NULL,byte_element((uint64)&int64val,8,"+NewByteElement$+"),0);"
            'e$="UBOUND"+sp+"("+sp+n$+sp+","+sp+str2$(x2)+sp+")"
            'e$=evaluatetotyp(fixoperationorder$(e$),64)
            'print #22,"int64val="+e$+";"'LBOUND
            'print #22,"sub_put(FF,NULL,byte_element((uint64)&int64val,8,"+NewByteElement$+"),0);"
            'next
            ''add array data
            'e$=evaluatetotyp(fixoperationorder$(n$+sp+"("+sp+")"),-4)
            'print #22,"sub_put(FF,NULL,"+e$+",0);"
            'close #22
            '
            ''inpchain???.txt
            'open tmpdir$ + "chain" + str2$(x) + ".txt" for append as #22
            'print #22,"if (int32val==1){" 'common declaration of an array
            'print #22,"sub_get(FF,NULL,byte_element((uint64)&int32val,4,"+NewByteElement$+"),0);"
            'print #22,"if (int32val==3){" 'fixed-length-element array
            '
            'print #22,"sub_get(FF,NULL,byte_element((uint64)&int64val,8,"+NewByteElement$+"),0);"
            ''***assume size correct and continue***
            '
            ''get number of elements
            'print #22,"sub_get(FF,NULL,byte_element((uint64)&int32val,4,"+NewByteElement$+"),0);"
            '
            ''call dim2 and tell it to redim an array
            '
            ''*********this should happen BEFORE the array (above) is actually dimensioned,
            ''*********where the common() declaration is
            '
            ''****although, if you never reference the array.............
            ''****ARGH! you can access an undimmed array just like in a sub/function
            '
            '
            '
            '
            'print #22,"}"
            'print #22,"}"
            'close #22
            '
            'chaindone:
            'tlayout$=l2$
            'end if 'chaincommonarray




            'OPEN tmpdir$ + "chain.txt" FOR APPEND AS #22
            ''include directive
            'print #22, "#include " + CHR$(34) + "chain" + str2$(x) + ".txt" + CHR$(34)
            'close #22
            ''create/clear include file
            'open tmpdir$ + "chain" + str2$(x) + ".txt" for output as #22:close #22
            '
            'OPEN tmpdir$ + "inpchain.txt" FOR APPEND AS #22
            ''include directive
            'print #22, "#include " + CHR$(34) + "inpchain" + str2$(x) + ".txt" + CHR$(34)
            'close #22
            ''create/clear include file
            'open tmpdir$ + "inpchain" + str2$(x) + ".txt" for output as #22:close #22






        END IF 'id.arrayelements=-1

    NEXT
    use_global_byte_elements = 0
    IF Debug THEN PRINT #9, "Finished generation of code for saving/sharing common array data!"


    FOR closeall = 1 TO 255: CLOSE closeall: NEXT
        OPEN tmpdir$ + "temp.bin" FOR OUTPUT LOCK WRITE AS #26 'relock
        InitializeCompilationLog

        ReportUnusedVariableWarnings

        IF No_C_Compile_Mode = 0 THEN
            IF PrepareExecutableOutputTarget%(file$) THEN GOTO errmes
        END IF


        IF os$ = "WIN" THEN
            IF PrepareWindowsResourceArtifacts%(file$) THEN GOTO errmes
        END IF

        PrepareDependencyBuildInputs defines$, libs$, libqb$, o$, win, lnx, mac

        IF RunNativeBuild%(file$, libqb$, libs$, defines$) THEN GOTO errmes
        FinalizeCompilerRun file$

        qberror_test:
        E = 1
        RESUME NEXT

        qberror:
        '_CONSOLE ON
        '_ECHO "A QB error has occurred (and you have compiled in debugging support)."
        '_ECHO "Some key information (qbnex.bas):"
        '_ECHO "Error" + STR$(ERR)
        '_ECHO "Description: " + _ERRORMESSAGE$
        '_ECHO "Line" + STR$(_ERRORLINE)
        'IF _INCLERRORLINE THEN
        '    _ECHO "Included line" + STR$(_INCLERRORLINE)
        '    _ECHO "Included file " + _INCLERRORFILE$
        'END IF
        '_ECHO ""
        '_ECHO "Loaded source file details:"
        '_ECHO "qberrorhappened =" + STR$(qberrorhappened) + "; qberrorhappenedvalue =" + STR$(qberrorhappenedvalue) + "; linenumber =" + STR$(linenumber)
        '_ECHO "ca$ = {" + ca$ + "}"
        '_ECHO "linefragment = {" + linefragment+ "}"

        IF Debug THEN ShowQbErrorDebugDisplay

        qberrorhappenedvalue = qberrorhappened
        qberrorhappened = 1

        LogQbErrorDetails

        IF qberrorhappenedvalue >= 0 THEN
            a$ = "UNEXPECTED INTERNAL COMPILER ERROR!": GOTO errmes 'internal comiler error
        END IF


        qberrorcode = ERR
        qberrorline = ERL
        IF qberrorhappenedvalue = -1 THEN RESUME qberrorhappened1
        IF qberrorhappenedvalue = -2 THEN RESUME qberrorhappened2
        IF qberrorhappenedvalue = -3 THEN RESUME qberrorhappened3
        END

        errmes: 'set a$ to message
        HandleFrontendErrorAndExit a$

        '$INCLUDE:'includes\frontend.bas'

        FUNCTION Type2MemTypeValue (t1)
            t = 0
            IF t1 AND ISARRAY THEN t = t + 65536
            IF t1 AND ISUDT THEN
                IF (t1 AND 511) = 1 THEN
                    t = t + 4096 '_MEM type
                ELSE
                    t = t + 32768
                END IF
            ELSE
                IF t1 AND ISSTRING THEN
                    t = t + 512 'string
                ELSE
                    IF t1 AND ISFLOAT THEN
                        t = t + 256 'float
                    ELSE
                        t = t + 128 'integer
                        IF t1 AND ISUNSIGNED THEN t = t + 1024
                        IF t1 AND ISOFFSET THEN t = t + 8192 'offset type
                    END IF
                    t1s = (t1 AND 511) \ 8
                    IF t1s = 1 THEN t = t + t1s
                    IF t1s = 2 THEN t = t + t1s
                    IF t1s = 4 THEN t = t + t1s
                    IF t1s = 8 THEN t = t + t1s
                    IF t1s = 16 THEN t = t + t1s
                    IF t1s = 32 THEN t = t + t1s
                    IF t1s = 64 THEN t = t + t1s
                END IF
            END IF
            Type2MemTypeValue = t
        END FUNCTION

        'udt is non-zero if this is an array of udt's, to allow examining each udt element
        '$INCLUDE:'includes\codegen.bas'



        SUB vWatchVariable (this$, action AS _BYTE)
            STATIC totalLocalVariables AS LONG, localVariablesList$
            STATIC totalMainModuleVariables AS LONG, mainModuleVariablesList$

            SELECT CASE action
            CASE -1 'reset
                totalLocalVariables = 0
                localVariablesList$ = ""
                totalMainModuleVariables = 0
                mainModuleVariablesList$ = ""
            CASE 0 'add
                IF INSTR(vWatchVariableExclusions$, "@" + this$ + "@") > 0 OR LEFT$(this$, 12) = "_SUB_VWATCH_" THEN
                    EXIT SUB
                END IF

                vWatchNewVariable$ = this$
                IF subfunc = "" THEN
                    totalMainModuleVariables = totalMainModuleVariables + 1
                    mainModuleVariablesList$ = mainModuleVariablesList$ + "vwatch_global_vars[" + str2$(totalMainModuleVariables - 1) + "] = &" + this$ + ";" + CRLF
                    manageVariableList id.cn, this$, totalMainModuleVariables - 1, 0
                ELSE
                    totalLocalVariables = totalLocalVariables + 1
                    localVariablesList$ = localVariablesList$ + "vwatch_local_vars[" + str2$(totalLocalVariables - 1) + "] = &" + this$ + ";" + CRLF
                    manageVariableList id.cn, this$, totalLocalVariables - 1, 0
                END IF
            CASE 1 'dump to data[].txt & reset
                IF subfunc = "" THEN
                    IF totalMainModuleVariables > 0 THEN
                        PRINT #13, "void *vwatch_local_vars[0];"
                        PRINT #18, "void *vwatch_global_vars["; totalMainModuleVariables; "];"
                        PRINT #13, mainModuleVariablesList$
                    ELSE
                        PRINT #13, "void *vwatch_local_vars[0];"
                        PRINT #18, "void *vwatch_global_vars[0];"
                    END IF

                    mainModuleVariablesList$ = ""
                    totalMainModuleVariables = 0
                ELSE
                    IF subfunc <> "SUB_VWATCH" THEN
                        IF totalLocalVariables > 0 THEN
                            PRINT #13, "void *vwatch_local_vars["; (totalLocalVariables); "];"
                            PRINT #13, localVariablesList$
                        ELSE
                            PRINT #13, "void *vwatch_local_vars[0];"
                        END IF
                    ELSE
                        PRINT #13, "void *vwatch_local_vars[0];"
                    END IF

                    localVariablesList$ = ""
                    totalLocalVariables = 0
                END IF
            END SELECT
        END SUB

        SUB vWatchAddLabel (this AS LONG, lastLine AS _BYTE)
            STATIC prevLabel AS LONG, prevSkip AS LONG

            IF lastLine = 0 THEN
                WHILE this > LEN(vWatchUsedLabels)
                    vWatchUsedLabels = vWatchUsedLabels + SPACE$(1000)
                    vWatchUsedSkipLabels = vWatchUsedSkipLabels + SPACE$(1000)
                WEND

                IF firstLineNumberLabelvWatch = 0 THEN
                    firstLineNumberLabelvWatch = this
                ELSE
                    IF prevSkip <> prevLabel THEN
                        ASC(vWatchUsedSkipLabels, prevLabel) = 1
                        PRINT #12, "VWATCH_SKIPLABEL_" + str2$(prevLabel) + ":;"
                        prevSkip = prevLabel
                    END IF
                END IF

                IF prevLabel <> this THEN
                    ASC(vWatchUsedLabels, this) = 1
                    PRINT #12, "VWATCH_LABEL_" + str2$(this) + ":;"
                    prevLabel = this
                    lastLineNumberLabelvWatch = this
                END IF
            ELSE
                IF prevSkip <> prevLabel THEN
                    ASC(vWatchUsedSkipLabels, prevLabel) = 1
                    PRINT #12, "VWATCH_SKIPLABEL_" + str2$(prevLabel) + ":;"
                    prevSkip = prevLabel
                END IF
            END IF
        END SUB

        SUB closemain
            xend

            PRINT #12, "return;"

            IF vWatchOn AND firstLineNumberLabelvWatch > 0 THEN
                PRINT #12, "VWATCH_SETNEXTLINE:;"
                PRINT #12, "switch (*__LONG_VWATCH_GOTO) {"
                FOR i = firstLineNumberLabelvWatch TO lastLineNumberLabelvWatch
                    IF ASC(vWatchUsedLabels, i) = 1 THEN
                        PRINT #12, "    case " + str2$(i) + ":"
                        PRINT #12, "        goto VWATCH_LABEL_" + str2$(i) + ";"
                        PRINT #12, "        break;"
                    END IF
                NEXT
                PRINT #12, "    default:"
                PRINT #12, "        *__LONG_VWATCH_GOTO=*__LONG_VWATCH_LINENUMBER;"
                PRINT #12, "        goto VWATCH_SETNEXTLINE;"
                PRINT #12, "}"

                PRINT #12, "VWATCH_SKIPLINE:;"
                PRINT #12, "switch (*__LONG_VWATCH_GOTO) {"
                FOR i = firstLineNumberLabelvWatch TO lastLineNumberLabelvWatch
                    IF ASC(vWatchUsedSkipLabels, i) = 1 THEN
                        PRINT #12, "    case -" + str2$(i) + ":"
                        PRINT #12, "        goto VWATCH_SKIPLABEL_" + str2$(i) + ";"
                        PRINT #12, "        break;"
                    END IF
                NEXT
                PRINT #12, "}"

            END IF

            PRINT #12, "}"
            PRINT #15, "}" 'end case
            PRINT #15, "}"
            PRINT #15, "error(3);" 'no valid return possible

            closedmain = 1
            firstLineNumberLabelvWatch = 0
        END SUB
















        '$INCLUDE:'utilities\symbols.bas'





        '$INCLUDE:'utilities\parsing.bas'




        SUB getid (i AS LONG)
            IF i = -1 THEN Give_Error "-1 passed to getid!": EXIT SUB

            id = ids(i)

            currentid = i
        END SUB

        FUNCTION isoperator (a2$)
            a$ = UCASE$(a2$)
            l = 0
            l = l + 1: IF a$ = "IMP" THEN GOTO opfound
            l = l + 1: IF a$ = "EQV" THEN GOTO opfound
            l = l + 1: IF a$ = "XOR" THEN GOTO opfound
            l = l + 1: IF a$ = "OR" THEN GOTO opfound
            l = l + 1: IF a$ = "AND" THEN GOTO opfound
            l = l + 1: IF a$ = "NOT" THEN GOTO opfound
            l = l + 1
            IF a$ = "=" THEN GOTO opfound
            IF a$ = ">" THEN GOTO opfound
            IF a$ = "<" THEN GOTO opfound
            IF a$ = "<>" THEN GOTO opfound
            IF a$ = "<=" THEN GOTO opfound
            IF a$ = ">=" THEN GOTO opfound
            l = l + 1
            IF a$ = "+" THEN GOTO opfound
            IF a$ = "-" THEN GOTO opfound '!CAREFUL! could be negation
            l = l + 1: IF a$ = "MOD" THEN GOTO opfound
            l = l + 1: IF a$ = "\" THEN GOTO opfound
            l = l + 1
            IF a$ = "*" THEN GOTO opfound
            IF a$ = "/" THEN GOTO opfound
            'NEGATION LEVEL (MUST BE SET AFTER CALLING ISOPERATOR BY CONTEXT)
            l = l + 1: IF a$ = CHR$(241) THEN GOTO opfound
            l = l + 1: IF a$ = "^" THEN GOTO opfound
            EXIT FUNCTION
            opfound:
            isoperator = l
        END FUNCTION

        FUNCTION isuinteger (i$)
            IF LEN(i$) = 0 THEN EXIT FUNCTION
            IF ASC(i$, 1) = 48 AND LEN(i$) > 1 THEN EXIT FUNCTION
            FOR c = 1 TO LEN(i$)
                v = ASC(i$, c)
                IF v < 48 OR v > 57 THEN EXIT FUNCTION
            NEXT
            isuinteger = -1
        END FUNCTION

        FUNCTION isvalidvariable (a$)
            FOR i = 1 TO LEN(a$)
                c = ASC(a$, i)
                t = 0
                IF c >= 48 AND c <= 57 THEN t = 1 'numeric
                IF c >= 65 AND c <= 90 THEN t = 2 'uppercase
                IF c >= 97 AND c <= 122 THEN t = 2 'lowercase
                IF c = 95 THEN t = 2 '_ underscore
                IF t = 2 OR (t = 1 AND i > 1) THEN
                    'valid (continue)
                ELSE
                    IF i = 1 THEN isvalidvariable = 0: EXIT FUNCTION
                    EXIT FOR
                END IF
            NEXT

            'All characters matched base identifier rules (letters, digits after first, underscore).
            IF i > LEN(a$) THEN
                isvalidvariable = 1
                EXIT FUNCTION
            END IF

            'Typed suffix begins at the first character that failed the loop (e.g. "name$" at "$").
            e$ = MID$(a$, i)
            IF e$ = "%%" OR e$ = "~%%" THEN isvalidvariable = 1: EXIT FUNCTION
            IF e$ = "%" OR e$ = "~%" THEN isvalidvariable = 1: EXIT FUNCTION
            IF e$ = "&" OR e$ = "~&" THEN isvalidvariable = 1: EXIT FUNCTION
            IF e$ = "&&" OR e$ = "~&&" THEN isvalidvariable = 1: EXIT FUNCTION
            IF e$ = "!" OR e$ = "#" OR e$ = "##" THEN isvalidvariable = 1: EXIT FUNCTION
            IF e$ = "$" THEN isvalidvariable = 1: EXIT FUNCTION
            IF e$ = "`" THEN isvalidvariable = 1: EXIT FUNCTION
            IF LEFT$(e$, 1) <> "$" AND LEFT$(e$, 1) <> "`" THEN isvalidvariable = 0: EXIT FUNCTION
            e$ = RIGHT$(e$, LEN(e$) - 1)
            IF isuinteger(e$) THEN isvalidvariable = 1: EXIT FUNCTION
            isvalidvariable = 0
        END FUNCTION

        '$INCLUDE:'utilities\tokens.bas'
        '$INCLUDE:'utilities\imports.bas'
        '$INCLUDE:'utilities\classes.bas'



        SUB makeidrefer (ref$, typ AS LONG)
            ref$ = str2$(currentid)
            typ = id.t + ISREFERENCE
        END SUB

        FUNCTION operatorusage (operator$, typ AS LONG, info$, lhs AS LONG, rhs AS LONG, result AS LONG)
            lhs = 7: rhs = 7: result = 0
            'return values
            '1 = use info$ as the operator without any other changes
            '2 = use the function returned in info$ to apply this operator
            '    upon left and right side of equation
            '3=  bracket left and right side with negation and change operator to info$
            '4=  BINARY NOT l.h.s, then apply operator in info$
            '5=  UNARY, bracket up rhs, apply operator info$ to left, rebracket again

            'lhs & rhs bit-field values
            '1=integeral
            '2=floating point
            '4=string
            '8=bool

            'string operator
            IF (typ AND ISSTRING) THEN
                lhs = 4: rhs = 4
                result = 4
                IF operator$ = "+" THEN info$ = "qbs_add": operatorusage = 2: EXIT FUNCTION
                result = 8
                IF operator$ = "=" THEN info$ = "qbs_equal": operatorusage = 2: EXIT FUNCTION
                IF operator$ = "<>" THEN info$ = "qbs_notequal": operatorusage = 2: EXIT FUNCTION
                IF operator$ = ">" THEN info$ = "qbs_greaterthan": operatorusage = 2: EXIT FUNCTION
                IF operator$ = "<" THEN info$ = "qbs_lessthan": operatorusage = 2: EXIT FUNCTION
                IF operator$ = ">=" THEN info$ = "qbs_greaterorequal": operatorusage = 2: EXIT FUNCTION
                IF operator$ = "<=" THEN info$ = "qbs_lessorequal": operatorusage = 2: EXIT FUNCTION
                IF Debug THEN PRINT #9, "INVALID STRING OPERATOR!": END
            END IF

            'assume numeric operator
            lhs = 1 + 2: rhs = 1 + 2
            IF operator$ = "^" THEN result = 2: info$ = "pow2": operatorusage = 2: EXIT FUNCTION
            IF operator$ = CHR$(241) THEN info$ = "-": operatorusage = 5: EXIT FUNCTION
            IF operator$ = "/" THEN
                info$ = "/ ": operatorusage = 1
                'for / division, either the lhs or the rhs must be a float to make
                'c++ return a result in floating point form
                IF (typ AND ISFLOAT) THEN
                    'lhs is a float
                    lhs = 2
                    rhs = 1 + 2
                ELSE
                    'lhs isn't a float!
                    lhs = 1 + 2
                    rhs = 2
                END IF
                result = 2
                EXIT FUNCTION
            END IF
            IF operator$ = "*" THEN info$ = "*": operatorusage = 1: EXIT FUNCTION
            IF operator$ = "+" THEN info$ = "+": operatorusage = 1: EXIT FUNCTION
            IF operator$ = "-" THEN info$ = "-": operatorusage = 1: EXIT FUNCTION

            result = 8
            IF operator$ = "=" THEN info$ = "==": operatorusage = 3: EXIT FUNCTION
            IF operator$ = ">" THEN info$ = ">": operatorusage = 3: EXIT FUNCTION
            IF operator$ = "<" THEN info$ = "<": operatorusage = 3: EXIT FUNCTION
            IF operator$ = "<>" THEN info$ = "!=": operatorusage = 3: EXIT FUNCTION
            IF operator$ = "<=" THEN info$ = "<=": operatorusage = 3: EXIT FUNCTION
            IF operator$ = ">=" THEN info$ = ">=": operatorusage = 3: EXIT FUNCTION

            lhs = 1: rhs = 1: result = 1
            operator$ = UCASE$(operator$)
            IF operator$ = "MOD" THEN info$ = "%": operatorusage = 1: EXIT FUNCTION
            IF operator$ = "\" THEN info$ = "/ ": operatorusage = 1: EXIT FUNCTION
            IF operator$ = "IMP" THEN info$ = "|": operatorusage = 4: EXIT FUNCTION
            IF operator$ = "EQV" THEN info$ = "^": operatorusage = 4: EXIT FUNCTION
            IF operator$ = "XOR" THEN info$ = "^": operatorusage = 1: EXIT FUNCTION
            IF operator$ = "OR" THEN info$ = "|": operatorusage = 1: EXIT FUNCTION
            IF operator$ = "AND" THEN info$ = "&": operatorusage = 1: EXIT FUNCTION

            lhs = 7
            IF operator$ = "NOT" THEN info$ = "~": operatorusage = 5: EXIT FUNCTION

            IF Debug THEN PRINT #9, "INVALID NUMBERIC OPERATOR!": END

        END FUNCTION

        FUNCTION refer$ (a2$, typ AS LONG, method AS LONG)
            typbak = typ
            'method: 0 return an equation which calculates the value of the "variable"
            '        1 return the C name of the variable, typ will be left unchanged

            a$ = a2$

            'retrieve ID
            i = INSTR(a$, sp3)
            IF i THEN
                idnumber = VAL(LEFT$(a$, i - 1)): a$ = RIGHT$(a$, LEN(a$) - i)
            ELSE
                idnumber = VAL(a$)
            END IF
            getid idnumber
            IF Error_Happened THEN EXIT FUNCTION

            'UDT?
            IF typ AND ISUDT THEN
                IF method = 1 THEN
                    n$ = "UDT_" + RTRIM$(id.n)
                    IF id.t = 0 THEN n$ = "ARRAY_" + n$
                    n$ = scope$ + n$
                    refer$ = n$
                    EXIT FUNCTION
                END IF

                'print "UDTSUBSTRING[idX|u|e|o]:"+a$

                u = VAL(a$)
                i = INSTR(a$, sp3): a$ = RIGHT$(a$, LEN(a$) - i): E = VAL(a$)
                i = INSTR(a$, sp3): o$ = RIGHT$(a$, LEN(a$) - i)
                n$ = "UDT_" + RTRIM$(id.n): IF id.t = 0 THEN n$ = "ARRAY_" + n$ + "[0]"
                IF E = 0 THEN Give_Error "User defined types in expressions are invalid": EXIT FUNCTION
                IF typ AND ISOFFSETINBITS THEN Give_Error "Cannot resolve bit-length variables inside user defined types": EXIT FUNCTION

                IF typ AND ISSTRING THEN
                    IF typ AND ISFIXEDLENGTH THEN
                        o2$ = "(((uint8*)" + scope$ + n$ + ")+(" + o$ + "))"
                        r$ = "qbs_new_fixed(" + o2$ + "," + str2(udtetypesize(E)) + ",1)"
                        typ = STRINGTYPE + ISFIXEDLENGTH 'ISPOINTER retained, it is still a pointer!
                    ELSE
                        r$ = "*((qbs**)((char*)" + scope$ + n$ + "+(" + o$ + ")))"
                        typ = STRINGTYPE
                    END IF
                ELSE
                    typ = typ - ISUDT - ISREFERENCE - ISPOINTER
                    IF typ AND ISARRAY THEN typ = typ - ISARRAY
                    t$ = typ2ctyp$(typ, "")
                    IF Error_Happened THEN EXIT FUNCTION
                    o2$ = "(((char*)" + scope$ + n$ + ")+(" + o$ + "))"
                    r$ = "*" + "(" + t$ + "*)" + o2$
                END IF

                'print "REFER:"+r$+","+str2$(typ)
                refer$ = r$
                EXIT FUNCTION
            END IF


            'array?
            IF id.arraytype THEN

                n$ = RTRIM$(id.callname)
                IF method = 1 THEN
                    refer$ = n$
                    typ = typbak
                    EXIT FUNCTION
                END IF
                typ = typ - ISPOINTER - ISREFERENCE 'typ now looks like a regular value

                IF (typ AND ISSTRING) THEN
                    IF (typ AND ISFIXEDLENGTH) THEN
                        offset$ = "&((uint8*)(" + n$ + "[0]))[(" + a$ + ")*" + str2(id.tsize) + "]"
                        r$ = "qbs_new_fixed(" + offset$ + "," + str2(id.tsize) + ",1)"
                    ELSE
                        r$ = "((qbs*)(((uint64*)(" + n$ + "[0]))[" + a$ + "]))"
                    END IF
                    stringprocessinghappened = 1
                    refer$ = r$
                    EXIT FUNCTION
                END IF

                IF (typ AND ISOFFSETINBITS) THEN
                    'IF (typ AND ISUNSIGNED) THEN r$ = "getubits_" ELSE r$ = "getbits_"
                    'r$ = r$ + str2(typ AND 511) + "("
                    IF (typ AND ISUNSIGNED) THEN r$ = "getubits" ELSE r$ = "getbits"
                    r$ = r$ + "(" + str2(typ AND 511) + ","
                    r$ = r$ + "(uint8*)(" + n$ + "[0])" + ","
                    r$ = r$ + a$ + ")"
                    refer$ = r$
                    EXIT FUNCTION
                ELSE
                    t$ = ""
                    IF (typ AND ISFLOAT) THEN
                        IF (typ AND 511) = 32 THEN t$ = "float"
                        IF (typ AND 511) = 64 THEN t$ = "double"
                        IF (typ AND 511) = 256 THEN t$ = "long double"
                    ELSE
                        IF (typ AND ISUNSIGNED) THEN
                            IF (typ AND 511) = 8 THEN t$ = "uint8"
                            IF (typ AND 511) = 16 THEN t$ = "uint16"
                            IF (typ AND 511) = 32 THEN t$ = "uint32"
                            IF (typ AND 511) = 64 THEN t$ = "uint64"
                            IF typ AND ISOFFSET THEN t$ = "uptrszint"
                        ELSE
                            IF (typ AND 511) = 8 THEN t$ = "int8"
                            IF (typ AND 511) = 16 THEN t$ = "int16"
                            IF (typ AND 511) = 32 THEN t$ = "int32"
                            IF (typ AND 511) = 64 THEN t$ = "int64"
                            IF typ AND ISOFFSET THEN t$ = "ptrszint"
                        END IF
                    END IF
                END IF
                IF t$ = "" THEN Give_Error "Cannot find C type to return array data": EXIT FUNCTION
                r$ = "((" + t$ + "*)(" + n$ + "[0]))[" + a$ + "]"
                refer$ = r$
                EXIT FUNCTION
            END IF 'array

            'variable?
            IF id.t THEN
                r$ = RTRIM$(id.n)
                t = id.t
                'remove irrelavant flags
                IF (t AND ISINCONVENTIONALMEMORY) THEN t = t - ISINCONVENTIONALMEMORY
                'string?
                IF (t AND ISSTRING) THEN
                    IF (t AND ISFIXEDLENGTH) THEN
                        r$ = scope$ + "STRING" + str2(id.tsize) + "_" + r$: GOTO ref
                    END IF
                    r$ = scope$ + "STRING_" + r$: GOTO ref
                END IF
                'bit-length single variable?
                IF (t AND ISOFFSETINBITS) THEN
                    IF (t AND ISUNSIGNED) THEN
                        r$ = "*" + scope$ + "UBIT" + str2(t AND 511) + "_" + r$
                    ELSE
                        r$ = "*" + scope$ + "BIT" + str2(t AND 511) + "_" + r$
                    END IF
                    GOTO ref
                END IF
                IF t = BYTETYPE THEN r$ = "*" + scope$ + "BYTE_" + r$: GOTO ref
                IF t = UBYTETYPE THEN r$ = "*" + scope$ + "UBYTE_" + r$: GOTO ref
                IF t = INTEGERTYPE THEN r$ = "*" + scope$ + "INTEGER_" + r$: GOTO ref
                IF t = UINTEGERTYPE THEN r$ = "*" + scope$ + "UINTEGER_" + r$: GOTO ref
                IF t = LONGTYPE THEN r$ = "*" + scope$ + "LONG_" + r$: GOTO ref
                IF t = ULONGTYPE THEN r$ = "*" + scope$ + "ULONG_" + r$: GOTO ref
                IF t = INTEGER64TYPE THEN r$ = "*" + scope$ + "INTEGER64_" + r$: GOTO ref
                IF t = UINTEGER64TYPE THEN r$ = "*" + scope$ + "UINTEGER64_" + r$: GOTO ref
                IF t = SINGLETYPE THEN r$ = "*" + scope$ + "SINGLE_" + r$: GOTO ref
                IF t = DOUBLETYPE THEN r$ = "*" + scope$ + "DOUBLE_" + r$: GOTO ref
                IF t = FLOATTYPE THEN r$ = "*" + scope$ + "FLOAT_" + r$: GOTO ref
                IF t = OFFSETTYPE THEN r$ = "*" + scope$ + "OFFSET_" + r$: GOTO ref
                IF t = UOFFSETTYPE THEN r$ = "*" + scope$ + "UOFFSET_" + r$: GOTO ref
                ref:
                IF (t AND ISSTRING) THEN stringprocessinghappened = 1
                IF (t AND ISPOINTER) THEN t = t - ISPOINTER
                typ = t
                IF method = 1 THEN
                    IF LEFT$(r$, 1) = "*" THEN r$ = RIGHT$(r$, LEN(r$) - 1)
                    typ = typbak
                END IF
                refer$ = r$
                EXIT FUNCTION
            END IF 'variable



        END FUNCTION


        SUB reginternal
            reginternalsubfunc = 1
            '$INCLUDE:'subs_functions\subs_functions.bas'
            reginternalsubfunc = 0
        END SUB

        'this sub is faulty atm!
        'sub replacelement (a$, i, newe$)
        ''note: performs no action for out of range values of i
        'e=1
        's=1
        'do
        'x=instr(s,a$,sp)
        'if x then
        'if e=i then
        'a1$=left$(a$,s-1): a2$=right$(a$,len(a$)-x+1)
        'a$=a1$+sp+newe$+a2$ 'note: a2 includes spacer
        'exit sub
        'end if
        's=x+1
        'e=e+1
        'end if
        'loop until x=0
        'if e=i then
        'a$=left$(a$,s-1)+sp+newe$
        'end if
        'end sub





        SUB setrefer (a2$, typ2 AS LONG, e2$, method AS LONG)
            a$ = a2$: typ = typ2: e$ = e2$
            IF method <> 1 THEN e$ = fixoperationorder$(e$)
            IF Error_Happened THEN EXIT SUB
            tl$ = tlayout$

            'method: 0 evaulatetotyp e$
            '        1 skip evaluation of e$ and use as is
            '*due to the complexity of setting a reference with a value/string
            ' this function handles the problem

            'retrieve ID
            i = INSTR(a$, sp3)
            IF i THEN
                idnumber = VAL(LEFT$(a$, i - 1)): a$ = RIGHT$(a$, LEN(a$) - i)
            ELSE
                idnumber = VAL(a$)
            END IF
            getid idnumber
            IF Error_Happened THEN EXIT SUB

            'UDT?
            IF typ AND ISUDT THEN

                'print "setrefer-ing a UDT!"
                u = VAL(a$)
                i = INSTR(a$, sp3): a$ = RIGHT$(a$, LEN(a$) - i): E = VAL(a$)
                i = INSTR(a$, sp3): o$ = RIGHT$(a$, LEN(a$) - i)
                n$ = "UDT_" + RTRIM$(id.n): IF id.t = 0 THEN n$ = "ARRAY_" + n$ + "[0]"

                IF E <> 0 AND u = 1 THEN 'Setting _MEM type elements is not allowed!
                Give_Error "Cannot set read-only element of _MEM TYPE": EXIT SUB
            END IF

            IF E = 0 THEN
                'use u and u's size

                IF method <> 0 THEN Give_Error "Unexpected internal code reference to UDT": EXIT SUB
                lhsscope$ = scope$
                e$ = evaluate(e$, t2)
                IF Error_Happened THEN EXIT SUB
                IF (t2 AND ISUDT) = 0 THEN Give_Error "Expected = similar user defined type": EXIT SUB

                IF (t2 AND ISREFERENCE) = 0 THEN
                    IF t2 AND ISPOINTER THEN
                        src$ = "((char*)" + e$ + ")"
                        e2 = 0: u2 = t2 AND 511
                    ELSE
                        src$ = "((char*)&" + e$ + ")"
                        e2 = 0: u2 = t2 AND 511
                    END IF
                    GOTO directudt
                END IF

                '****problem****
                idnumber2 = VAL(e$)
                getid idnumber2


                IF Error_Happened THEN EXIT SUB
                n2$ = "UDT_" + RTRIM$(id.n): IF id.t = 0 THEN n2$ = "ARRAY_" + n2$ + "[0]"
                i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i): u2 = VAL(e$)
                i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i): e2 = VAL(e$)
                i = INSTR(e$, sp3): o2$ = RIGHT$(e$, LEN(e$) - i)
                'WARNING: u2 may need minor modifications based on e to see if they are the same

                'we have now established we have 2 pointers to similar data types!
                'ASSUME BYTE TYPE!!!
                src$ = "((char*)" + scope$ + n2$ + ")+(" + o2$ + ")"
                directudt:
                IF u <> u2 OR e2 <> 0 THEN Give_Error "Expected = similar user defined type": EXIT SUB
                dst$ = "((char*)" + lhsscope$ + n$ + ")+(" + o$ + ")"
                copy_full_udt dst$, src$, 12, 0, u

                'print "setFULLUDTrefer!"

                tlayout$ = tl$
                EXIT SUB

            END IF 'e=0

            IF typ AND ISOFFSETINBITS THEN Give_Error "Cannot resolve bit-length variables inside user defined types": EXIT SUB
            IF typ AND ISSTRING THEN
                IF typ AND ISFIXEDLENGTH THEN
                    o2$ = "(((uint8*)" + scope$ + n$ + ")+(" + o$ + "))"
                    r$ = "qbs_new_fixed(" + o2$ + "," + str2(udtetypesize(E)) + ",1)"
                ELSE
                    r$ = "*((qbs**)((char*)(" + scope$ + n$ + ")+(" + o$ + ")))"
                END IF
                IF method = 0 THEN e$ = evaluatetotyp(e$, STRINGTYPE - ISPOINTER)
                IF Error_Happened THEN EXIT SUB
                PRINT #12, "qbs_set(" + r$ + "," + e$ + ");"
                PRINT #12, cleanupstringprocessingcall$ + "0);"
            ELSE
                typ = typ - ISUDT - ISREFERENCE - ISPOINTER
                IF typ AND ISARRAY THEN typ = typ - ISARRAY
                t$ = typ2ctyp$(typ, "")
                IF Error_Happened THEN EXIT SUB
                o2$ = "(((char*)" + scope$ + n$ + ")+(" + o$ + "))"
                r$ = "*" + "(" + t$ + "*)" + o2$
                IF method = 0 THEN e$ = evaluatetotyp(e$, typ)
                IF Error_Happened THEN EXIT SUB
                PRINT #12, r$ + "=" + e$ + ";"
            END IF

            'print "setUDTrefer:"+r$,e$
            tlayout$ = tl$
            IF LEFT$(r$, 1) = "*" THEN r$ = MID$(r$, 2)
            EXIT SUB
        END IF


        'array?
        IF id.arraytype THEN
            n$ = RTRIM$(id.callname)
            typ = typ - ISPOINTER - ISREFERENCE 'typ now looks like a regular value

            IF (typ AND ISSTRING) THEN
                IF (typ AND ISFIXEDLENGTH) THEN
                    offset$ = "&((uint8*)(" + n$ + "[0]))[tmp_long*" + str2(id.tsize) + "]"
                    r$ = "qbs_new_fixed(" + offset$ + "," + str2(id.tsize) + ",1)"
                    PRINT #12, "tmp_long=" + a$ + ";"
                    IF method = 0 THEN
                        l$ = "if (!new_error) qbs_set(" + r$ + "," + evaluatetotyp(e$, typ) + ");"
                        IF Error_Happened THEN EXIT SUB
                    ELSE
                        l$ = "if (!new_error) qbs_set(" + r$ + "," + e$ + ");"
                    END IF
                    PRINT #12, l$
                ELSE
                    PRINT #12, "tmp_long=" + a$ + ";"
                    IF method = 0 THEN
                        l$ = "if (!new_error) qbs_set( ((qbs*)(((uint64*)(" + n$ + "[0]))[tmp_long]))," + evaluatetotyp(e$, typ) + ");"
                        IF Error_Happened THEN EXIT SUB
                    ELSE
                        l$ = "if (!new_error) qbs_set( ((qbs*)(((uint64*)(" + n$ + "[0]))[tmp_long]))," + e$ + ");"
                    END IF
                    PRINT #12, l$
                END IF
                PRINT #12, cleanupstringprocessingcall$ + "0);"
                tlayout$ = tl$
                IF LEFT$(r$, 1) = "*" THEN r$ = MID$(r$, 2)
                EXIT SUB
            END IF

            IF (typ AND ISOFFSETINBITS) THEN
                'r$ = "setbits_" + str2(typ AND 511) + "("
                r$ = "setbits(" + str2(typ AND 511) + ","
                r$ = r$ + "(uint8*)(" + n$ + "[0])" + ",tmp_long,"
                PRINT #12, "tmp_long=" + a$ + ";"
                IF method = 0 THEN
                    l$ = "if (!new_error) " + r$ + evaluatetotyp(e$, typ) + ");"
                    IF Error_Happened THEN EXIT SUB
                ELSE
                    l$ = "if (!new_error) " + r$ + e$ + ");"
                END IF
                PRINT #12, l$
                tlayout$ = tl$
                EXIT SUB
            ELSE
                t$ = ""
                IF (typ AND ISFLOAT) THEN
                    IF (typ AND 511) = 32 THEN t$ = "float"
                    IF (typ AND 511) = 64 THEN t$ = "double"
                    IF (typ AND 511) = 256 THEN t$ = "long double"
                ELSE
                    IF (typ AND ISUNSIGNED) THEN
                        IF (typ AND 511) = 8 THEN t$ = "uint8"
                        IF (typ AND 511) = 16 THEN t$ = "uint16"
                        IF (typ AND 511) = 32 THEN t$ = "uint32"
                        IF (typ AND 511) = 64 THEN t$ = "uint64"
                        IF typ AND ISOFFSET THEN t$ = "uptrszint"
                    ELSE
                        IF (typ AND 511) = 8 THEN t$ = "int8"
                        IF (typ AND 511) = 16 THEN t$ = "int16"
                        IF (typ AND 511) = 32 THEN t$ = "int32"
                        IF (typ AND 511) = 64 THEN t$ = "int64"
                        IF typ AND ISOFFSET THEN t$ = "ptrszint"
                    END IF
                END IF
            END IF
            IF t$ = "" THEN Give_Error "Cannot find C type to return array data": EXIT SUB
            PRINT #12, "tmp_long=" + a$ + ";"
            IF method = 0 THEN
                l$ = "if (!new_error) ((" + t$ + "*)(" + n$ + "[0]))[tmp_long]=" + evaluatetotyp(e$, typ) + ";"
                IF Error_Happened THEN EXIT SUB
            ELSE
                l$ = "if (!new_error) ((" + t$ + "*)(" + n$ + "[0]))[tmp_long]=" + e$ + ";"
            END IF

            PRINT #12, l$
            tlayout$ = tl$
            EXIT SUB
        END IF 'array

        'variable?
        IF id.t THEN
            r$ = RTRIM$(id.n)
            t = id.t
            'remove irrelavant flags
            IF (t AND ISINCONVENTIONALMEMORY) THEN t = t - ISINCONVENTIONALMEMORY
            typ = t

            'string variable?
            IF (t AND ISSTRING) THEN
                IF (t AND ISFIXEDLENGTH) THEN
                    r$ = scope$ + "STRING" + str2(id.tsize) + "_" + r$
                ELSE
                    r$ = scope$ + "STRING_" + r$
                END IF
                IF method = 0 THEN e$ = evaluatetotyp(e$, ISSTRING)
                IF Error_Happened THEN EXIT SUB
                PRINT #12, "qbs_set(" + r$ + "," + e$ + ");"
                PRINT #12, cleanupstringprocessingcall$ + "0);"
                IF arrayprocessinghappened THEN arrayprocessinghappened = 0
                tlayout$ = tl$
                IF LEFT$(r$, 1) = "*" THEN r$ = MID$(r$, 2)
                EXIT SUB
            END IF

            'bit-length variable?
            IF (t AND ISOFFSETINBITS) THEN
                b = t AND 511
                IF (t AND ISUNSIGNED) THEN
                    r$ = "*" + scope$ + "UBIT" + str2(t AND 511) + "_" + r$
                    IF method = 0 THEN e$ = evaluatetotyp(e$, 64& + ISUNSIGNED)
                    IF Error_Happened THEN EXIT SUB
                    l$ = r$ + "=(" + e$ + ")&" + str2(bitmask(b)) + ";"
                    PRINT #12, l$
                ELSE
                    r$ = "*" + scope$ + "BIT" + str2(t AND 511) + "_" + r$
                    IF method = 0 THEN e$ = evaluatetotyp(e$, 64&)
                    IF Error_Happened THEN EXIT SUB
                    l$ = "if ((" + r$ + "=" + e$ + ")&" + str2(2 ^ (b - 1)) + "){"
                    PRINT #12, l$
                    'signed bit is set
                    l$ = r$ + "|=" + str2(bitmaskinv(b)) + ";"
                    PRINT #12, l$
                    PRINT #12, "}else{"
                    'signed bit is not set
                    l$ = r$ + "&=" + str2(bitmask(b)) + ";"
                    PRINT #12, l$
                    PRINT #12, "}"
                END IF
                IF stringprocessinghappened THEN PRINT #12, cleanupstringprocessingcall$ + "0);": stringprocessinghappened = 0
                IF arrayprocessinghappened THEN arrayprocessinghappened = 0
                tlayout$ = tl$
                IF LEFT$(r$, 1) = "*" THEN r$ = MID$(r$, 2)
                EXIT SUB
            END IF

            'standard variable?
            IF t = BYTETYPE THEN r$ = "*" + scope$ + "BYTE_" + r$: GOTO sref
            IF t = UBYTETYPE THEN r$ = "*" + scope$ + "UBYTE_" + r$: GOTO sref
            IF t = INTEGERTYPE THEN r$ = "*" + scope$ + "INTEGER_" + r$: GOTO sref
            IF t = UINTEGERTYPE THEN r$ = "*" + scope$ + "UINTEGER_" + r$: GOTO sref
            IF t = LONGTYPE THEN r$ = "*" + scope$ + "LONG_" + r$: GOTO sref
            IF t = ULONGTYPE THEN r$ = "*" + scope$ + "ULONG_" + r$: GOTO sref
            IF t = INTEGER64TYPE THEN r$ = "*" + scope$ + "INTEGER64_" + r$: GOTO sref
            IF t = UINTEGER64TYPE THEN r$ = "*" + scope$ + "UINTEGER64_" + r$: GOTO sref
            IF t = SINGLETYPE THEN r$ = "*" + scope$ + "SINGLE_" + r$: GOTO sref
            IF t = DOUBLETYPE THEN r$ = "*" + scope$ + "DOUBLE_" + r$: GOTO sref
            IF t = FLOATTYPE THEN r$ = "*" + scope$ + "FLOAT_" + r$: GOTO sref
            IF t = OFFSETTYPE THEN r$ = "*" + scope$ + "OFFSET_" + r$: GOTO sref
            IF t = UOFFSETTYPE THEN r$ = "*" + scope$ + "UOFFSET_" + r$: GOTO sref
            sref:
            t2 = t - ISPOINTER
            IF method = 0 THEN e$ = evaluatetotyp(e$, t2)
            IF Error_Happened THEN EXIT SUB
            l$ = r$ + "=" + e$ + ";"
            PRINT #12, l$
            IF stringprocessinghappened THEN PRINT #12, cleanupstringprocessingcall$ + "0);": stringprocessinghappened = 0
            IF arrayprocessinghappened THEN arrayprocessinghappened = 0
            tlayout$ = tl$

            IF LEFT$(r$, 1) = "*" THEN r$ = MID$(r$, 2)
            EXIT SUB
        END IF 'variable

        tlayout$ = tl$
    END SUB

    FUNCTION str2$ (v AS LONG)
        str2$ = _TRIM$(STR$(v))
    END FUNCTION

    FUNCTION str2u64$ (v~&&)
        str2u64$ = LTRIM$(RTRIM$(STR$(v~&&)))
    END FUNCTION

    FUNCTION str2i64$ (v&&)
        str2i64$ = LTRIM$(RTRIM$(STR$(v&&)))
    END FUNCTION

    SUB xend
        IF vWatchOn = 1 THEN
            'check if closedmain = 0 in case a main module ends in an include.
            IF (inclinenumber(inclevel) = 0 OR closedmain = 0) THEN vWatchAddLabel 0, -1
            PRINT #12, "*__LONG_VWATCH_LINENUMBER= 0; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);"
        END IF
        PRINT #12, "sub_end();"
    END SUB

    SUB xfileprint (a$, ca$, n)
        u$ = str2$(uniquenumber)
        PRINT #12, "tab_spc_cr_size=2;"
        IF n = 2 THEN Give_Error "Expected # ... , ...": EXIT SUB
        a3$ = ""
        b = 0
        FOR i = 3 TO n
            a2$ = getelement$(ca$, i)
            IF a2$ = "(" THEN b = b + 1
            IF a2$ = ")" THEN b = b - 1
            IF a2$ = "," AND b = 0 THEN
                IF a3$ = "" THEN Give_Error "Expected # ... , ...": EXIT SUB
                GOTO printgotfn
            END IF
            IF a3$ = "" THEN a3$ = a2$ ELSE a3$ = a3$ + sp + a2$
        NEXT
        Give_Error "Expected # ... ,": EXIT SUB
        printgotfn:
        e$ = fixoperationorder$(a3$)
        IF Error_Happened THEN EXIT SUB
        l$ = SCase$("Print") + sp + "#" + sp2 + tlayout$ + sp2 + ","
        e$ = evaluatetotyp(e$, 64&)
        IF Error_Happened THEN EXIT SUB
        PRINT #12, "tab_fileno=tmp_fileno=" + e$ + ";"
        PRINT #12, "if (new_error) goto skip" + u$ + ";"
        i = i + 1

        'PRINT USING? (file)
        IF n >= i THEN
            IF getelement(a$, i) = "USING" THEN
                'get format string
                fpujump:
                l$ = l$ + sp + SCase$("Using")
                e$ = "": b = 0: puformat$ = ""
                FOR i = i + 1 TO n
                    a2$ = getelement(ca$, i)
                    IF a2$ = "(" THEN b = b + 1
                    IF a2$ = ")" THEN b = b - 1
                    IF b = 0 THEN
                        IF a2$ = "," THEN Give_Error "Expected PRINT USING #filenumber, formatstring ; ...": EXIT SUB
                        IF a2$ = ";" THEN
                            e$ = fixoperationorder$(e$)
                            IF Error_Happened THEN EXIT SUB
                            l$ = l$ + sp + tlayout$ + sp2 + ";"
                            e$ = evaluate(e$, typ)
                            IF Error_Happened THEN EXIT SUB
                            IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                            IF Error_Happened THEN EXIT SUB
                            IF (typ AND ISSTRING) = 0 THEN Give_Error "Expected PRINT USING #filenumber, formatstring ; ...": EXIT SUB
                            puformat$ = e$
                            EXIT FOR
                        END IF ';
                    END IF 'b
                    IF LEN(e$) THEN e$ = e$ + sp + a2$ ELSE e$ = a2$
                NEXT
                IF puformat$ = "" THEN Give_Error "Expected PRINT USING #filenumber, formatstring ; ...": EXIT SUB
                IF i = n THEN Give_Error "Expected PRINT USING #filenumber, formatstring ; ...": EXIT SUB
                'create build string
                PRINT #12, "tqbs=qbs_new(0,0);"
                'set format start/index variable
                PRINT #12, "tmp_long=0;" 'scan format from beginning
                'create string to hold format in for multiple references
                puf$ = "print_using_format" + u$
                IF subfunc = "" THEN
                    PRINT #13, "static qbs *" + puf$ + ";"
                ELSE
                    PRINT #13, "qbs *" + puf$ + ";"
                END IF
                PRINT #12, puf$ + "=qbs_new(0,0); qbs_set(" + puf$ + "," + puformat$ + ");"
                PRINT #12, "if (new_error) goto skip" + u$ + ";"
                'print expressions
                b = 0
                e$ = ""
                last = 0
                FOR i = i + 1 TO n
                    a2$ = getelement(ca$, i)
                    IF a2$ = "(" THEN b = b + 1
                    IF a2$ = ")" THEN b = b - 1
                    IF b = 0 THEN
                        IF a2$ = ";" OR a2$ = "," THEN
                            fprintulast:
                            e$ = fixoperationorder$(e$)
                            IF Error_Happened THEN EXIT SUB
                            IF last THEN l$ = l$ + sp + tlayout$ ELSE l$ = l$ + sp + tlayout$ + sp2 + a2$
                            e$ = evaluate(e$, typ)
                            IF Error_Happened THEN EXIT SUB
                            IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                            IF Error_Happened THEN EXIT SUB
                            IF typ AND ISSTRING THEN

                                IF LEFT$(e$, 9) = "func_tab(" OR LEFT$(e$, 9) = "func_spc(" THEN

                                    'TAB/SPC exception
                                    'note: position in format-string must be maintained
                                    '-print any string up until now
                                    PRINT #12, "sub_file_print(tmp_fileno,tqbs,0,0,0);"
                                    '-print e$
                                    PRINT #12, "qbs_set(tqbs," + e$ + ");"
                                    PRINT #12, "if (new_error) goto skip_pu" + u$ + ";"
                                    PRINT #12, "sub_file_print(tmp_fileno,tqbs,0,0,0);"
                                    '-set length of tqbs to 0
                                    PRINT #12, "tqbs->len=0;"

                                ELSE

                                    'regular string
                                    PRINT #12, "tmp_long=print_using(" + puf$ + ",tmp_long,tqbs," + e$ + ");"

                                END IF

                            ELSE 'not a string
                                IF typ AND ISFLOAT THEN
                                    IF (typ AND 511) = 32 THEN PRINT #12, "tmp_long=print_using_single(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                                    IF (typ AND 511) = 64 THEN PRINT #12, "tmp_long=print_using_double(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                                    IF (typ AND 511) > 64 THEN PRINT #12, "tmp_long=print_using_float(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                                ELSE
                                    IF ((typ AND 511) = 64) AND (typ AND ISUNSIGNED) <> 0 THEN
                                        PRINT #12, "tmp_long=print_using_uinteger64(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                                    ELSE
                                        PRINT #12, "tmp_long=print_using_integer64(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                                    END IF
                                END IF
                            END IF 'string/not string
                            PRINT #12, "if (new_error) goto skip_pu" + u$ + ";"
                            e$ = ""
                            IF last THEN EXIT FOR
                            GOTO fprintunext
                        END IF
                    END IF
                    IF LEN(e$) THEN e$ = e$ + sp + a2$ ELSE e$ = a2$
                    fprintunext:
                NEXT
                IF e$ <> "" THEN a2$ = "": last = 1: GOTO fprintulast
                PRINT #12, "skip_pu" + u$ + ":"
                'check for errors
                PRINT #12, "if (new_error){"
                PRINT #12, "g_tmp_long=new_error; new_error=0; sub_file_print(tmp_fileno,tqbs,0,0,0); new_error=g_tmp_long;"
                PRINT #12, "}else{"
                IF a2$ = "," OR a2$ = ";" THEN nl = 0 ELSE nl = 1 'note: a2$ is set to the last element of a$
                PRINT #12, "sub_file_print(tmp_fileno,tqbs,0,0," + str2$(nl) + ");"
                PRINT #12, "}"
                PRINT #12, "qbs_free(tqbs);"
                PRINT #12, "qbs_free(" + puf$ + ");"
                PRINT #12, "skip" + u$ + ":"
                PRINT #12, cleanupstringprocessingcall$ + "0);"
                PRINT #12, "tab_spc_cr_size=1;"
                tlayout$ = l$
                EXIT SUB
            END IF
        END IF
        'end of print using code

        IF i > n THEN
            PRINT #12, "sub_file_print(tmp_fileno,nothingstring,0,0,1);"
            GOTO printblankline
        END IF
        b = 0
        e$ = ""
        last = 0
        FOR i = i TO n
            a2$ = getelement(ca$, i)
            IF a2$ = "(" THEN b = b + 1
            IF a2$ = ")" THEN b = b - 1
            IF b = 0 THEN
                IF a2$ = ";" OR a2$ = "," OR UCASE$(a2$) = "USING" THEN
                    printfilelast:

                    IF UCASE$(a2$) = "USING" THEN
                        IF e$ <> "" THEN gotofpu = 1 ELSE GOTO fpujump
                    END IF

                    IF a2$ = "," THEN usetab = 1 ELSE usetab = 0
                    IF last = 1 THEN newline = 1 ELSE newline = 0
                    extraspace = 0

                    IF LEN(e$) THEN
                        ebak$ = e$
                        pnrtnum = 0
                        printfilenumber:
                        e$ = fixoperationorder$(e$)
                        IF Error_Happened THEN EXIT SUB
                        IF pnrtnum = 0 THEN
                            IF last THEN l$ = l$ + sp + tlayout$ ELSE l$ = l$ + sp + tlayout$ + sp2 + a2$
                        END IF
                        e$ = evaluate(e$, typ)
                        IF Error_Happened THEN EXIT SUB
                        IF (typ AND ISSTRING) = 0 THEN
                            e$ = "STR$" + sp + "(" + sp + ebak$ + sp + ")"
                            extraspace = 1
                            pnrtnum = 1
                            GOTO printfilenumber 'force re-evaluation
                        END IF
                        IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                        IF Error_Happened THEN EXIT SUB
                        'format: string, (1/0) extraspace, (1/0) tab, (1/0)begin a new line
                        PRINT #12, "sub_file_print(tmp_fileno," + e$ + ","; extraspace; ","; usetab; ","; newline; ");"
                    ELSE 'len(e$)=0
                        IF a2$ = "," THEN l$ = l$ + sp + a2$
                        IF a2$ = ";" THEN
                            IF RIGHT$(l$, 1) <> ";" THEN l$ = l$ + sp + a2$ 'concat ;; to ;
                        END IF
                        IF usetab THEN PRINT #12, "sub_file_print(tmp_fileno,nothingstring,0,1,0);"
                    END IF 'len(e$)
                    PRINT #12, "if (new_error) goto skip" + u$ + ";"

                    e$ = ""
                    IF gotofpu THEN GOTO fpujump
                    IF last THEN EXIT FOR
                    GOTO printfilenext
                END IF ', or ;
            END IF 'b=0
            IF e$ <> "" THEN e$ = e$ + sp + a2$ ELSE e$ = a2$
            printfilenext:
        NEXT
        IF e$ <> "" THEN a2$ = "": last = 1: GOTO printfilelast
        printblankline:
        PRINT #12, "skip" + u$ + ":"
        PRINT #12, cleanupstringprocessingcall$ + "0);"
        PRINT #12, "tab_spc_cr_size=1;"
        tlayout$ = l$
    END SUB

    SUB xfilewrite (ca$, n)
        l$ = SCase$("Write") + sp + "#"
        u$ = str2$(uniquenumber)
        PRINT #12, "tab_spc_cr_size=2;"
        IF n = 2 THEN Give_Error "Expected # ...": EXIT SUB
        a3$ = ""
        b = 0
        FOR i = 3 TO n
            a2$ = getelement$(ca$, i)
            IF a2$ = "(" THEN b = b + 1
            IF a2$ = ")" THEN b = b - 1
            IF a2$ = "," AND b = 0 THEN
                IF a3$ = "" THEN Give_Error "Expected # ... , ...": EXIT SUB
                GOTO writegotfn
            END IF
            IF a3$ = "" THEN a3$ = a2$ ELSE a3$ = a3$ + sp + a2$
        NEXT
        Give_Error "Expected # ... ,": EXIT SUB
        writegotfn:
        e$ = fixoperationorder$(a3$)
        IF Error_Happened THEN EXIT SUB
        l$ = l$ + sp2 + tlayout$ + sp2 + ","
        e$ = evaluatetotyp(e$, 64&)
        IF Error_Happened THEN EXIT SUB
        PRINT #12, "tab_fileno=tmp_fileno=" + e$ + ";"
        PRINT #12, "if (new_error) goto skip" + u$ + ";"
        i = i + 1
        IF i > n THEN
            PRINT #12, "sub_file_print(tmp_fileno,nothingstring,0,0,1);"
            GOTO writeblankline
        END IF
        b = 0
        e$ = ""
        last = 0
        FOR i = i TO n
            a2$ = getelement(ca$, i)
            IF a2$ = "(" THEN b = b + 1
            IF a2$ = ")" THEN b = b - 1
            IF b = 0 THEN
                IF a2$ = "," THEN
                    writefilelast:
                    IF last = 1 THEN newline = 1 ELSE newline = 0
                    ebak$ = e$
                    reevaled = 0
                    writefilenumber:
                    e$ = fixoperationorder$(e$)
                    IF Error_Happened THEN EXIT SUB
                    IF reevaled = 0 THEN
                        l$ = l$ + sp + tlayout$
                        IF last = 0 THEN l$ = l$ + sp2 + ","
                    END IF
                    e$ = evaluate(e$, typ)
                    IF Error_Happened THEN EXIT SUB
                    IF reevaled = 0 THEN
                        IF (typ AND ISSTRING) = 0 THEN
                            e$ = "LTRIM$" + sp + "(" + sp + "STR$" + sp + "(" + sp + ebak$ + sp + ")" + sp + ")"
                            IF last = 0 THEN e$ = e$ + sp + "+" + sp + CHR$(34) + "," + CHR$(34) + ",1"
                            reevaled = 1
                            GOTO writefilenumber 'force re-evaluation
                        ELSE
                            e$ = CHR$(34) + "\042" + CHR$(34) + ",1" + sp + "+" + sp + ebak$ + sp + "+" + sp + CHR$(34) + "\042" + CHR$(34) + ",1"
                            IF last = 0 THEN e$ = e$ + sp + "+" + sp + CHR$(34) + "," + CHR$(34) + ",1"
                            reevaled = 1
                            GOTO writefilenumber 'force re-evaluation
                        END IF
                    END IF
                    IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                    IF Error_Happened THEN EXIT SUB
                    'format: string, (1/0) extraspace, (1/0) tab, (1/0)begin a new line
                    PRINT #12, "sub_file_print(tmp_fileno," + e$ + ",0,0,"; newline; ");"
                    PRINT #12, "if (new_error) goto skip" + u$ + ";"
                    e$ = ""
                    IF last THEN EXIT FOR
                    GOTO writefilenext
                END IF ',
            END IF 'b=0
            IF e$ <> "" THEN e$ = e$ + sp + a2$ ELSE e$ = a2$
            writefilenext:
        NEXT
        IF e$ <> "" THEN a2$ = ",": last = 1: GOTO writefilelast
        writeblankline:
        'print #12, "}"'new_error
        PRINT #12, "skip" + u$ + ":"
        PRINT #12, cleanupstringprocessingcall$ + "0);"
        PRINT #12, "tab_spc_cr_size=1;"
        layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    END SUB

    SUB xgosub (ca$)
        a2$ = getelement(ca$, 2)
        IF validlabel(a2$) = 0 THEN Give_Error "Invalid label": EXIT SUB

        v = HashFind(a2$, HASHFLAG_LABEL, ignore, r)
        x = 1
        labchk200:
        IF v THEN
            s = Labels(r).Scope
            IF s = subfuncn OR s = -1 THEN 'same scope?
            IF s = -1 THEN Labels(r).Scope = subfuncn 'acquire scope
            x = 0 'already defined
            tlayout$ = RTRIM$(Labels(r).cn)
        ELSE
            IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk200
        END IF
    END IF
    IF x THEN
        'does not exist
        nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
        Labels(nLabels) = Empty_Label
        HashAdd a2$, HASHFLAG_LABEL, nLabels
        r = nLabels
        Labels(r).State = 0
        Labels(r).cn = a2$
        Labels(r).Scope = subfuncn
        Labels(r).Error_Line = linenumber
    END IF 'x

    l$ = SCase$("GoSub") + sp + tlayout$
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
    'note: This code fragment also used by ON ... GOTO/GOSUB
    'assume label is reachable (revise)
    PRINT #12, "return_point[next_return_point++]=" + str2(gosubid) + ";"
    PRINT #12, "if (next_return_point>=return_points) more_return_points();"
    PRINT #12, "goto LABEL_" + a2$ + ";"
    'add return point jump
    PRINT #15, "case " + str2(gosubid) + ":"
    PRINT #15, "goto RETURN_" + str2(gosubid) + ";"
    PRINT #15, "break;"
    PRINT #12, "RETURN_" + str2(gosubid) + ":;"
    gosubid = gosubid + 1
END SUB

SUB xongotogosub (a$, ca$, n)
    IF n < 4 THEN Give_Error "Expected ON expression GOTO/GOSUB label,label,...": EXIT SUB
    l$ = SCase$("On")
    b = 0
    FOR i = 2 TO n
        e2$ = getelement$(a$, i)
        IF e2$ = "(" THEN b = b + 1
        IF e2$ = ")" THEN b = b - 1
        IF e2$ = "GOTO" OR e2$ = "GOSUB" THEN EXIT FOR
    NEXT
    IF i >= n OR i = 2 THEN Give_Error "Expected ON expression GOTO/GOSUB label,label,...": EXIT SUB
    e$ = getelements$(ca$, 2, i - 1)

    g = 0: IF e2$ = "GOSUB" THEN g = 1
    e$ = fixoperationorder(e$)
    IF Error_Happened THEN EXIT SUB
    l$ = l$ + sp + tlayout$
    e$ = evaluate(e$, typ)
    IF Error_Happened THEN EXIT SUB
    IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
    IF Error_Happened THEN EXIT SUB
    IF (typ AND ISSTRING) THEN Give_Error "Expected numeric expression": EXIT SUB
    IF (typ AND ISFLOAT) THEN
        e$ = "qbr_float_to_long(" + e$ + ")"
    END IF
    l$ = l$ + sp + e2$
    u$ = str2$(uniquenumber)
    PRINT #13, "static int32 ongo_" + u$ + "=0;"
    PRINT #12, "ongo_" + u$ + "=" + e$ + ";"
    ln = 1
    labelwaslast = 0
    FOR i = i + 1 TO n
        e$ = getelement$(ca$, i)
        IF e$ = "," THEN
            l$ = l$ + sp2 + ","
            IF i = n THEN Give_Error "Trailing , invalid": EXIT SUB
            ln = ln + 1
            labelwaslast = 0
        ELSE
            IF labelwaslast THEN Give_Error "Expected ,": EXIT SUB
            IF validlabel(e$) = 0 THEN Give_Error "Invalid label!": EXIT SUB

            v = HashFind(e$, HASHFLAG_LABEL, ignore, r)
            x = 1
            labchk507:
            IF v THEN
                s = Labels(r).Scope
                IF s = subfuncn OR s = -1 THEN 'same scope?
                IF s = -1 THEN Labels(r).Scope = subfuncn 'acquire scope
                x = 0 'already defined
                tlayout$ = RTRIM$(Labels(r).cn)
            ELSE
                IF v = 2 THEN v = HashFindCont(ignore, r): GOTO labchk507
            END IF
        END IF
        IF x THEN
            'does not exist
            nLabels = nLabels + 1: IF nLabels > Labels_Ubound THEN Labels_Ubound = Labels_Ubound * 2: REDIM _PRESERVE Labels(1 TO Labels_Ubound) AS Label_Type
            Labels(nLabels) = Empty_Label
            HashAdd e$, HASHFLAG_LABEL, nLabels
            r = nLabels
            Labels(r).State = 0
            Labels(r).cn = e$
            Labels(r).Scope = subfuncn
            Labels(r).Error_Line = linenumber
        END IF 'x

        l$ = l$ + sp + tlayout$
        IF g THEN 'gosub
        lb$ = e$
        PRINT #12, "if (ongo_" + u$ + "==" + str2$(ln) + "){"
        'note: This code fragment also used by ON ... GOTO/GOSUB
        'assume label is reachable (revise)
        PRINT #12, "return_point[next_return_point++]=" + str2(gosubid) + ";"
        PRINT #12, "if (next_return_point>=return_points) more_return_points();"
        PRINT #12, "goto LABEL_" + lb$ + ";"
        'add return point jump
        PRINT #15, "case " + str2(gosubid) + ":"
        PRINT #15, "goto RETURN_" + str2(gosubid) + ";"
        PRINT #15, "break;"
        PRINT #12, "RETURN_" + str2(gosubid) + ":;"
        gosubid = gosubid + 1
        PRINT #12, "goto ongo_" + u$ + "_skip;"
        PRINT #12, "}"
    ELSE 'goto
        PRINT #12, "if (ongo_" + u$ + "==" + str2$(ln) + ") goto LABEL_" + e$ + ";"
    END IF
    labelwaslast = 1
END IF
NEXT
PRINT #12, "if (ongo_" + u$ + "<0) error(5);"
IF g = 1 THEN PRINT #12, "ongo_" + u$ + "_skip:;"
layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
END SUB

SUB xprint (a$, ca$, n)
    u$ = str2$(uniquenumber)

    l$ = SCase$("Print")
    IF ASC(a$) = 76 THEN lp = 1: lp$ = "l": l$ = SCase$("LPrint"): PRINT #12, "tab_LPRINT=1;": DEPENDENCY(DEPENDENCY_PRINTER) = 1 '"L"

    'PRINT USING?
    IF n >= 2 THEN
        IF getelement(a$, 2) = "USING" THEN
            'get format string
            i = 3
            pujump:
            l$ = l$ + sp + SCase$("Using")
            e$ = "": b = 0: puformat$ = ""
            FOR i = i TO n
                a2$ = getelement(ca$, i)
                IF a2$ = "(" THEN b = b + 1
                IF a2$ = ")" THEN b = b - 1
                IF b = 0 THEN
                    IF a2$ = "," THEN Give_Error "Expected PRINT USING formatstring ; ...": EXIT SUB
                    IF a2$ = ";" THEN
                        e$ = fixoperationorder$(e$)
                        IF Error_Happened THEN EXIT SUB
                        l$ = l$ + sp + tlayout$ + sp2 + ";"
                        e$ = evaluate(e$, typ)
                        IF Error_Happened THEN EXIT SUB
                        IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                        IF Error_Happened THEN EXIT SUB
                        IF (typ AND ISSTRING) = 0 THEN Give_Error "Expected PRINT USING formatstring ; ...": EXIT SUB
                        puformat$ = e$
                        EXIT FOR
                    END IF ';
                END IF 'b
                IF LEN(e$) THEN e$ = e$ + sp + a2$ ELSE e$ = a2$
            NEXT
            IF puformat$ = "" THEN Give_Error "Expected PRINT USING formatstring ; ...": EXIT SUB
            IF i = n THEN Give_Error "Expected PRINT USING formatstring ; ...": EXIT SUB
            'create build string
            IF TQBSset = 0 THEN
                PRINT #12, "tqbs=qbs_new(0,0);"
            ELSE
                PRINT #12, "qbs_set(tqbs,qbs_new_txt_len(" + CHR$(34) + CHR$(34) + ",0));"
            END IF
            'set format start/index variable
            PRINT #12, "tmp_long=0;" 'scan format from beginning


            'create string to hold format in for multiple references
            puf$ = "print_using_format" + u$
            IF subfunc = "" THEN
                PRINT #13, "static qbs *" + puf$ + ";"
            ELSE
                PRINT #13, "qbs *" + puf$ + ";"
            END IF
            PRINT #12, puf$ + "=qbs_new(0,0); qbs_set(" + puf$ + "," + puformat$ + ");"
            PRINT #12, "if (new_error) goto skip_pu" + u$ + ";"

            'print expressions
            b = 0
            e$ = ""
            last = 0
            FOR i = i + 1 TO n
                a2$ = getelement(ca$, i)
                IF a2$ = "(" THEN b = b + 1
                IF a2$ = ")" THEN b = b - 1
                IF b = 0 THEN
                    IF a2$ = ";" OR a2$ = "," THEN
                        printulast:
                        e$ = fixoperationorder$(e$)
                        IF Error_Happened THEN EXIT SUB
                        IF last THEN l$ = l$ + sp + tlayout$ ELSE l$ = l$ + sp + tlayout$ + sp2 + a2$
                        e$ = evaluate(e$, typ)
                        IF Error_Happened THEN EXIT SUB
                        IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                        IF Error_Happened THEN EXIT SUB
                        IF typ AND ISSTRING THEN

                            IF LEFT$(e$, 9) = "func_tab(" OR LEFT$(e$, 9) = "func_spc(" THEN

                                'TAB/SPC exception
                                'note: position in format-string must be maintained
                                '-print any string up until now
                                PRINT #12, "qbs_" + lp$ + "print(tqbs,0);"
                                '-print e$
                                PRINT #12, "qbs_set(tqbs," + e$ + ");"
                                PRINT #12, "if (new_error) goto skip_pu" + u$ + ";"
                                IF lp THEN PRINT #12, "lprint_makefit(tqbs);" ELSE PRINT #12, "makefit(tqbs);"
                                PRINT #12, "qbs_" + lp$ + "print(tqbs,0);"
                                '-set length of tqbs to 0
                                PRINT #12, "tqbs->len=0;"

                            ELSE

                                'regular string
                                PRINT #12, "tmp_long=print_using(" + puf$ + ",tmp_long,tqbs," + e$ + ");"

                            END IF



                        ELSE 'not a string
                            IF typ AND ISFLOAT THEN
                                IF (typ AND 511) = 32 THEN PRINT #12, "tmp_long=print_using_single(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                                IF (typ AND 511) = 64 THEN PRINT #12, "tmp_long=print_using_double(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                                IF (typ AND 511) > 64 THEN PRINT #12, "tmp_long=print_using_float(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                            ELSE
                                IF ((typ AND 511) = 64) AND (typ AND ISUNSIGNED) <> 0 THEN
                                    PRINT #12, "tmp_long=print_using_uinteger64(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                                ELSE
                                    PRINT #12, "tmp_long=print_using_integer64(" + puf$ + "," + e$ + ",tmp_long,tqbs);"
                                END IF
                            END IF
                        END IF 'string/not string
                        PRINT #12, "if (new_error) goto skip_pu" + u$ + ";"
                        e$ = ""
                        IF last THEN EXIT FOR
                        GOTO printunext
                    END IF
                END IF
                IF LEN(e$) THEN e$ = e$ + sp + a2$ ELSE e$ = a2$
                printunext:
            NEXT
            IF e$ <> "" THEN a2$ = "": last = 1: GOTO printulast
            PRINT #12, "skip_pu" + u$ + ":"
            'check for errors
            PRINT #12, "if (new_error){"
            PRINT #12, "g_tmp_long=new_error; new_error=0; qbs_" + lp$ + "print(tqbs,0); new_error=g_tmp_long;"
            PRINT #12, "}else{"
            IF a2$ = "," OR a2$ = ";" THEN nl = 0 ELSE nl = 1 'note: a2$ is set to the last element of a$
            PRINT #12, "qbs_" + lp$ + "print(tqbs," + str2$(nl) + ");"
            PRINT #12, "}"
            PRINT #12, "qbs_free(tqbs);"
            PRINT #12, "qbs_free(" + puf$ + ");"
            PRINT #12, "skip" + u$ + ":"
            PRINT #12, cleanupstringprocessingcall$ + "0);"
            IF lp THEN PRINT #12, "tab_LPRINT=0;"
            tlayout$ = l$
            EXIT SUB
        END IF
    END IF
    'end of print using code

    b = 0
    e$ = ""
    last = 0
    PRINT #12, "tqbs=qbs_new(0,0);" 'initialize the temp string
    TQBSset = -1 'set the temporary flag so we don't create a temp string twice, in case USING comes after something
    FOR i = 2 TO n
        a2$ = getelement(ca$, i)
        IF a2$ = "(" THEN b = b + 1
        IF a2$ = ")" THEN b = b - 1
        IF b = 0 THEN
            IF a2$ = ";" OR a2$ = "," OR UCASE$(a2$) = "USING" THEN
                printlast:

                IF UCASE$(a2$) = "USING" THEN
                    IF e$ <> "" THEN gotopu = 1 ELSE i = i + 1: GOTO pujump
                END IF

                IF LEN(e$) THEN
                    ebak$ = e$
                    pnrtnum = 0
                    printnumber:
                    e$ = fixoperationorder$(e$)
                    IF Error_Happened THEN EXIT SUB
                    IF pnrtnum = 0 THEN
                        IF last THEN l$ = l$ + sp + tlayout$ ELSE l$ = l$ + sp + tlayout$ + sp2 + a2$
                    END IF
                    e$ = evaluate(e$, typ)
                    IF Error_Happened THEN EXIT SUB
                    IF (typ AND ISSTRING) = 0 THEN
                        'not a string expresion!
                        e$ = "STR$" + sp + "(" + sp + ebak$ + sp + ")" + sp + "+" + sp + CHR$(34) + " " + CHR$(34)
                        pnrtnum = 1
                        GOTO printnumber
                    END IF
                    IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                    IF Error_Happened THEN EXIT SUB
                    PRINT #12, "qbs_set(tqbs," + e$ + ");"
                    PRINT #12, "if (new_error) goto skip" + u$ + ";"
                    IF lp THEN PRINT #12, "lprint_makefit(tqbs);" ELSE PRINT #12, "makefit(tqbs);"
                    PRINT #12, "qbs_" + lp$ + "print(tqbs,0);"
                ELSE
                    IF a2$ = "," THEN l$ = l$ + sp + a2$
                    IF a2$ = ";" THEN
                        IF RIGHT$(l$, 1) <> ";" THEN l$ = l$ + sp + a2$ 'concat ;; to ;
                    END IF
                END IF 'len(e$)
                IF a2$ = "," THEN PRINT #12, "tab();"
                e$ = ""

                IF gotopu THEN i = i + 1: GOTO pujump

                IF last THEN
                    PRINT #12, "qbs_" + lp$ + "print(nothingstring,1);" 'go to new line
                    EXIT FOR
                END IF

                GOTO printnext
            END IF 'a2$
        END IF 'b=0

        IF LEN(e$) THEN e$ = e$ + sp + a2$ ELSE e$ = a2$
        printnext:
    NEXT
    IF LEN(e$) THEN a2$ = "": last = 1: GOTO printlast
    IF n = 1 THEN PRINT #12, "qbs_" + lp$ + "print(nothingstring,1);"
    PRINT #12, "skip" + u$ + ":"
    PRINT #12, "qbs_free(tqbs);"
    PRINT #12, cleanupstringprocessingcall$ + "0);"
    IF lp THEN PRINT #12, "tab_LPRINT=0;"
    tlayout$ = l$
END SUB




SUB xread (ca$, n)
    l$ = SCase$("Read")
    IF n = 1 THEN Give_Error "Expected variable": EXIT SUB
    i = 2
    IF i > n THEN Give_Error "Expected , ...": EXIT SUB
    a3$ = ""
    b = 0
    FOR i = i TO n
        a2$ = getelement$(ca$, i)
        IF a2$ = "(" THEN b = b + 1
        IF a2$ = ")" THEN b = b - 1
        IF (a2$ = "," AND b = 0) OR i = n THEN
            IF i = n THEN
                IF a3$ = "" THEN a3$ = a2$ ELSE a3$ = a3$ + sp + a2$
            END IF
            IF a3$ = "" THEN Give_Error "Expected , ...": EXIT SUB
            e$ = fixoperationorder$(a3$)
            IF Error_Happened THEN EXIT SUB
            l$ = l$ + sp + tlayout$: IF i <> n THEN l$ = l$ + sp2 + ","
            e$ = evaluate(e$, t)
            IF Error_Happened THEN EXIT SUB
            IF (t AND ISREFERENCE) = 0 THEN Give_Error "Expected variable": EXIT SUB

            IF (t AND ISSTRING) THEN
                e$ = refer(e$, t, 0)
                IF Error_Happened THEN EXIT SUB
                PRINT #12, "sub_read_string(data,&data_offset,data_size," + e$ + ");"
                stringprocessinghappened = 1
            ELSE
                'numeric variable
                IF (t AND ISFLOAT) <> 0 OR (t AND 511) <> 64 THEN
                    IF (t AND ISOFFSETINBITS) THEN
                        setrefer e$, t, "((int64)func_read_float(data,&data_offset,data_size," + str2(t) + "))", 1
                        IF Error_Happened THEN EXIT SUB
                    ELSE
                        setrefer e$, t, "func_read_float(data,&data_offset,data_size," + str2(t) + ")", 1
                        IF Error_Happened THEN EXIT SUB
                    END IF
                ELSE
                    IF t AND ISUNSIGNED THEN
                        setrefer e$, t, "func_read_uint64(data,&data_offset,data_size)", 1
                        IF Error_Happened THEN EXIT SUB
                    ELSE
                        setrefer e$, t, "func_read_int64(data,&data_offset,data_size)", 1
                        IF Error_Happened THEN EXIT SUB
                    END IF
                END IF
            END IF 'string/numeric
            IF i = n THEN EXIT FOR
            a3$ = "": a2$ = ""
        END IF
        IF a3$ = "" THEN a3$ = a2$ ELSE a3$ = a3$ + sp + a2$
    NEXT
    IF stringprocessinghappened THEN PRINT #12, cleanupstringprocessingcall$ + "0);"
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
END SUB

SUB xwrite (ca$, n)
    l$ = SCase$("Write")
    u$ = str2$(uniquenumber)
    IF n = 1 THEN
        PRINT #12, "qbs_print(nothingstring,1);"
        GOTO writeblankline2
    END IF
    b = 0
    e$ = ""
    last = 0
    FOR i = 2 TO n
        a2$ = getelement(ca$, i)
        IF a2$ = "(" THEN b = b + 1
        IF a2$ = ")" THEN b = b - 1
        IF b = 0 THEN
            IF a2$ = "," THEN
                writelast:
                IF last = 1 THEN newline = 1 ELSE newline = 0
                ebak$ = e$
                reevaled = 0
                writechecked:
                e$ = fixoperationorder$(e$)
                IF Error_Happened THEN EXIT SUB
                IF reevaled = 0 THEN
                    l$ = l$ + sp + tlayout$
                    IF last = 0 THEN l$ = l$ + sp2 + ","
                END IF
                e$ = evaluate(e$, typ)
                IF Error_Happened THEN EXIT SUB
                IF reevaled = 0 THEN
                    IF (typ AND ISSTRING) = 0 THEN
                        e$ = "LTRIM$" + sp + "(" + sp + "STR$" + sp + "(" + sp + ebak$ + sp + ")" + sp + ")"
                        IF last = 0 THEN e$ = e$ + sp + "+" + sp + CHR$(34) + "," + CHR$(34) + ",1"
                        reevaled = 1
                        GOTO writechecked 'force re-evaluation
                    ELSE
                        e$ = CHR$(34) + "\042" + CHR$(34) + ",1" + sp + "+" + sp + ebak$ + sp + "+" + sp + CHR$(34) + "\042" + CHR$(34) + ",1"
                        IF last = 0 THEN e$ = e$ + sp + "+" + sp + CHR$(34) + "," + CHR$(34) + ",1"
                        reevaled = 1
                        GOTO writechecked 'force re-evaluation
                    END IF
                END IF
                IF (typ AND ISREFERENCE) THEN e$ = refer(e$, typ, 0)
                IF Error_Happened THEN EXIT SUB
                'format: string, (1/0) extraspace, (1/0) tab, (1/0)begin a new line
                PRINT #12, "qbs_print(" + e$ + ","; newline; ");"
                PRINT #12, "if (new_error) goto skip" + u$ + ";"
                e$ = ""
                IF last THEN EXIT FOR
                GOTO writenext
            END IF ',
        END IF 'b=0
        IF e$ <> "" THEN e$ = e$ + sp + a2$ ELSE e$ = a2$
        writenext:
    NEXT
    IF e$ <> "" THEN a2$ = ",": last = 1: GOTO writelast
    writeblankline2:
    PRINT #12, "skip" + u$ + ":"
    PRINT #12, cleanupstringprocessingcall$ + "0);"
    layoutdone = 1: IF LEN(layout$) THEN layout$ = layout$ + sp + l$ ELSE layout$ = l$
END SUB


FUNCTION IsDigitChar% (ch AS STRING)
    IF LEN(ch) = 0 THEN
        IsDigitChar% = 0
        EXIT FUNCTION
    END IF

    IF ASC(ch) >= 48 AND ASC(ch) <= 57 THEN
        IsDigitChar% = -1
    ELSE
        IsDigitChar% = 0
    END IF
END FUNCTION

'$INCLUDE:'includes\support.bas'


FUNCTION NewByteElement$
    a$ = "byte_element_" + str2$(uniquenumber)
    NewByteElement$ = a$
    IF use_global_byte_elements THEN
        PRINT #18, "byte_element_struct *" + a$ + "=(byte_element_struct*)malloc(12);"
    ELSE
        PRINT #13, "byte_element_struct *" + a$ + "=NULL;"
        PRINT #13, "if (!" + a$ + "){"
        PRINT #13, "if ((mem_static_pointer+=12)<mem_static_limit) " + a$ + "=(byte_element_struct*)(mem_static_pointer-12); else " + a$ + "=(byte_element_struct*)mem_static_malloc(12);"
        PRINT #13, "}"
    END IF
END FUNCTION

FUNCTION validname (a$)
    'notes:
    '1) '_1' is invalid because it has no alphabet letters
    '2) 'A_' is invalid because it has a trailing _
    '3) '_1A' is invalid because it contains a number before the first alphabet letter
    '4) names cannot be longer than 40 characters
    l = LEN(a$)

    IF l = 0 OR l > 40 THEN
        IF l = 0 THEN EXIT FUNCTION
        'Note: variable names with periods need to be obfuscated, and this affects their length
        i = INSTR(a$, fix046$)
        DO WHILE i
            l = l - LEN(fix046$) + 1
            i = INSTR(i + 1, a$, fix046$)
        LOOP
        IF l > 40 THEN EXIT FUNCTION
        l = LEN(a$)
    END IF

    'check for single, leading underscore
    IF l >= 2 THEN
        IF ASC(a$, 1) = 95 AND ASC(a$, 2) <> 95 THEN EXIT FUNCTION
    END IF

    FOR i = 1 TO l
        a = ASC(a$, i)
        IF alphanumeric(a) = 0 THEN EXIT FUNCTION
        IF isnumeric(a) THEN
            trailingunderscore = 0
            IF alphabetletter = 0 THEN EXIT FUNCTION
        ELSE
            IF a = 95 THEN
                trailingunderscore = 1
            ELSE
                alphabetletter = 1
                trailingunderscore = 0
            END IF
        END IF
    NEXT
    IF trailingunderscore THEN EXIT FUNCTION
    validname = 1
END FUNCTION

FUNCTION str_nth$ (x)
    IF x = 1 THEN str_nth$ = "1st": EXIT FUNCTION
    IF x = 2 THEN str_nth$ = "2nd": EXIT FUNCTION
    IF x = 3 THEN str_nth$ = "3rd": EXIT FUNCTION
    str_nth$ = str2(x) + "th"
END FUNCTION

SUB Give_Error (a$)
    Error_Happened = 1
    Error_Message = a$
END SUB

FUNCTION VRGBS~& (text$, DefaultColor AS _UNSIGNED LONG)
    'Value of RGB String = VRGBS without a ton of typing
    'A function to get the RGB value back from a string such as _RGB32(255,255,255)
    'text$ is the string that we send to check for a value
    'DefaultColor is the value we send back if the string isn't in the proper format

    VRGBS~& = DefaultColor 'A return the default value if we can't parse the string properly
    IF UCASE$(LEFT$(text$, 4)) = "_RGB" THEN
        rpos = INSTR(text$, "(")
        gpos = INSTR(rpos, text$, ",")
        bpos = INSTR(gpos + 1, text$, ",")
        IF rpos <> 0 AND bpos <> 0 AND gpos <> 0 THEN
            red = VAL(_TRIM$(MID$(text$, rpos + 1)))
            green = VAL(_TRIM$(MID$(text$, gpos + 1)))
            blue = VAL(_TRIM$(MID$(text$, bpos + 1)))
            VRGBS~& = _RGB32(red, green, blue)
        END IF
    END IF
END FUNCTION

FUNCTION rgbs$ (c AS _UNSIGNED LONG)
    rgbs$ = "_RGB32(" + _TRIM$(STR$(_RED32(c))) + ", " + _TRIM$(STR$(_GREEN32(c))) + ", " + _TRIM$(STR$(_BLUE32(c))) + ")"
END FUNCTION

FUNCTION EvalPreIF (text$, err$)
    temp$ = text$ 'so we don't corrupt the string sent to us for evaluation
    err$ = "" 'null the err message to begin with
    'first order of business is to solve for <>=
    DIM PC_Op(3) AS STRING
    PC_Op(1) = "="
    PC_Op(2) = "<"
    PC_Op(3) = ">"
    DO
        'look for the existence of the first symbol if there is any
        firstsymbol$ = "": first = 0
        FOR i = 1 TO UBOUND(PC_Op)
            temp = INSTR(temp$, PC_Op(i))
            IF first = 0 THEN first = temp: firstsymbol$ = PC_Op(i)
            IF temp <> 0 AND temp < first THEN first = temp: firstsymbol$ = PC_Op(i)
        NEXT
        IF firstsymbol$ <> "" THEN 'we've got = < >; let's see if we have a combination of them
        secondsymbol = 0: second = 0
        FOR i = first + 1 TO LEN(temp$)
            a$ = MID$(temp$, i, 1)
            SELECT CASE a$
            CASE " " 'ignore spaces
            CASE "=", "<", ">"
                IF a$ = firstsymbol$ THEN err$ = "Duplicate operator (" + a$ + ")": EXIT FUNCTION
                second = i: secondsymbol$ = a$
            CASE ELSE 'we found a symbol we don't recognize
                EXIT FOR
            END SELECT
        NEXT
    END IF
    IF first THEN 'we found a symbol
    l$ = RTRIM$(LEFT$(temp$, first - 1))
    IF second THEN rightstart = second + 1 ELSE rightstart = first + 1

    r$ = LTRIM$(MID$(temp$, rightstart))
    symbol$ = MID$(temp$, first, 1) + MID$(temp$, second, 1)
    'now we check for spaces to separate this segment from any other AND/OR conditions and such
    FOR i = LEN(l$) TO 1 STEP -1
        IF ASC(l$, i) = 32 THEN EXIT FOR
    NEXT
    leftside$ = RTRIM$(LEFT$(temp$, i))
    l$ = LTRIM$(RTRIM$(MID$(temp$, i + 1, LEN(l$) - i)))
    IF validname(l$) = 0 THEN err$ = "Invalid flag name": EXIT FUNCTION
    rightstop = LEN(r$)
    FOR i = 1 TO LEN(r$)
        IF ASC(r$, i) = 32 THEN EXIT FOR
    NEXT
    rightside$ = LTRIM$(MID$(r$, i + 1))
    r$ = LTRIM$(RTRIM$(LEFT$(r$, i - 1)))
    IF symbol$ = "=<" THEN symbol$ = "<="
    IF symbol$ = "=>" THEN symbol$ = ">="
    IF symbol$ = "><" THEN symbol$ = "<>"
    result$ = " 0 "
    IF symbol$ = "<>" THEN 'check to see if we're NOT equal in any case with <>
    FOR i = 0 TO UserDefineCount
        IF UserDefineName$(i) = l$ AND UserDefineValue$(i) <> r$ THEN result$ = " -1 ": GOTO finishedcheck
    NEXT
END IF
IF INSTR(symbol$, "=") THEN 'check to see if we're equal in any case with =
UserFound = 0
FOR i = 0 TO UserDefineCount
    IF UserDefineName$(i) = l$ THEN
        UserFound = -1
        IF UserDefineValue$(i) = r$ THEN result$ = " -1 ": GOTO finishedcheck
    END IF
NEXT
IF UserFound = 0 AND LTRIM$(RTRIM$(r$)) = "UNDEFINED" THEN result$ = " -1 ": GOTO finishedcheck
IF UserFound = -1 AND LTRIM$(RTRIM$(r$)) = "DEFINED" THEN result$ = " -1 ": GOTO finishedcheck
END IF

IF INSTR(symbol$, ">") THEN 'check to see if we're greater than in any case with >
FOR i = 0 TO UserDefineCount
    IF VerifyNumber(r$) AND VerifyNumber(UserDefineValue$(i)) THEN 'we're comparing numeric values
    IF UserDefineName$(i) = l$ AND VAL(UserDefineValue$(i)) > VAL(r$) THEN result$ = " -1 ": GOTO finishedcheck
ELSE
    IF UserDefineName$(i) = l$ AND UserDefineValue$(i) > r$ THEN result$ = " -1 ": GOTO finishedcheck
END IF
NEXT
END IF
IF INSTR(symbol$, "<") THEN 'check to see if we're less than in any case with <
FOR i = 0 TO UserDefineCount
    IF VerifyNumber(r$) AND VerifyNumber(UserDefineValue$(i)) THEN 'we're comparing numeric values
    IF UserDefineName$(i) = l$ AND VAL(UserDefineValue$(i)) < VAL(r$) THEN result$ = " -1 ": GOTO finishedcheck
ELSE
    IF UserDefineName$(i) = l$ AND UserDefineValue$(i) < r$ THEN result$ = " -1 ": GOTO finishedcheck
END IF
NEXT
END IF



finishedcheck:
temp$ = leftside$ + result$ + rightside$
END IF
LOOP UNTIL first = 0

'And at this point we should now be down to a statement with nothing but AND/OR/XORS in it

PC_Op(1) = " AND "
PC_Op(2) = " OR "
PC_Op(3) = " XOR "

DO
    first = 0
    FOR i = 1 TO UBOUND(PC_Op)
        IF PC_Op(i) <> "" THEN
            t = INSTR(temp$, PC_Op(i))
            IF first <> 0 THEN
                IF t < first AND t <> 0 THEN first = t: firstsymbol = i
            ELSE
                first = t: firstsymbol = i
            END IF
        END IF
    NEXT
    IF first = 0 THEN EXIT DO
    leftside$ = RTRIM$(LEFT$(temp$, first - 1))
    symbol$ = MID$(temp$, first, LEN(PC_Op(firstsymbol)))
    t$ = MID$(temp$, first + LEN(PC_Op(firstsymbol)))
    t = INSTR(t$, " ") 'the first space we come to
    IF t THEN
        m$ = LTRIM$(RTRIM$(LEFT$(t$, t - 1)))
        rightside$ = LTRIM$(MID$(t$, t))
    ELSE
        m$ = LTRIM$(MID$(t$, t))
        rightside$ = ""
    END IF
    leftresult = 0
    IF VerifyNumber(leftside$) THEN
        IF VAL(leftside$) <> 0 THEN leftresult = -1
    ELSE
        FOR i = 0 TO UserDefineCount
            IF UserDefineName$(i) = leftside$ THEN
                t$ = LTRIM$(RTRIM$(UserDefineValue$(i)))
                IF t$ <> "0" AND t$ <> "" THEN leftresult = -1: EXIT FOR
            END IF
        NEXT
    END IF
    rightresult = 0
    IF VerifyNumber(m$) THEN
        IF VAL(m$) <> 0 THEN rightresult = -1
    ELSE
        FOR i = 0 TO UserDefineCount
            IF UserDefineName$(i) = m$ THEN
                t$ = LTRIM$(RTRIM$(UserDefineValue$(i)))
                IF t$ <> "0" AND t$ <> "" THEN rightresult = -1: EXIT FOR
            END IF
        NEXT
    END IF
    SELECT CASE LTRIM$(RTRIM$(symbol$))
    CASE "AND"
        IF leftresult <> 0 AND rightresult <> 0 THEN result$ = " -1 " ELSE result$ = " 0 "
    CASE "OR"
        IF leftresult <> 0 OR rightresult <> 0 THEN result$ = " -1 " ELSE result$ = " 0 "
    CASE "XOR"
        IF leftresult <> rightresult THEN result$ = " -1 " ELSE result$ = " 0 "
    END SELECT
    temp$ = result$ + rightside$
LOOP

IF VerifyNumber(temp$) THEN
    EvalPreIF = VAL(temp$)
ELSE
    IF INSTR(temp$, " ") THEN err$ = "Invalid Resolution of $IF; check statements" 'If we've got more than 1 statement, it's invalid
    FOR i = 0 TO UserDefineCount
        IF UserDefineName$(i) = temp$ THEN
            t$ = LTRIM$(RTRIM$(UserDefineValue$(i)))
            IF t$ <> "0" AND t$ <> "" THEN EvalPreIF = -1: EXIT FOR
        END IF
    NEXT
END IF

END FUNCTION

FUNCTION VerifyNumber (text$)
    t$ = LTRIM$(RTRIM$(text$))
    v = VAL(t$)
    t1$ = LTRIM$(STR$(v))
    IF t$ = t1$ THEN VerifyNumber = -1
END FUNCTION

'$INCLUDE:'utilities\udts.bas'

SUB manageVariableList (__name$, __cname$, localIndex AS LONG, action AS _BYTE)
    DIM findItem AS LONG, cname$, i AS LONG, j AS LONG, name$, temp$
    name$ = RTRIM$(__name$)
    cname$ = RTRIM$(__cname$)

    IF LEN(cname$) = 0 THEN EXIT SUB

    findItem = INSTR(cname$, "[")
    IF findItem THEN
        cname$ = LEFT$(cname$, findItem - 1)
    END IF

    found = 0
    FOR i = 1 TO totalVariablesCreated
        IF usedVariableList(i).cname = cname$ THEN found = -1: EXIT FOR
    NEXT

    SELECT CASE action
    CASE 0 'add
        IF found = 0 THEN
            IF i > UBOUND(usedVariableList) THEN
                REDIM _PRESERVE usedVariableList(UBOUND(usedVariableList) + 999) AS usedVarList
            END IF

            usedVariableList(i).id = currentid
            usedVariableList(i).used = 0
            usedVariableList(i).watch = 0
            usedVariableList(i).displayFormat = 0
            usedVariableList(i).storage = ""
            usedVariableList(i).linenumber = linenumber
            usedVariableList(i).includeLevel = inclevel
            IF inclevel > 0 THEN
                usedVariableList(i).includedLine = inclinenumber(inclevel)
                thisincname$ = getfilepath$(incname$(inclevel))
                thisincname$ = MID$(incname$(inclevel), LEN(thisincname$) + 1)
                usedVariableList(i).includedFile = thisincname$
            ELSE
                totalMainVariablesCreated = totalMainVariablesCreated + 1
                usedVariableList(i).includedLine = 0
                usedVariableList(i).includedFile = ""
            END IF
            usedVariableList(i).scope = subfuncn
            usedVariableList(i).subfunc = subfunc
            usedVariableList(i).varType = id2fulltypename$
            usedVariableList(i).cname = cname$
            usedVariableList(i).localIndex = localIndex

            'remove eventual instances of fix046$ in name$
            DO WHILE INSTR(name$, fix046$)
                x = INSTR(name$, fix046$): name$ = LEFT$(name$, x - 1) + "." + RIGHT$(name$, LEN(name$) - x + 1 - LEN(fix046$))
            LOOP

            IF LEN(RTRIM$(id.musthave)) > 0 THEN
                usedVariableList(i).NAME = name$ + RTRIM$(id.musthave)
            ELSEIF LEN(RTRIM$(id.mayhave)) > 0 THEN
                usedVariableList(i).NAME = name$ + RTRIM$(id.mayhave)
            ELSE
                usedVariableList(i).NAME = name$
            END IF

            IF (id.arrayelements > 0) THEN
                usedVariableList(i).isarray = -1
                usedVariableList(i).NAME = usedVariableList(i).NAME + "()"
            ELSE
                usedVariableList(i).isarray = 0
            END IF
            usedVariableList(i).watchRange = ""
            usedVariableList(i).arrayElementSize = 0
            usedVariableList(i).indexes = ""
            usedVariableList(i).elements = ""
            usedVariableList(i).elementTypes = ""
            usedVariableList(i).elementOffset = ""
            totalVariablesCreated = totalVariablesCreated + 1

            temp$ = MKL$(-1) + MKL$(LEN(cname$)) + cname$
            found = INSTR(backupVariableWatchList$, temp$)
            IF found THEN
                'this variable existed in a previous edit of this program
                'in this same session; let's preselect it.
                j = CVL(MID$(backupVariableWatchList$, found + LEN(temp$), 4))
            END IF
        END IF
    CASE ELSE 'find and mark as used
        IF found THEN
            usedVariableList(i).used = -1
        END IF
    END SELECT
END SUB

SUB addWarning (whichLineNumber AS LONG, includeLevel AS LONG, incLineNumber AS LONG, incFileName$, header$, text$)
    DIM warningContext AS STRING
    DIM warningSecondary AS STRING
    DIM warningFile AS STRING
    DIM warningLine AS LONG
    DIM warningLocation AS STRING
    DIM warningCode AS INTEGER
    DIM warningMessage AS STRING
    DIM activePhase AS STRING
    DIM flowText AS STRING
    DIM suggestionText AS STRING
    DIM causeText AS STRING
    DIM exampleText AS STRING
    DIM locationText AS STRING
    DIM lineStr AS STRING
    totalWarnings = totalWarnings + 1

    IF WarningsAsErrors THEN
        warningFile = sourcefile$
        warningLine = whichLineNumber
        IF includeLevel > 0 THEN
            IF RTRIM$(incFileName$) <> "" THEN warningFile = incFileName$
            IF incLineNumber > 0 THEN warningLine = incLineNumber
            warningLocation = "Triggered while compiling an included module."
        ELSE
            warningLocation = ""
        END IF

        warningContext = CleanDiagnosticContext$(diagnosticSourceLine)
        IF RTRIM$(warningContext) = "" THEN warningContext = CleanDiagnosticContext$(wholeline$)
        IF RTRIM$(warningContext) = "" THEN warningContext = CleanDiagnosticContext$(linefragment)
        warningSecondary = CleanDiagnosticContext$(text$)

        warningCode = WARN_GENERIC
        warningMessage = RTRIM$(header$)
        SELECT CASE LCASE$(RTRIM$(header$))
        CASE "unused variable"
            warningCode = WARN_UNUSED_VARIABLE
        CASE "duplicate constant definition"
            warningCode = WARN_DUPLICATE_CONSTANT
        CASE "empty select case block"
            warningCode = WARN_EMPTY_SELECT_CASE
        CASE "feature incompatible with $debug mode"
            warningCode = WARN_DEBUG_INCOMPATIBLE_FEATURE
        END SELECT
        IF warningCode = WARN_DEBUG_INCOMPATIBLE_FEATURE AND RTRIM$(warningSecondary) <> "" THEN
            warningMessage = warningMessage + ": " + RTRIM$(warningSecondary)
        END IF

        activePhase = GetErrorPhase$
        IF RTRIM$(activePhase) = "" THEN activePhase = "Warning Analysis"
        flowText = activePhase + " :: warning evaluation"
        IF RTRIM$(warningFile) <> "" THEN flowText = flowText + " :: file: " + RTRIM$(warningFile)

        suggestionText = GetDetailedSuggestion$(warningCode, warningMessage, warningContext)
        causeText = GetErrorCause$(warningCode, warningMessage, warningContext)
        exampleText = GetFixExample$(warningCode, warningMessage, warningContext)
        locationText = FormatDiagnosticLocation$(RTRIM$(warningFile), warningLine, FindErrorColumn%(warningCode, warningMessage, warningContext))
        lineStr = LTRIM$(STR$(warningLine))
        IF lineStr = "" THEN lineStr = "?"

        _DEST _CONSOLE
        PRINT
        PRINT "[x] QBNex :: Error [" + GetDiagnosticCodeTag$(ERR_WARNING, warningCode) + "]  "; GetDiagnosticHeadline$(warningCode, warningMessage, warningContext)
        IF RTRIM$(locationText) <> "" THEN PRINT "  [@] " + RTRIM$(locationText)
        IF RTRIM$(warningContext) <> "" THEN
            PRINT "  [#] source"
            PRINT "    " + lineStr + " | " + RTRIM$(warningContext)
        END IF
        IF RTRIM$(suggestionText) <> "" THEN PRINT "  [>] next     " + RTRIM$(suggestionText)
        IF RTRIM$(flowText) <> "" THEN PRINT "  [::] flow    " + RTRIM$(flowText)
        PRINT "  [*] config   warning promoted to blocking diagnostic"
        IF RTRIM$(warningMessage) <> "" THEN PRINT "  [.] detail   " + RTRIM$(warningMessage)
        IF RTRIM$(warningLocation) <> "" THEN PRINT "  [^] where    " + RTRIM$(warningLocation)
        IF RTRIM$(warningSecondary) <> "" THEN PRINT "  [>>] while   " + RTRIM$(warningSecondary)
        IF RTRIM$(causeText) <> "" THEN PRINT "  [!] cause    " + RTRIM$(causeText)
        IF RTRIM$(exampleText) <> "" THEN PRINT "  [+] example  " + RTRIM$(exampleText)
        PRINT
        PRINT "[x] QBNex :: Build Halted  1 blocking diagnostic(s) (1 warning(s) promoted)"
        WarnIfStaleOutputBinary
        CleanupErrorHandler
        SYSTEM 1
    END IF

    IF ShowWarnings AND NOT IgnoreWarnings THEN
        thissource$ = getfilepath$(CMDLineFile)
        thissource$ = MID$(CMDLineFile, LEN(thissource$) + 1)
        thisincname$ = getfilepath$(incFileName$)
        thisincname$ = MID$(incFileName$, LEN(thisincname$) + 1)

        IF NOT MonochromeLoggingMode THEN COLOR 15
        IF includeLevel > 0 AND incLineNumber > 0 THEN
            PRINT thisincname$; ":";
            PRINT str2$(incLineNumber); ": ";
        ELSE
            PRINT thissource$; ":";
            PRINT str2$(whichLineNumber); ": ";
        END IF

        IF NOT MonochromeLoggingMode THEN COLOR 13
        PRINT "warning: ";
        IF NOT MonochromeLoggingMode THEN COLOR 7
        PRINT header$

        IF LEN(text$) > 0 THEN
            IF NOT MonochromeLoggingMode THEN COLOR 2
            PRINT SPACE$(4); text$
            IF NOT MonochromeLoggingMode THEN COLOR 7
        END IF
    END IF
    EXIT SUB
    increaseWarningCount:
    warningListItems = warningListItems + 1
    IF warningListItems > UBOUND(warning$) THEN
        REDIM _PRESERVE warning$(warningListItems + 999)
        REDIM _PRESERVE warningLines(warningListItems + 999) AS LONG
        REDIM _PRESERVE warningIncLines(warningListItems + 999) AS LONG
        REDIM _PRESERVE warningIncFiles(warningListItems + 999) AS STRING
    END IF
    RETURN
END SUB

FUNCTION CompilerProgressLine$ (percentage AS LONG)
    IF percentage < 0 THEN percentage = 0
    IF percentage > 100 THEN percentage = 100
    x2 = (percentage * 40) \ 100
    a$ = STRING$(x2, "#") + STRING$(40 - x2, ".")
    CompilerProgressLine$ = "Preparing build files... [" + a$ + "] " + str2$(percentage) + "%"
END FUNCTION

SUB FinishCompilerProgress
    IF compilerProgressVisible THEN
        LOCATE compilerProgressRow + 2, 1
        compilerProgressVisible = 0
    END IF
END SUB

SUB ShowCompilerBanner
    PRINT "  QQQQ    BBBB    N   N   EEEEE   X   X  "
    PRINT " Q    Q   B   B   NN  N   E        X X   "
    PRINT " Q  QQ    BBBB    N N N   EEEE      X    "
    PRINT " Q   Q    B   B   N  NN   E        X X   "
    PRINT "  QQQQ    BBBB    N   N   EEEEE   X   X  "
    PRINT
    PRINT "QBNex Compiler"
    PRINT
    compilerProgressRow = CSRLIN
    compilerProgressVisible = 0
END SUB

FUNCTION SCase$ (t$)
    SCase$ = t$
END FUNCTION

SUB UpdateCompilerProgress (percentage AS LONG)
    IF compilerProgressVisible = 0 THEN EXIT SUB
    LOCATE compilerProgressRow, 1
    PRINT CompilerProgressLine$(percentage);
END SUB

FUNCTION SCase2$ (t$)
    separator$ = sp
    newWord = -1
    temp$ = ""
    FOR i = 1 TO LEN(t$)
        s$ = MID$(t$, i, 1)
        IF newWord THEN
            IF s$ = "_" OR s$ = separator$ THEN
                temp$ = temp$ + s$
            ELSE
                temp$ = temp$ + UCASE$(s$)
                newWord = 0
            END IF
        ELSE
            IF s$ = separator$ THEN
                temp$ = temp$ + separator$
                newWord = -1
            ELSE
                temp$ = temp$ + LCASE$(s$)
            END IF
        END IF
    NEXT
    SCase2$ = temp$
END FUNCTION

' Generic method (can be used outside of QBNex) includes:

'$INCLUDE:'includes\runtime.bas'

DEFLNG A-Z

'-------- Optional layout component (2/2) --------
