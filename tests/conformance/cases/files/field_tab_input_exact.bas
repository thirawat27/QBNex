$CONSOLE:ONLY
' FIELD on a variable of a SUB ends with the SUB, overlapping FIELDs show each
' other's LSET, OPEN "R", #n, file$, reclen uses the record length, TAB in PRINT #
' counts from the file's column, READ wraps _BIT values, INPUT # reads 64-bit
' integers exactly, a literal past 64 bits is a floating-point value, and PUT / GET
' take a whole array.
OPEN "fd.dat" FOR RANDOM AS #1 LEN = 10
FIELD #1, 10 AS w$
LSET w$ = "0123456789": PUT #1, 1
callee
FOR i = 1 TO 50
  s$ = STRING$(i, "x")
  GET #1, 1
NEXT
PRINT "got "; w$; LEN(s$)
FIELD #1, 4 AS a$, 6 AS b$
FIELD #1, 10 AS whole$
LSET a$ = "ABCD": LSET b$ = "123456"
PRINT "["; whole$; "]"
CLOSE #1: KILL "fd.dat"
OPEN "R", 3, "x.x", 4
x% = 5
PUT #3, 3, x%
PRINT LOF(3)
FIELD #3, 4 AS k$
CLOSE : KILL "x.x"
OPEN "t.txt" FOR OUTPUT AS #1
PRINT #1, "Name"; TAB(12); "Qty"; TAB(20); "Price"
PRINT #1, "abcdef"; TAB(3); "y"
CLOSE
OPEN "t.txt" FOR INPUT AS #1
DO UNTIL EOF(1): LINE INPUT #1, l$: PRINT "["; l$; "]": LOOP
CLOSE : KILL "t.txt"
DIM bt AS _BIT, u3 AS _UNSIGNED _BIT * 3, b3 AS _BIT * 3
READ bt, u3, b3
PRINT bt; u3; b3
DATA 1,9,7
DIM a AS _INTEGER64, c AS _INTEGER64
OPEN "i.txt" FOR OUTPUT AS #1
PRINT #1, "9007199254740993,-9223372036854775807"
WRITE #1, 1234567890123456789
PRINT #1, "9223372036854775807"
PRINT #1, "2.5,12e3"
CLOSE
OPEN "i.txt" FOR INPUT AS #1
INPUT #1, a, c: PRINT a; c
INPUT #1, a: PRINT a
INPUT #1, a: PRINT a
INPUT #1, a, c: PRINT a; c
CLOSE : KILL "i.txt"
x# = 18446744073709551616
PRINT x#
DIM arr(1 TO 3) AS INTEGER, back(1 TO 3) AS INTEGER
arr(1) = 11: arr(2) = 22: arr(3) = 33
OPEN "g.dat" FOR BINARY AS #1
PUT #1, , arr()
GET #1, 1, back()
PRINT LOF(1); back(1); back(2); back(3)
CLOSE : KILL "g.dat"
SYSTEM
SUB callee
  FIELD #1, 5 AS p$, 5 AS q$
  GET #1, 1
  PRINT "sub:"; p$; "|"; q$
END SUB

