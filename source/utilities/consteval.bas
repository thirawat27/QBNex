'Steve Subs/Functins for _MATH support with CONST
FUNCTION Evaluate_Expression$ (e$)
    t$ = e$ 'So we preserve our original data, we parse a temp copy of it
    PreParse t$


    IF LEFT$(t$, 5) = "ERROR" THEN Evaluate_Expression$ = t$: EXIT FUNCTION

    'Deal with brackets first
    exp$ = "(" + t$ + ")" 'Starting and finishing brackets for our parse routine.

    DO
        Eval_E = INSTR(exp$, ")")
        IF Eval_E > 0 THEN
            c = 0
            DO UNTIL Eval_E - c <= 0
                c = c + 1
                IF Eval_E THEN
                    IF MID$(exp$, Eval_E - c, 1) = "(" THEN EXIT DO
                END IF
            LOOP
            s = Eval_E - c + 1
            IF s < 1 THEN Evaluate_Expression$ = "ERROR -- BAD () Count": EXIT FUNCTION
            eval$ = " " + MID$(exp$, s, Eval_E - s) + " " 'pad with a space before and after so the parser can pick up the values properly.

            ParseExpression eval$
            eval$ = LTRIM$(RTRIM$(eval$))
            IF LEFT$(eval$, 5) = "ERROR" THEN Evaluate_Expression$ = eval$: EXIT FUNCTION
            exp$ = DWD(LEFT$(exp$, s - 2) + eval$ + MID$(exp$, Eval_E + 1))
            IF MID$(exp$, 1, 1) = "N" THEN MID$(exp$, 1) = "-"
        END IF
    LOOP UNTIL Eval_E = 0
    c = 0
    DO
        c = c + 1
        SELECT CASE MID$(exp$, c, 1)
        CASE "0" TO "9", ".", "-" 'At this point, we should only have number values left.
        CASE ELSE: Evaluate_Expression$ = "ERROR - Unknown Diagnosis: (" + exp$ + ") ": EXIT FUNCTION
        END SELECT
    LOOP UNTIL c >= LEN(exp$)

    Evaluate_Expression$ = exp$
END FUNCTION



SUB ParseExpression (exp$)
    DIM num(10) AS STRING
    'PRINT exp$
    exp$ = DWD(exp$)
    'We should now have an expression with no () to deal with

    FOR J = 1 TO 250
        lowest = 0
        DO UNTIL lowest = LEN(exp$)
            lowest = LEN(exp$): OpOn = 0
            FOR P = 1 TO UBOUND(OName)
                'Look for first valid operator
                IF J = PL(P) THEN 'Priority levels match
                IF LEFT$(exp$, 1) = "-" THEN startAt = 2 ELSE startAt = 1
                op = INSTR(startAt, exp$, OName(P))
                IF op = 0 AND LEFT$(OName(P), 1) = "_" AND qbnexprefix_set = 1 THEN
                    'try again without prefix
                    op = INSTR(startAt, exp$, MID$(OName(P), 2))
                    IF op > 0 THEN
                        exp$ = LEFT$(exp$, op - 1) + "_" + MID$(exp$, op)
                        lowest = lowest + 1
                    END IF
                END IF
                IF op > 0 AND op < lowest THEN lowest = op: OpOn = P
            END IF
        NEXT
        IF OpOn = 0 THEN EXIT DO 'We haven't gotten to the proper PL for this OP to be processed yet.
        IF LEFT$(exp$, 1) = "-" THEN startAt = 2 ELSE startAt = 1
        op = INSTR(startAt, exp$, OName(OpOn))

        numset = 0

        '*** SPECIAL OPERATION RULESETS
        IF OName(OpOn) = "-" THEN 'check for BOOLEAN operators before the -
        SELECT CASE MID$(exp$, op - 3, 3)
        CASE "NOT", "XOR", "AND", "EQV", "IMP"
            EXIT DO 'Not an operator, it's a negative
        END SELECT
        IF MID$(exp$, op - 3, 2) = "OR" THEN EXIT DO 'Not an operator, it's a negative
    END IF

    IF op THEN
        c = LEN(OName(OpOn)) - 1
        DO
            SELECT CASE MID$(exp$, op + c + 1, 1)
            CASE "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", ".", "N": numset = -1 'Valid digit
            CASE "-" 'We need to check if it's a minus or a negative
                IF OName(OpOn) = "_PI" OR numset THEN EXIT DO
            CASE ",": numset = 0
            CASE ELSE 'Not a valid digit, we found our separator
                EXIT DO
            END SELECT
            c = c + 1
        LOOP UNTIL op + c >= LEN(exp$)
        E = op + c

        c = 0
        DO
            c = c + 1
            SELECT CASE MID$(exp$, op - c, 1)
            CASE "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", ".", "N" 'Valid digit
            CASE "-" 'We need to check if it's a minus or a negative
                c1 = c
                bad = 0
                DO
                    c1 = c1 + 1
                    SELECT CASE MID$(exp$, op - c1, 1)
                    CASE "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "."
                        bad = -1
                        EXIT DO 'It's a minus sign
                    CASE ELSE
                        'It's a negative sign and needs to count as part of our numbers
                    END SELECT
                LOOP UNTIL op - c1 <= 0
                IF bad THEN EXIT DO 'We found our seperator
            CASE ELSE 'Not a valid digit, we found our separator
                EXIT DO
            END SELECT
        LOOP UNTIL op - c <= 0
        s = op - c
        num(1) = MID$(exp$, s + 1, op - s - 1) 'Get our first number
        num(2) = MID$(exp$, op + LEN(OName(OpOn)), E - op - LEN(OName(OpOn)) + 1) 'Get our second number
        IF MID$(num(1), 1, 1) = "N" THEN MID$(num(1), 1) = "-"
        IF MID$(num(2), 1, 1) = "N" THEN MID$(num(2), 1) = "-"
        IF num(1) = "-" THEN
            num(3) = "N" + EvaluateNumbers(OpOn, num())
        ELSE
            num(3) = EvaluateNumbers(OpOn, num())
        END IF
        IF MID$(num(3), 1, 1) = "-" THEN MID$(num(3), 1) = "N"
        IF LEFT$(num(3), 5) = "ERROR" THEN exp$ = num(3): EXIT SUB
        exp$ = LTRIM$(N2S(DWD(LEFT$(exp$, s) + RTRIM$(LTRIM$(num(3))) + MID$(exp$, E + 1))))
    END IF
    op = 0
