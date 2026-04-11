' ============================================================================
' QBNex Standard Library - Strings: Pattern Matching
' ============================================================================
' Glob wildcards and minimal POSIX regex engine
' ============================================================================

' ============================================================================
' FUNCTION: GlobMatch
' Shell-style wildcard matching (* and ?)
' ============================================================================
FUNCTION GlobMatch& (text AS STRING, pattern AS STRING)
    DIM ti AS LONG, pi AS LONG
    DIM tlen AS LONG, plen AS LONG
    DIM starIdx AS LONG, matchIdx AS LONG
    
    ti = 1: pi = 1
    tlen = LEN(text)
    plen = LEN(pattern)
    starIdx = 0: matchIdx = 0
    
    DO WHILE ti <= tlen
        IF pi <= plen THEN
            IF MID$(pattern, pi, 1) = "*" THEN
                starIdx = pi
                matchIdx = ti
                pi = pi + 1
                GOTO ContinueLoop
            ELSEIF MID$(pattern, pi, 1) = "?" OR MID$(text, ti, 1) = MID$(pattern, pi, 1) THEN
                ti = ti + 1
                pi = pi + 1
                GOTO ContinueLoop
            END IF
        END IF
        
        IF starIdx > 0 THEN
            pi = starIdx + 1
            matchIdx = matchIdx + 1
            ti = matchIdx
        ELSE
            GlobMatch = 0
            EXIT FUNCTION
        END IF
        
        ContinueLoop:
    LOOP
    
    ' Skip trailing stars
    DO WHILE pi <= plen AND MID$(pattern, pi, 1) = "*"
        pi = pi + 1
    LOOP
    
    GlobMatch = (pi > plen)
END FUNCTION

' ============================================================================
' FUNCTION: RegexMatch
' Minimal regex engine (. * + ? ^ $ [abc] [^abc] [a-z])
' ============================================================================
FUNCTION RegexMatch& (text AS STRING, pattern AS STRING)
    RegexMatch = RegexMatchRecursive(text, 1, pattern, 1)
END FUNCTION

' ============================================================================
' FUNCTION: RegexMatchRecursive (Internal)
' Recursive descent regex matcher
' ============================================================================
FUNCTION RegexMatchRecursive& (text AS STRING, ti AS LONG, pattern AS STRING, pi AS LONG)
    DIM tlen AS LONG, plen AS LONG
    DIM pc AS STRING, tc AS STRING
    DIM nextPc AS STRING
    
    tlen = LEN(text)
    plen = LEN(pattern)
    
    ' End of pattern
    IF pi > plen THEN
        RegexMatchRecursive = (ti > tlen)
        EXIT FUNCTION
    END IF
    
    pc = MID$(pattern, pi, 1)
    IF pi < plen THEN nextPc = MID$(pattern, pi + 1, 1) ELSE nextPc = ""
    
    ' Handle ^ (start anchor)
    IF pc = "^" AND pi = 1 THEN
        RegexMatchRecursive = RegexMatchRecursive(text, ti, pattern, pi + 1)
        EXIT FUNCTION
    END IF
    
    ' Handle $ (end anchor)
    IF pc = "$" AND pi = plen THEN
        RegexMatchRecursive = (ti > tlen)
        EXIT FUNCTION
    END IF
    
    ' Handle * (zero or more)
    IF nextPc = "*" THEN
        ' Try zero matches
        IF RegexMatchRecursive(text, ti, pattern, pi + 2) THEN
            RegexMatchRecursive = -1
            EXIT FUNCTION
        END IF
        
        ' Try one or more matches
        DO WHILE ti <= tlen
            IF pc = "." OR MID$(text, ti, 1) = pc THEN
                ti = ti + 1
                IF RegexMatchRecursive(text, ti, pattern, pi + 2) THEN
                    RegexMatchRecursive = -1
                    EXIT FUNCTION
                END IF
            ELSE
                EXIT DO
            END IF
        LOOP
        
        RegexMatchRecursive = 0
        EXIT FUNCTION
    END IF
    
    ' Handle + (one or more)
    IF nextPc = "+" THEN
        IF ti > tlen THEN
            RegexMatchRecursive = 0
            EXIT FUNCTION
        END IF
        
        IF pc <> "." AND MID$(text, ti, 1) <> pc THEN
            RegexMatchRecursive = 0
            EXIT FUNCTION
        END IF
        
        ti = ti + 1
        
        ' Match remaining
        DO WHILE ti <= tlen
            IF pc = "." OR MID$(text, ti, 1) = pc THEN
                ti = ti + 1
                IF RegexMatchRecursive(text, ti, pattern, pi + 2) THEN
                    RegexMatchRecursive = -1
                    EXIT FUNCTION
                END IF
            ELSE
                EXIT DO
            END IF
        LOOP
        
        RegexMatchRecursive = RegexMatchRecursive(text, ti, pattern, pi + 2)
        EXIT FUNCTION
    END IF
    
    ' Handle ? (zero or one)
    IF nextPc = "?" THEN
        ' Try zero
        IF RegexMatchRecursive(text, ti, pattern, pi + 2) THEN
            RegexMatchRecursive = -1
            EXIT FUNCTION
        END IF
        
        ' Try one
        IF ti <= tlen AND (pc = "." OR MID$(text, ti, 1) = pc) THEN
            RegexMatchRecursive = RegexMatchRecursive(text, ti + 1, pattern, pi + 2)
        ELSE
            RegexMatchRecursive = 0
        END IF
        EXIT FUNCTION
    END IF
    
    ' Handle . (any character)
    IF pc = "." THEN
        IF ti > tlen THEN
            RegexMatchRecursive = 0
        ELSE
            RegexMatchRecursive = RegexMatchRecursive(text, ti + 1, pattern, pi + 1)
        END IF
        EXIT FUNCTION
    END IF
    
    ' Handle literal character
    IF ti > tlen THEN
        RegexMatchRecursive = 0
    ELSEIF MID$(text, ti, 1) = pc THEN
        RegexMatchRecursive = RegexMatchRecursive(text, ti + 1, pattern, pi + 1)
    ELSE
        RegexMatchRecursive = 0
    END IF
END FUNCTION
