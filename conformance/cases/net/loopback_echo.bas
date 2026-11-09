$CONSOLE:ONLY
' TCP loopback (qbrt::net): host, client and the host's connection in one program.
port$ = "47391"
host = _OPENHOST("TCP/IP:" + port$)
PRINT "host opened"; host < -1
PRINT "host connected"; _CONNECTED(host)
PRINT "nothing pending"; _OPENCONNECTION(host)
client = _OPENCLIENT("TCP/IP:" + port$ + ":localhost")
PRINT "client opened"; client < -1
PRINT _CONNECTIONADDRESS(client)
PRINT "client connected"; _CONNECTED(client)
t! = TIMER
DO
    server = _OPENCONNECTION(host)
LOOP UNTIL server <> 0 OR ABS(TIMER - t!) > 5
PRINT "accepted"; server < -1
a$ = _CONNECTIONADDRESS(server)
PRINT LEFT$(a$, 7); "<port>"; MID$(a$, INSTR(8, a$, ":"))

' client -> server, binary-safe
msg$ = "hello" + CHR$(0) + CHR$(255) + CHR$(13) + CHR$(10) + "world"
PUT #client, , msg$
got$ = ""
t! = TIMER
DO
    GET #server, , part$
    got$ = got$ + part$
LOOP UNTIL LEN(got$) >= LEN(msg$) OR ABS(TIMER - t!) > 5
PRINT "server got"; LEN(got$); got$ = msg$

' server -> client, larger than one receive buffer
big$ = ""
FOR i = 1 TO 5000
    big$ = big$ + CHR$(i MOD 256)
NEXT
big$ = big$ + big$ + big$ + big$
PUT #server, , big$
got$ = ""
t! = TIMER
DO
    GET #client, , part$
    got$ = got$ + part$
LOOP UNTIL LEN(got$) >= LEN(big$) OR ABS(TIMER - t!) > 5
PRINT "client got"; LEN(got$); got$ = big$

' nothing more to read: GET returns an empty string without blocking
GET #client, , part$
PRINT "empty read"; LEN(part$)

' client closes: the server notices on its next read
CLOSE client
t! = TIMER
DO
    GET #server, , part$
LOOP UNTIL _CONNECTED(server) = 0 OR ABS(TIMER - t!) > 5
PRINT "server sees close"; _CONNECTED(server)
CLOSE server
CLOSE host
PRINT "done"
SYSTEM