LOOP
NEXT

END SUB



SUB Set_OrderOfOperations
    'PL sets our priortity level. 1 is highest to 65535 for the lowest.
    'I used a range here so I could add in new priority levels as needed.
    'OName ended up becoming the name of our commands, as I modified things.... Go figure!  LOL!
    REDIM OName(10000) AS STRING, PL(10000) AS INTEGER
    'Constants get evaluated first, with a Priority Level of 1

    i = i + 1: OName(i) = "C_UOF": PL(i) = 5 'convert to unsigned offset
    i = i + 1: OName(i) = "C_OF": PL(i) = 5 'convert to offset
    i = i + 1: OName(i) = "C_UBY": PL(i) = 5 'convert to unsigned byte
    i = i + 1: OName(i) = "C_BY": PL(i) = 5 'convert to byte
    i = i + 1: OName(i) = "C_UIN": PL(i) = 5 'convert to unsigned integer
    i = i + 1: OName(i) = "C_IN": PL(i) = 5 'convert to integer
    i = i + 1: OName(i) = "C_UIF": PL(i) = 5 'convert to unsigned int64
    i = i + 1: OName(i) = "C_IF": PL(i) = 5 'convert to int64
    i = i + 1: OName(i) = "C_ULO": PL(i) = 5 'convert to unsigned long
    i = i + 1: OName(i) = "C_LO": PL(i) = 5 'convert to long
    i = i + 1: OName(i) = "C_SI": PL(i) = 5 'convert to single
    i = i + 1: OName(i) = "C_FL": PL(i) = 5 'convert to float
    i = i + 1: OName(i) = "C_DO": PL(i) = 5 'convert to double
    i = i + 1: OName(i) = "C_UBI": PL(i) = 5 'convert to unsigned bit
    i = i + 1: OName(i) = "C_BI": PL(i) = 5 'convert to bit

    'Then Functions with PL 10
    i = i + 1:: OName(i) = "_PI": PL(i) = 10
    i = i + 1: OName(i) = "_ACOS": PL(i) = 10
    i = i + 1: OName(i) = "_ASIN": PL(i) = 10
    i = i + 1: OName(i) = "_ARCSEC": PL(i) = 10
    i = i + 1: OName(i) = "_ARCCSC": PL(i) = 10
    i = i + 1: OName(i) = "_ARCCOT": PL(i) = 10
    i = i + 1: OName(i) = "_SECH": PL(i) = 10
    i = i + 1: OName(i) = "_CSCH": PL(i) = 10
    i = i + 1: OName(i) = "_COTH": PL(i) = 10
    i = i + 1: OName(i) = "COS": PL(i) = 10
    i = i + 1: OName(i) = "SIN": PL(i) = 10
    i = i + 1: OName(i) = "TAN": PL(i) = 10
    i = i + 1: OName(i) = "LOG": PL(i) = 10
    i = i + 1: OName(i) = "EXP": PL(i) = 10
    i = i + 1: OName(i) = "ATN": PL(i) = 10
    i = i + 1: OName(i) = "_D2R": PL(i) = 10
    i = i + 1: OName(i) = "_D2G": PL(i) = 10
    i = i + 1: OName(i) = "_R2D": PL(i) = 10
    i = i + 1: OName(i) = "_R2G": PL(i) = 10
    i = i + 1: OName(i) = "_G2D": PL(i) = 10
    i = i + 1: OName(i) = "_G2R": PL(i) = 10
    i = i + 1: OName(i) = "ABS": PL(i) = 10
    i = i + 1: OName(i) = "SGN": PL(i) = 10
    i = i + 1: OName(i) = "INT": PL(i) = 10
    i = i + 1: OName(i) = "_ROUND": PL(i) = 10
    i = i + 1: OName(i) = "_CEIL": PL(i) = 10
    i = i + 1: OName(i) = "FIX": PL(i) = 10
    i = i + 1: OName(i) = "_SEC": PL(i) = 10
    i = i + 1: OName(i) = "_CSC": PL(i) = 10
    i = i + 1: OName(i) = "_COT": PL(i) = 10
    i = i + 1: OName(i) = "ASC": PL(i) = 10
    i = i + 1: OName(i) = "C_RG": PL(i) = 10 '_RGB32 converted
    i = i + 1: OName(i) = "C_RA": PL(i) = 10 '_RGBA32 converted
    i = i + 1: OName(i) = "_RGB": PL(i) = 10
    i = i + 1: OName(i) = "_RGBA": PL(i) = 10
    i = i + 1: OName(i) = "C_RX": PL(i) = 10 '_RED32 converted
    i = i + 1: OName(i) = "C_GR": PL(i) = 10 ' _GREEN32 converted
    i = i + 1: OName(i) = "C_BL": PL(i) = 10 '_BLUE32 converted
    i = i + 1: OName(i) = "C_AL": PL(i) = 10 '_ALPHA32 converted
    i = i + 1: OName(i) = "_RED": PL(i) = 10
    i = i + 1: OName(i) = "_GREEN": PL(i) = 10
    i = i + 1: OName(i) = "_BLUE": PL(i) = 10
    i = i + 1: OName(i) = "_ALPHA": PL(i) = 10

    'Exponents with PL 20
    i = i + 1: OName(i) = "^": PL(i) = 20
    i = i + 1: OName(i) = "SQR": PL(i) = 20
    i = i + 1: OName(i) = "ROOT": PL(i) = 20
    'Multiplication and Division PL 30
    i = i + 1: OName(i) = "*": PL(i) = 30
    i = i + 1: OName(i) = "/": PL(i) = 30
    'Integer Division PL 40
    i = i + 1: OName(i) = "\": PL(i) = 40
    'MOD PL 50
    i = i + 1: OName(i) = "MOD": PL(i) = 50
    'Addition and Subtraction PL 60
    i = i + 1: OName(i) = "+": PL(i) = 60
    i = i + 1: OName(i) = "-": PL(i) = 60

    'Relational Operators =, >, <, <>, <=, >=   PL 70
    i = i + 1: OName(i) = "<>": PL(i) = 70 'These next three are just reversed symbols as an attempt to help process a common typo
    i = i + 1: OName(i) = "><": PL(i) = 70
    i = i + 1: OName(i) = "<=": PL(i) = 70
    i = i + 1: OName(i) = ">=": PL(i) = 70
    i = i + 1: OName(i) = "=<": PL(i) = 70 'I personally can never keep these things straight.  Is it < = or = <...
    i = i + 1: OName(i) = "=>": PL(i) = 70 'Who knows, check both!
    i = i + 1: OName(i) = ">": PL(i) = 70
    i = i + 1: OName(i) = "<": PL(i) = 70
    i = i + 1: OName(i) = "=": PL(i) = 70
    'Logical Operations PL 80+
    i = i + 1: OName(i) = "NOT": PL(i) = 80
    i = i + 1: OName(i) = "AND": PL(i) = 90
    i = i + 1: OName(i) = "OR": PL(i) = 100
    i = i + 1: OName(i) = "XOR": PL(i) = 110
    i = i + 1: OName(i) = "EQV": PL(i) = 120
    i = i + 1: OName(i) = "IMP": PL(i) = 130
    i = i + 1: OName(i) = ",": PL(i) = 1000

    REDIM _PRESERVE OName(i) AS STRING, PL(i) AS INTEGER
