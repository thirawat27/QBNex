$CONSOLE:ONLY
' TCP: PUT / GET of a whole array, and EOF(handle) after a GET that found too few bytes.
ON ERROR GOTO eh
port$ = "47393"
host = _OPENHOST("TCP/IP:" + port$)
c = _OPENCLIENT("TCP/IP:" + port$ + ":localhost")
t! = TIMER
DO: s = _OPENCONNECTION(host): LOOP UNTIL s <> 0 OR ABS(TIMER - t!) > 5
DIM a(3) AS INTEGER, b(3) AS INTEGER
a(0) = 1: a(1) = 2: a(2) = 3: a(3) = 4
PUT #c, , a()
t! = TIMER
DO
    GET #s, , b()
LOOP UNTIL EOF(s) = 0 OR ABS(TIMER - t!) > 5
PRINT b(0); b(1); b(2); b(3); EOF(s)
GET #s, , b()
PRINT "nothing more"; EOF(s)
CLOSE c: CLOSE s: CLOSE host
SYSTEM
eh:
PRINT "error"; ERR
RESUME NEXT
