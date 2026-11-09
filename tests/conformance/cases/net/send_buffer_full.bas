$CONSOLE:ONLY
' PUT on a network handle whose send buffer is full (nobody reads): the connection must stay usable.
host = _OPENHOST("TCP/IP:47396")
client = _OPENCLIENT("TCP/IP:47396:127.0.0.1")
t! = TIMER
DO: server = _OPENCONNECTION(host): LOOP UNTIL server <> 0 OR ABS(TIMER - t!) > 5
block$ = STRING$(1048576, "x")
FOR i = 1 TO 64
    PUT #client, , block$
NEXT
PRINT "client still connected"; _CONNECTED(client)
CLOSE client: CLOSE server: CLOSE host
SYSTEM