END SUB

FUNCTION EvaluateNumbers$ (p, num() AS STRING)
    DIM n1 AS _FLOAT, n2 AS _FLOAT, n3 AS _FLOAT
    'PRINT "EVALNUM:"; OName(p), num(1), num(2)

    IF _TRIM$(num(1)) = "" THEN num(1) = "0"

    IF PL(p) >= 20 AND (LEN(_TRIM$(num(1))) = 0 OR LEN(_TRIM$(num(2))) = 0) THEN
        EvaluateNumbers$ = "ERROR - Missing operand": EXIT FUNCTION
    END IF

    IF INSTR(num(1), ",") THEN
        EvaluateNumbers$ = "ERROR - Invalid comma (" + num(1) + ")": EXIT FUNCTION
    END IF
    l2 = INSTR(num(2), ",")
    IF l2 THEN
        SELECT CASE OName(p) 'only certain commands should pass a comma value
        CASE "C_RG", "C_RA", "_RGB", "_RGBA", "_RED", "_GREEN", "C_BL", "_ALPHA"
        CASE ELSE
            C$ = MID$(num(2), l2)
            num(2) = LEFT$(num(2), l2 - 1)
        END SELECT
    END IF

    SELECT CASE PL(p) 'divide up the work so we want do as much case checking
    CASE 5 'Type conversions
        'Note, these are special cases and work with the number BEFORE the command and not after
        SELECT CASE OName(p) 'Depending on our operator..
        CASE "C_UOF": n1~%& = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1~%&)))
        CASE "C_ULO": n1%& = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1%&)))
        CASE "C_UBY": n1~%% = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1~%%)))
        CASE "C_UIN": n1~% = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1~%)))
        CASE "C_BY": n1%% = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1%%)))
        CASE "C_IN": n1% = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1%)))
        CASE "C_UIF": n1~&& = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1~&&)))
        CASE "C_OF": n1~& = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1~&)))
        CASE "C_IF": n1&& = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1&&)))
        CASE "C_LO": n1& = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1&)))
        CASE "C_UBI": n1~` = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1~`)))
        CASE "C_BI": n1` = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1`)))
        CASE "C_FL": n1## = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1##)))
        CASE "C_DO": n1# = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1#)))
        CASE "C_SI": n1! = VAL(num(1)): EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1!)))
        END SELECT
        EXIT FUNCTION
    CASE 10 'functions
        SELECT CASE OName(p) 'Depending on our operator..
        CASE "_PI"
            n1 = 3.14159265358979323846264338327950288## 'Future compatable in case something ever stores extra digits for PI
            IF num(2) <> "" THEN n1 = n1 * VAL(num(2))
        CASE "_ACOS": n1 = _ACOS(VAL(num(2)))
        CASE "_ASIN": n1 = _ASIN(VAL(num(2)))
        CASE "_ARCSEC": n1 = _ARCSEC(VAL(num(2)))
        CASE "_ARCCSC": n1 = _ARCCSC(VAL(num(2)))
        CASE "_ARCCOT": n1 = _ARCCOT(VAL(num(2)))
        CASE "_SECH": n1 = _SECH(VAL(num(2)))
        CASE "_CSCH": n1 = _CSCH(VAL(num(2)))
        CASE "_COTH": n1 = _COTH(VAL(num(2)))
        CASE "C_RG"
            n$ = num(2)
            IF n$ = "" THEN EvaluateNumbers$ = "ERROR - Invalid null _RGB32": EXIT FUNCTION
            c1 = INSTR(n$, ",")
            IF c1 THEN c2 = INSTR(c1 + 1, n$, ",")
            IF c2 THEN c3 = INSTR(c2 + 1, n$, ",")
            IF c3 THEN c4 = INSTR(c3 + 1, n$, ",")
            IF c1 = 0 THEN 'there's no comma in the command to parse.  It's a grayscale value
            n = VAL(num(2))
            n1 = _RGB32(n, n, n)
        ELSEIF c2 = 0 THEN 'there's one comma and not 2.  It's grayscale with alpha.
            n = VAL(LEFT$(num(2), c1))
            n2 = VAL(MID$(num(2), c1 + 1))
            n1 = _RGBA32(n, n, n, n2)
        ELSEIF c3 = 0 THEN 'there's two commas.  It's _RGB values
            n = VAL(LEFT$(num(2), c1))
            n2 = VAL(MID$(num(2), c1 + 1))
            n3 = VAL(MID$(num(2), c2 + 1))
            n1 = _RGB32(n, n2, n3)
        ELSEIF c4 = 0 THEN 'there's three commas.  It's _RGBA values
            n = VAL(LEFT$(num(2), c1))
            n2 = VAL(MID$(num(2), c1 + 1))
            n3 = VAL(MID$(num(2), c2 + 1))
            n4 = VAL(MID$(num(2), c3 + 1))
            n1 = _RGBA32(n, n2, n3, n4)
        ELSE 'we have more than three commas.  I have no idea WTH type of values got passed here!
            EvaluateNumbers$ = "ERROR - Invalid comma count (" + num(2) + ")": EXIT FUNCTION
        END IF
    CASE "C_RA"
        n$ = num(2)
        IF n$ = "" THEN EvaluateNumbers$ = "ERROR - Invalid null _RGBA32": EXIT FUNCTION
        c1 = INSTR(n$, ",")
        IF c1 THEN c2 = INSTR(c1 + 1, n$, ",")
        IF c2 THEN c3 = INSTR(c2 + 1, n$, ",")
        IF c3 THEN c4 = INSTR(c3 + 1, n$, ",")
        IF c3 = 0 OR c4 <> 0 THEN EvaluateNumbers$ = "ERROR - Invalid comma count (" + num(2) + ")": EXIT FUNCTION
        'we have to have 3 commas; not more, not less.
        n = VAL(LEFT$(num(2), c1))
        n2 = VAL(MID$(num(2), c1 + 1))
        n3 = VAL(MID$(num(2), c2 + 1))
        n4 = VAL(MID$(num(2), c3 + 1))
        n1 = _RGBA32(n, n2, n3, n4)
    CASE "_RGB"
        n$ = num(2)
        IF n$ = "" THEN EvaluateNumbers$ = "ERROR - Invalid null _RGB": EXIT FUNCTION
        c1 = INSTR(n$, ",")
        IF c1 THEN c2 = INSTR(c1 + 1, n$, ",")
        IF c2 THEN c3 = INSTR(c2 + 1, n$, ",")
        IF c3 THEN c4 = INSTR(c3 + 1, n$, ",")
        IF c3 = 0 OR c4 <> 0 THEN EvaluateNumbers$ = "ERROR - Invalid comma count (" + num(2) + "). _RGB requires 4 parameters for Red, Green, Blue, ScreenMode.": EXIT FUNCTION
        'we have to have 3 commas; not more, not less.
        n = VAL(LEFT$(num(2), c1))
        n2 = VAL(MID$(num(2), c1 + 1))
        n3 = VAL(MID$(num(2), c2 + 1))
        n4 = VAL(MID$(num(2), c3 + 1))
        SELECT CASE n4
        CASE 0 TO 2, 7 TO 13, 256, 32 'these are the good screen values
        CASE ELSE
            EvaluateNumbers$ = "ERROR - Invalid Screen Mode (" + STR$(n4) + ")": EXIT FUNCTION
        END SELECT
        t = _NEWIMAGE(1, 1, n4)
        n1 = _RGB(n, n2, n3, t)
        _FREEIMAGE t
    CASE "_RGBA"
        n$ = num(2)
        IF n$ = "" THEN EvaluateNumbers$ = "ERROR - Invalid null _RGBA": EXIT FUNCTION
        c1 = INSTR(n$, ",")
        IF c1 THEN c2 = INSTR(c1 + 1, n$, ",")
        IF c2 THEN c3 = INSTR(c2 + 1, n$, ",")
        IF c3 THEN c4 = INSTR(c3 + 1, n$, ",")
        IF c4 THEN c5 = INSTR(c4 + 1, n$, ",")
        IF c4 = 0 OR c5 <> 0 THEN EvaluateNumbers$ = "ERROR - Invalid comma count (" + num(2) + "). _RGBA requires 5 parameters for Red, Green, Blue, Alpha, ScreenMode.": EXIT FUNCTION
        'we have to have 4 commas; not more, not less.
        n = VAL(LEFT$(num(2), c1))
        n2 = VAL(MID$(num(2), c1 + 1))
        n3 = VAL(MID$(num(2), c2 + 1))
        n4 = VAL(MID$(num(2), c3 + 1))
        n5 = VAL(MID$(num(2), c4 + 1))
        SELECT CASE n5
        CASE 0 TO 2, 7 TO 13, 256, 32 'these are the good screen values
        CASE ELSE
            EvaluateNumbers$ = "ERROR - Invalid Screen Mode (" + STR$(n5) + ")": EXIT FUNCTION
        END SELECT
        t = _NEWIMAGE(1, 1, n5)
        n1 = _RGBA(n, n2, n3, n4, t)
        _FREEIMAGE t
    CASE "_RED", "_GREEN", "_BLUE", "_ALPHA"
        n$ = num(2)
        IF n$ = "" THEN EvaluateNumbers$ = "ERROR - Invalid null " + OName(p): EXIT FUNCTION
        c1 = INSTR(n$, ",")
        IF c1 = 0 THEN EvaluateNumbers$ = "ERROR - " + OName(p) + " requires 2 parameters for Color, ScreenMode.": EXIT FUNCTION
        IF c1 THEN c2 = INSTR(c1 + 1, n$, ",")
        IF c2 THEN EvaluateNumbers$ = "ERROR - " + OName(p) + " requires 2 parameters for Color, ScreenMode.": EXIT FUNCTION
        n = VAL(LEFT$(num(2), c1))
        n2 = VAL(MID$(num(2), c1 + 1))
        SELECT CASE n2
        CASE 0 TO 2, 7 TO 13, 256, 32 'these are the good screen values
        CASE ELSE
            EvaluateNumbers$ = "ERROR - Invalid Screen Mode (" + STR$(n2) + ")": EXIT FUNCTION
        END SELECT
        t = _NEWIMAGE(1, 1, n4)
        SELECT CASE OName(p)
        CASE "_RED": n1 = _RED(n, t)
        CASE "_BLUE": n1 = _BLUE(n, t)
        CASE "_GREEN": n1 = _GREEN(n, t)
        CASE "_ALPHA": n1 = _ALPHA(n, t)
        END SELECT
        _FREEIMAGE t
    CASE "C_RX", "C_GR", "C_BL", "C_AL"
        n$ = num(2)
        IF n$ = "" THEN EvaluateNumbers$ = "ERROR - Invalid null " + OName(p): EXIT FUNCTION
        n = VAL(num(2))
        SELECT CASE OName(p)
        CASE "C_RX": n1 = _RED32(n)
        CASE "C_BL": n1 = _BLUE32(n)
        CASE "C_GR": n1 = _GREEN32(n)
        CASE "C_AL": n1 = _ALPHA32(n)
        END SELECT
    CASE "COS": n1 = COS(VAL(num(2)))
    CASE "SIN": n1 = SIN(VAL(num(2)))
    CASE "TAN": n1 = TAN(VAL(num(2)))
    CASE "LOG": n1 = LOG(VAL(num(2)))
    CASE "EXP": n1 = EXP(VAL(num(2)))
    CASE "ATN": n1 = ATN(VAL(num(2)))
    CASE "_D2R": n1 = 0.0174532925 * (VAL(num(2)))
    CASE "_D2G": n1 = 1.1111111111 * (VAL(num(2)))
    CASE "_R2D": n1 = 57.2957795 * (VAL(num(2)))
    CASE "_R2G": n1 = 0.015707963 * (VAL(num(2)))
    CASE "_G2D": n1 = 0.9 * (VAL(num(2)))
    CASE "_G2R": n1 = 63.661977237 * (VAL(num(2)))
    CASE "ABS": n1 = ABS(VAL(num(2)))
    CASE "SGN": n1 = SGN(VAL(num(2)))
    CASE "INT": n1 = INT(VAL(num(2)))
    CASE "_ROUND": n1 = _ROUND(VAL(num(2)))
    CASE "_CEIL": n1 = _CEIL(VAL(num(2)))
    CASE "FIX": n1 = FIX(VAL(num(2)))
    CASE "_SEC": n1 = _SEC(VAL(num(2)))
    CASE "_CSC": n1 = _CSC(VAL(num(2)))
    CASE "_COT": n1 = _COT(VAL(num(2)))
    END SELECT
