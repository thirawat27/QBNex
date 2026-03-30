' types.bas
TYPE Player
    Name AS STRING * 20
    Score AS LONG
    Health AS SINGLE
    Level AS INTEGER
END TYPE

DIM player1 AS Player
DIM player2 AS Player

player1.Name = "Alice"
player1.Score = 1500
player1.Health = 100.0
player1.Level = 5

player2.Name = "Bob"
player2.Score = 2300
player2.Health = 85.5
player2.Level = 7

PRINT "Player 1"
PRINT "  Name "; player1.Name
PRINT "  Score "; player1.Score
PRINT "  Health "; player1.Health
PRINT "  Level "; player1.Level
PRINT

PRINT "Player 2"
PRINT "  Name "; player2.Name
PRINT "  Score "; player2.Score
PRINT "  Health "; player2.Health
PRINT "  Level "; player2.Level
