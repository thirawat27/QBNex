$CONSOLE:ONLY
' Network opens that fail without an error (qbrt::net): they return 0.
PRINT "port too big"; _OPENHOST("TCP/IP:70000")
PRINT "negative port"; _OPENHOST("TCP/IP:-1")
PRINT "client port too big"; _OPENCLIENT("TCP/IP:65536:localhost")
PRINT "nobody listening"; _OPENCLIENT("TCP/IP:47393:127.0.0.1")
PRINT "unknown host"; _OPENCLIENT("TCP/IP:80:no-such-host.invalid")
h = _OPENHOST("tcp/ip:47394")
PRINT "lower case kind"; h < -1
PRINT "port in use"; _OPENHOST("TCP/IP:47394")
PRINT "no client yet"; _OPENCONNECTION(h)
CLOSE h
h2 = _OPENHOST("TCP/IP:47394")
PRINT "reopen after close"; h2 < -1
CLOSE h2
' VAL-style port text: leading blanks, rounding, &H
h3 = _OPENHOST("TCP/IP: 47394.4")
PRINT "rounded port"; h3 < -1
c = _OPENCLIENT("TCP/IP:&HB922:127.0.0.1")
PRINT "hex port"; c < -1
PRINT _CONNECTIONADDRESS(c)
CLOSE c
CLOSE h3
SYSTEM