CASE 20 TO 60 'Math Operators
    SELECT CASE OName(p) 'Depending on our operator..
    CASE "^": n1 = VAL(num(1)) ^ VAL(num(2))
    CASE "SQR": n1 = SQR(VAL(num(2)))
    CASE "ROOT"
        n1 = VAL(num(1)): n2 = VAL(num(2))
        IF n2 = 1 THEN EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1))): EXIT FUNCTION
        IF n1 < 0 AND n2 >= 1 THEN sign = -1: n1 = -n1 ELSE sign = 1
        n3 = 1## / n2
        IF n3 <> INT(n3) AND n2 < 1 THEN sign = SGN(n1): n1 = ABS(n1)
        n1 = sign * (n1 ^ n3)
    CASE "*": n1 = VAL(num(1)) * VAL(num(2))
    CASE "/"
        IF VAL(num(2)) <> 0 THEN
            n1 = VAL(num(1)) / VAL(num(2))
        ELSE
            EvaluateNumbers$ = "ERROR - Division By Zero"
            EXIT FUNCTION
        END IF
    CASE "\"
        IF FIX(VAL(num(2))) = 0 THEN
            EvaluateNumbers$ = "ERROR - Division By Zero"
            EXIT FUNCTION
        END IF

        n1 = VAL(num(1)) \ VAL(num(2))
    CASE "MOD"
        IF FIX(VAL(num(2))) = 0 THEN
            EvaluateNumbers$ = "ERROR - Division By Zero"
            EXIT FUNCTION
        END IF

        n1 = VAL(num(1)) MOD VAL(num(2))

    CASE "+": n1 = VAL(num(1)) + VAL(num(2))
    CASE "-":
        n1 = VAL(num(1)) - VAL(num(2))
    END SELECT
