$CONSOLE:ONLY
' CLOSE without a file number closes the TCP/IP handles too: the handles are gone and the port is free again.
ON ERROR GOTO eh
port$ = "47395"
host = _OPENHOST("TCP/IP:" + port$)
c = _OPENCLIENT("TCP/IP:" + port$ + ":localhost")
OPEN "close_all_tcp.tmp" FOR OUTPUT AS #1
PRINT "open"; host < 0; c < 0
CLOSE
PRINT "client"; _CONNECTED(c)
PRINT "host"; _CONNECTED(host)
PRINT #1, "x"
h2 = _OPENHOST("TCP/IP:" + port$)
PRINT "reopen"; h2 < 0
CLOSE h2
KILL "close_all_tcp.tmp"
SYSTEM
eh:
PRINT "error"; ERR
RESUME NEXT
