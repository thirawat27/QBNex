OPEN "release_validate.tmp" FOR OUTPUT AS #1
PRINT #1, "QBNex-123"
CLOSE #1

OPEN "release_validate.tmp" FOR INPUT AS #1
LINE INPUT #1, line$
CLOSE #1

PRINT line$