CASE 70 'Relational Operators =, >, <, <>, <=, >=
    SELECT CASE OName(p) 'Depending on our operator..
    CASE "=": n1 = VAL(num(1)) = VAL(num(2))
    CASE ">": n1 = VAL(num(1)) > VAL(num(2))
    CASE "<": n1 = VAL(num(1)) < VAL(num(2))
    CASE "<>", "><": n1 = VAL(num(1)) <> VAL(num(2))
    CASE "<=", "=<": n1 = VAL(num(1)) <= VAL(num(2))
    CASE ">=", "=>": n1 = VAL(num(1)) >= VAL(num(2))
    END SELECT
CASE ELSE 'a value we haven't processed elsewhere
    SELECT CASE OName(p) 'Depending on our operator..
    CASE "NOT": n1 = NOT VAL(num(2))
    CASE "AND": n1 = VAL(num(1)) AND VAL(num(2))
    CASE "OR": n1 = VAL(num(1)) OR VAL(num(2))
    CASE "XOR": n1 = VAL(num(1)) XOR VAL(num(2))
    CASE "EQV": n1 = VAL(num(1)) EQV VAL(num(2))
    CASE "IMP": n1 = VAL(num(1)) IMP VAL(num(2))
    END SELECT
END SELECT

EvaluateNumbers$ = RTRIM$(LTRIM$(STR$(n1))) + C$
END FUNCTION

