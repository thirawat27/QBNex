$CONSOLE:ONLY
' Text is bytes: UTF-8 in and out, and a byte outside UTF-8 is a code page 437 character.
LINE INPUT a$
PRINT "สวัสดี "; a$; " "; CHR$(201); CHR$(205); LEN(a$)
SYSTEM