FUNCTION DWD$ (exp$) 'Deal With Duplicates
    'To deal with duplicate operators in our code.
    'Such as --  becomes a +
    '++ becomes a +
    '+- becomes a -
    '-+ becomes a -
    t$ = exp$
    DO
        bad = 0
        DO
            l = INSTR(t$, "++")
            IF l THEN t$ = LEFT$(t$, l - 1) + "+" + MID$(t$, l + 2): bad = -1
        LOOP UNTIL l = 0
        DO
            l = INSTR(t$, "+-")
            IF l THEN t$ = LEFT$(t$, l - 1) + "-" + MID$(t$, l + 2): bad = -1
        LOOP UNTIL l = 0
        DO
            l = INSTR(t$, "-+")
            IF l THEN t$ = LEFT$(t$, l - 1) + "-" + MID$(t$, l + 2): bad = -1
        LOOP UNTIL l = 0
        DO
            l = INSTR(t$, "--")
            IF l THEN t$ = LEFT$(t$, l - 1) + "+" + MID$(t$, l + 2): bad = -1
        LOOP UNTIL l = 0
    LOOP UNTIL NOT bad
    DWD$ = t$
END FUNCTION

SUB PreParse (e$)
    DIM f AS _FLOAT
    STATIC TotalPrefixedPP_TypeMod AS LONG, TotalPP_TypeMod AS LONG

    IF PP_TypeMod(0) = "" THEN
        REDIM PP_TypeMod(100) AS STRING, PP_ConvertedMod(100) AS STRING 'Large enough to hold all values to begin with
        PP_TypeMod(0) = "Initialized" 'Set so we don't do this section over and over, as we keep the values in shared memory.
        'and the below is a conversion list so symbols don't get cross confused.
        i = i + 1: PP_TypeMod(i) = "~`": PP_ConvertedMod(i) = "C_UBI" 'unsigned bit
        i = i + 1: PP_TypeMod(i) = "~%%": PP_ConvertedMod(i) = "C_UBY" 'unsigned byte
        i = i + 1: PP_TypeMod(i) = "~%&": PP_ConvertedMod(i) = "C_UOF" 'unsigned offset
        i = i + 1: PP_TypeMod(i) = "~%": PP_ConvertedMod(i) = "C_UIN" 'unsigned integer
        i = i + 1: PP_TypeMod(i) = "~&&": PP_ConvertedMod(i) = "C_UIF" 'unsigned integer64
        i = i + 1: PP_TypeMod(i) = "~&": PP_ConvertedMod(i) = "C_ULO" 'unsigned long
        i = i + 1: PP_TypeMod(i) = "`": PP_ConvertedMod(i) = "C_BI" 'bit
        i = i + 1: PP_TypeMod(i) = "%%": PP_ConvertedMod(i) = "C_BY" 'byte
        i = i + 1: PP_TypeMod(i) = "%&": PP_ConvertedMod(i) = "C_OF" 'offset
        i = i + 1: PP_TypeMod(i) = "%": PP_ConvertedMod(i) = "C_IN" 'integer
        i = i + 1: PP_TypeMod(i) = "&&": PP_ConvertedMod(i) = "C_IF" 'integer64
        i = i + 1: PP_TypeMod(i) = "&": PP_ConvertedMod(i) = "C_LO" 'long
        i = i + 1: PP_TypeMod(i) = "!": PP_ConvertedMod(i) = "C_SI" 'single
        i = i + 1: PP_TypeMod(i) = "##": PP_ConvertedMod(i) = "C_FL" 'float
        i = i + 1: PP_TypeMod(i) = "#": PP_ConvertedMod(i) = "C_DO" 'double
        i = i + 1: PP_TypeMod(i) = "_RGB32": PP_ConvertedMod(i) = "C_RG" 'rgb32
        i = i + 1: PP_TypeMod(i) = "_RGBA32": PP_ConvertedMod(i) = "C_RA" 'rgba32
        i = i + 1: PP_TypeMod(i) = "_RED32": PP_ConvertedMod(i) = "C_RX" 'red32
        i = i + 1: PP_TypeMod(i) = "_GREEN32": PP_ConvertedMod(i) = "C_GR" 'green32
        i = i + 1: PP_TypeMod(i) = "_BLUE32": PP_ConvertedMod(i) = "C_BL" 'blue32
        i = i + 1: PP_TypeMod(i) = "_ALPHA32": PP_ConvertedMod(i) = "C_AL" 'alpha32
        TotalPrefixedPP_TypeMod = i
        i = i + 1: PP_TypeMod(i) = "RGB32": PP_ConvertedMod(i) = "C_RG" 'rgb32
        i = i + 1: PP_TypeMod(i) = "RGBA32": PP_ConvertedMod(i) = "C_RA" 'rgba32
        i = i + 1: PP_TypeMod(i) = "RED32": PP_ConvertedMod(i) = "C_RX" 'red32
        i = i + 1: PP_TypeMod(i) = "GREEN32": PP_ConvertedMod(i) = "C_GR" 'green32
        i = i + 1: PP_TypeMod(i) = "BLUE32": PP_ConvertedMod(i) = "C_BL" 'blue32
        i = i + 1: PP_TypeMod(i) = "ALPHA32": PP_ConvertedMod(i) = "C_AL" 'alpha32
        TotalPP_TypeMod = i
        REDIM _PRESERVE PP_TypeMod(i) AS STRING, PP_ConvertedMod(i) AS STRING 'And then resized to just contain the necessary space in memory
    END IF
    t$ = e$

    'First strip all spaces
    t$ = ""
    FOR i = 1 TO LEN(e$)
        IF MID$(e$, i, 1) <> " " THEN t$ = t$ + MID$(e$, i, 1)
    NEXT

    t$ = UCASE$(t$)
    IF t$ = "" THEN e$ = "ERROR -- NULL string; nothing to evaluate": EXIT SUB

    'ERROR CHECK by counting our brackets
    l = 0
    DO
        l = INSTR(l + 1, t$, "("): IF l THEN c = c + 1
    LOOP UNTIL l = 0
    l = 0
    DO
        l = INSTR(l + 1, t$, ")"): IF l THEN c1 = c1 + 1
    LOOP UNTIL l = 0
    IF c <> c1 THEN e$ = "ERROR -- Bad Parenthesis:" + STR$(c) + "( vs" + STR$(c1) + ")": EXIT SUB

    'replace existing CONST values
    sep$ = "()+-*/\><=^"
    FOR i2 = 0 TO constlast
        thisConstName$ = constname(i2)
        FOR replaceConstPass = 1 TO 2
            found = 0
            DO
                found = INSTR(found + 1, UCASE$(t$), thisConstName$)
                IF found THEN
                    IF found > 1 THEN
                        IF INSTR(sep$, MID$(t$, found - 1, 1)) = 0 THEN _CONTINUE
                    END IF
                    IF found + LEN(thisConstName$) <= LEN(t$) THEN
                        IF INSTR(sep$, MID$(t$, found + LEN(thisConstName$), 1)) = 0 THEN _CONTINUE
                    END IF
                    t = consttype(i2)
                    IF t AND ISSTRING THEN
                        r$ = conststring(i2)
                        i4 = _INSTRREV(r$, ",")
                        r$ = LEFT$(r$, i4 - 1)
                    ELSE
                        IF t AND ISFLOAT THEN
                            r$ = STR$(constfloat(i2))
                            r$ = N2S(r$)
                        ELSE
                            IF t AND ISUNSIGNED THEN r$ = STR$(constuinteger(i2)) ELSE r$ = STR$(constinteger(i2))
                        END IF
                    END IF
                    t$ = LEFT$(t$, found - 1) + _TRIM$(r$) + MID$(t$, found + LEN(thisConstName$))
                END IF
            LOOP UNTIL found = 0
            thisConstName$ = constname(i2) + constnamesymbol(i2)
        NEXT
    NEXT

    'Modify so that NOT will process properly
    l = 0
    DO
        l = INSTR(l + 1, t$, "NOT ")
        IF l THEN
            'We need to work magic on the statement so it looks pretty.
            ' 1 + NOT 2 + 1 is actually processed as 1 + (NOT 2 + 1)
            'Look for something not proper
            l1 = INSTR(l + 1, t$, "AND")
            IF l1 = 0 OR (INSTR(l + 1, t$, "OR") > 0 AND INSTR(l + 1, t$, "OR") < l1) THEN l1 = INSTR(l + 1, t$, "OR")
            IF l1 = 0 OR (INSTR(l + 1, t$, "XOR") > 0 AND INSTR(l + 1, t$, "XOR") < l1) THEN l1 = INSTR(l + 1, t$, "XOR")
            IF l1 = 0 OR (INSTR(l + 1, t$, "EQV") > 0 AND INSTR(l + 1, t$, "EQV") < l1) THEN l1 = INSTR(l + 1, t$, "EQV")
            IF l1 = 0 OR (INSTR(l + 1, t$, "IMP") > 0 AND INSTR(l + 1, t$, "IMP") < l1) THEN l1 = INSTR(l + 1, t$, "IMP")
            IF l1 = 0 THEN l1 = LEN(t$) + 1
            t$ = LEFT$(t$, l - 1) + "(" + MID$(t$, l, l1 - l) + ")" + MID$(t$, l + l1 - l)
            l = l + 3
            'PRINT t$
        END IF
    LOOP UNTIL l = 0

    uboundPP_TypeMod = TotalPrefixedPP_TypeMod
    IF qbnexprefix_set = 1 THEN uboundPP_TypeMod = TotalPP_TypeMod
    FOR j = 1 TO uboundPP_TypeMod
        l = 0
        DO
            l = INSTR(l + 1, t$, PP_TypeMod(j))
            IF l = 0 THEN EXIT DO
            i = 0: l1 = 0: l2 = 0: lo = LEN(PP_TypeMod(j))
            DO
                IF PL(i) > 10 THEN
                    l2 = _INSTRREV(l, t$, OName$(i))
                    IF l2 > 0 AND l2 > l1 THEN l1 = l2
                END IF
                i = i + lo
            LOOP UNTIL i > UBOUND(PL)
            l$ = LEFT$(t$, l1)
            m$ = MID$(t$, l1 + 1, l - l1 - 1)
            r$ = PP_ConvertedMod(j) + MID$(t$, l + lo)
            IF j > 15 THEN
                t$ = l$ + m$ + r$ 'replacement routine for commands which might get confused with others, like _RGB and _RGB32
            ELSE
                'the first 15 commands need to properly place the parenthesis around the value we want to convert.
                t$ = l$ + "(" + m$ + ")" + r$
            END IF
            l = l + 2 + LEN(PP_TypeMod(j)) 'move forward from the length of the symbol we checked + the new "(" and  ")"
        LOOP
    NEXT

    'Check for bad operators before a ( bracket
    l = 0
    DO
        l = INSTR(l + 1, t$, "(")
        IF l > 0 AND l > 2 THEN 'Don't check the starting bracket; there's nothing before it.
        good = 0
        FOR i = 1 TO UBOUND(OName)
            m$ = MID$(t$, l - LEN(OName(i)), LEN(OName(i)))
            IF m$ = OName(i) THEN
                good = -1: EXIT FOR 'We found an operator after our ), and it's not a CONST (like PI)
            ELSE
                IF LEFT$(OName(i), 1) = "_" AND qbnexprefix_set = 1 THEN
                    'try without prefix
                    m$ = MID$(t$, l - (LEN(OName(i)) - 1), LEN(OName(i)) - 1)
                    IF m$ = MID$(OName(i), 2) THEN good = -1: EXIT FOR
                END IF
            END IF
        NEXT
        IF NOT good THEN e$ = "ERROR - Improper operations before (.": EXIT SUB
        l = l + 1
    END IF
LOOP UNTIL l = 0

'Check for bad operators after a ) bracket
l = 0
DO
    l = INSTR(l + 1, t$, ")")
    IF l > 0 AND l < LEN(t$) THEN
        good = 0
        FOR i = 1 TO UBOUND(OName)
            m$ = MID$(t$, l + 1, LEN(OName(i)))
            IF m$ = OName(i) THEN
                good = -1: EXIT FOR 'We found an operator after our ), and it's not a CONST (like PI
            ELSE
                IF LEFT$(OName(i), 1) = "_" AND qbnexprefix_set = 1 THEN
                    'try without prefix
                    m$ = MID$(t$, l + 1, LEN(OName(i)) - 1)
                    IF m$ = MID$(OName(i), 2) THEN good = -1: EXIT FOR
                END IF
            END IF
        NEXT
        IF MID$(t$, l + 1, 1) = ")" THEN good = -1
        IF NOT good THEN e$ = "ERROR - Improper operations after ).": EXIT SUB
        l = l + 1
    END IF
LOOP UNTIL l = 0 OR l = LEN(t$) 'last symbol is a bracket

'Turn all &H (hex) numbers into decimal values for the program to process properly
l = 0
DO
    l = INSTR(t$, "&H")
    IF l THEN
        E = l + 1: finished = 0
        DO
            E = E + 1
            comp$ = MID$(t$, E, 1)
            SELECT CASE comp$
            CASE "0" TO "9", "A" TO "F" 'All is good, our next digit is a number, continue to add to the hex$
            CASE ELSE
                good = 0
                FOR i = 1 TO UBOUND(OName)
                    IF MID$(t$, E, LEN(OName(i))) = OName(i) AND PL(i) > 1 AND PL(i) <= 250 THEN good = -1: EXIT FOR 'We found an operator after our ), and it's not a CONST (like PI)
                NEXT
                IF NOT good THEN e$ = "ERROR - Improper &H value. (" + comp$ + ")": EXIT SUB
                E = E - 1
                finished = -1
            END SELECT
        LOOP UNTIL finished OR E = LEN(t$)
        t$ = LEFT$(t$, l - 1) + LTRIM$(RTRIM$(STR$(VAL(MID$(t$, l, E - l + 1))))) + MID$(t$, E + 1)
    END IF
LOOP UNTIL l = 0

'Turn all &B (binary) numbers into decimal values for the program to process properly
l = 0
DO
    l = INSTR(t$, "&B")
    IF l THEN
        E = l + 1: finished = 0
        DO
            E = E + 1
            comp$ = MID$(t$, E, 1)
            SELECT CASE comp$
            CASE "0", "1" 'All is good, our next digit is a number, continue to add to the hex$
            CASE ELSE
                good = 0
                FOR i = 1 TO UBOUND(OName)
                    IF MID$(t$, E, LEN(OName(i))) = OName(i) AND PL(i) > 1 AND PL(i) <= 250 THEN good = -1: EXIT FOR 'We found an operator after our ), and it's not a CONST (like PI)
                NEXT
                IF NOT good THEN e$ = "ERROR - Improper &B value. (" + comp$ + ")": EXIT SUB
                E = E - 1
                finished = -1
            END SELECT
        LOOP UNTIL finished OR E = LEN(t$)
        BIN$ = MID$(t$, l + 2, E - l - 1)
        FOR i = 1 TO LEN(BIN$)
            IF MID$(BIN$, i, 1) = "1" THEN f = f + 2 ^ (LEN(BIN$) - i)
        NEXT
        t$ = LEFT$(t$, l - 1) + LTRIM$(RTRIM$(STR$(f))) + MID$(t$, E + 1)
    END IF
LOOP UNTIL l = 0


't$ = N2S(t$)
VerifyString t$
e$ = t$
END SUB



SUB VerifyString (t$)
    'ERROR CHECK for unrecognized operations
    j = 1
    DO
        comp$ = MID$(t$, j, 1)
        SELECT CASE comp$
        CASE "0" TO "9", ".", "(", ")", ",": j = j + 1
        CASE ELSE
            good = 0
            extrachar = 0
            FOR i = 1 TO UBOUND(OName)
                IF MID$(t$, j, LEN(OName(i))) = OName(i) THEN
                    good = -1: EXIT FOR 'We found an operator after our ), and it's not a CONST (like PI)
                ELSE
                    IF LEFT$(OName(i), 1) = "_" AND qbnexprefix_set = 1 THEN
                        'try without prefix
                        IF MID$(t$, j, LEN(OName(i)) - 1) = MID$(OName(i), 2) THEN
                            good = -1: extrachar = 1: EXIT FOR
                        END IF
                    END IF
                END IF
            NEXT
            IF NOT good THEN t$ = "ERROR - Bad Operational value. (" + comp$ + ")": EXIT SUB
            j = j + (LEN(OName(i)) - extrachar)
        END SELECT
    LOOP UNTIL j > LEN(t$)
END SUB

FUNCTION N2S$ (exp$) 'scientific Notation to String

    t$ = LTRIM$(RTRIM$(exp$))
    IF LEFT$(t$, 1) = "-" OR LEFT$(t$, 1) = "N" THEN sign$ = "-": t$ = MID$(t$, 2)

    dp = INSTR(t$, "D+"): dm = INSTR(t$, "D-")
    ep = INSTR(t$, "E+"): em = INSTR(t$, "E-")
    check1 = SGN(dp) + SGN(dm) + SGN(ep) + SGN(em)
    IF check1 < 1 OR check1 > 1 THEN N2S = exp$: EXIT FUNCTION 'If no scientic notation is found, or if we find more than 1 type, it's not SN!

    SELECT CASE l 'l now tells us where the SN starts at.
    CASE IS < dp: l = dp
    CASE IS < dm: l = dm
    CASE IS < ep: l = ep
    CASE IS < em: l = em
    END SELECT

    l$ = LEFT$(t$, l - 1) 'The left of the SN
    r$ = MID$(t$, l + 1): r&& = VAL(r$) 'The right of the SN, turned into a workable long


    IF INSTR(l$, ".") THEN 'Location of the decimal, if any
    IF r&& > 0 THEN
        r&& = r&& - LEN(l$) + 2
    ELSE
        r&& = r&& + 1
    END IF
    l$ = LEFT$(l$, 1) + MID$(l$, 3)
END IF

SELECT CASE r&&
CASE 0 'what the heck? We solved it already?
    'l$ = l$
CASE IS < 0
    FOR i = 1 TO -r&&
        l$ = "0" + l$
    NEXT
    l$ = "0." + l$
CASE ELSE
    FOR i = 1 TO r&&
        l$ = l$ + "0"
    NEXT
END SELECT

N2S$ = sign$ + l$
END FUNCTION
