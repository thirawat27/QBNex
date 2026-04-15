'===============================================================================
' QBNex Parser Module
'===============================================================================
' Lexical analysis and parsing for BASIC source code.
' Tokenizes source and builds an abstract syntax tree (AST).
'===============================================================================

'-------------------------------------------------------------------------------
' TOKEN TYPES
'-------------------------------------------------------------------------------

CONST TOKEN_EOF = 0
CONST TOKEN_IDENTIFIER = 1
CONST TOKEN_NUMBER = 2
CONST TOKEN_STRING = 3
CONST TOKEN_KEYWORD = 4
CONST TOKEN_OPERATOR = 5
CONST TOKEN_SYMBOL = 6
CONST TOKEN_COMMENT = 7
CONST TOKEN_NEWLINE = 8
CONST TOKEN_WHITESPACE = 9

CONST MAX_TOKEN_LENGTH = 256

TYPE Token
    type AS INTEGER
    value AS STRING * MAX_TOKEN_LENGTH
    lineNum AS LONG
    colNum AS INTEGER
    flags AS LONG
END TYPE

'-------------------------------------------------------------------------------
' AST NODE TYPES
'-------------------------------------------------------------------------------

CONST AST_PROGRAM = 1
CONST AST_SUB = 2
CONST AST_FUNCTION = 3
CONST AST_DECLARATION = 4
CONST AST_ASSIGNMENT = 5
CONST AST_IF = 6
CONST AST_FOR = 7
CONST AST_WHILE = 8
CONST AST_DO = 9
CONST AST_SELECT = 10
CONST AST_CALL = 11
CONST AST_EXPRESSION = 12
CONST AST_LITERAL = 13
CONST AST_IDENTIFIER = 14
CONST AST_BINARY_OP = 15
CONST AST_UNARY_OP = 16

TYPE ASTNode
    nodeType AS INTEGER
    value AS STRING * 256
    lineNum AS LONG
    parent AS LONG
    firstChild AS LONG
    nextSibling AS LONG
    symbolRef AS LONG
    flags AS LONG
END TYPE

'-------------------------------------------------------------------------------
' PARSER STATE
'-------------------------------------------------------------------------------

TYPE ParserState
    sourceText AS STRING
    currentPos AS LONG
    currentLine AS LONG
    currentCol AS INTEGER
    currentToken AS Token
    lineCount AS LONG
    inComment AS _BYTE
    inString AS _BYTE
END TYPE

DIM SHARED Parser AS ParserState
DIM SHARED TokenBuffer(1 TO 100) AS Token
DIM SHARED TokenBufferCount AS INTEGER
DIM SHARED TokenBufferPos AS INTEGER
DIM SHARED ASTNodes(1 TO 10000) AS ASTNode
DIM SHARED ASTNodeCount AS LONG
DIM SHARED ASTRoot AS LONG

'-------------------------------------------------------------------------------
' KEYWORDS
'-------------------------------------------------------------------------------

DIM SHARED Keywords(1 TO 50) AS STRING
DIM SHARED KeywordCount AS INTEGER

SUB InitKeywords
    KeywordCount = 0
    
    ' Add BASIC keywords
    AddKeyword "SUB"
    AddKeyword "FUNCTION"
    AddKeyword "END"
    AddKeyword "IF"
    AddKeyword "THEN"
    AddKeyword "ELSE"
    AddKeyword "ELSEIF"
    AddKeyword "FOR"
    AddKeyword "TO"
    AddKeyword "STEP"
    AddKeyword "NEXT"
    AddKeyword "DO"
    AddKeyword "LOOP"
    AddKeyword "WHILE"
    AddKeyword "UNTIL"
    AddKeyword "WEND"
    AddKeyword "SELECT"
    AddKeyword "CASE"
    AddKeyword "DIM"
    AddKeyword "REDIM"
    AddKeyword "SHARED"
    AddKeyword "CONST"
    AddKeyword "STATIC"
    AddKeyword "COMMON"
    AddKeyword "DECLARE"
    AddKeyword "DEF"
    AddKeyword "DEFINT"
    AddKeyword "DEFLNG"
    AddKeyword "DEFSTR"
    AddKeyword "DEFDBL"
    AddKeyword "DEFSNG"
    AddKeyword "TYPE"
    AddKeyword "AS"
    AddKeyword "INTEGER"
    AddKeyword "LONG"
    AddKeyword "SINGLE"
    AddKeyword "DOUBLE"
    AddKeyword "STRING"
    AddKeyword "VARIANT"
    AddKeyword "_BYTE"
    AddKeyword "_INTEGER64"
    AddKeyword "_FLOAT"
    AddKeyword "_UNSIGNED"
    AddKeyword "GOTO"
    AddKeyword "GOSUB"
    AddKeyword "RETURN"
    AddKeyword "EXIT"
    AddKeyword "CALL"
    AddKeyword "PRINT"
    AddKeyword "INPUT"
END SUB

SUB AddKeyword (word AS STRING)
    IF KeywordCount < 50 THEN
        KeywordCount = KeywordCount + 1
        Keywords(KeywordCount) = UCASE$(word)
    END IF
END SUB

FUNCTION IsKeyword% (word AS STRING)
    DIM i AS INTEGER
    DIM uword AS STRING
    uword = UCASE$(word)
    
    FOR i = 1 TO KeywordCount
        IF Keywords(i) = uword THEN
            IsKeyword% = -1
            EXIT FUNCTION
        END IF
    NEXT
    IsKeyword% = 0
END FUNCTION

'-------------------------------------------------------------------------------
' PARSER INITIALIZATION
'-------------------------------------------------------------------------------

SUB InitParser
    Parser.sourceText = ""
    Parser.currentPos = 1
    Parser.currentLine = 1
    Parser.currentCol = 1
    Parser.lineCount = 0
    Parser.inComment = 0
    Parser.inString = 0
    
    TokenBufferCount = 0
    TokenBufferPos = 1
    
    ASTNodeCount = 0
    ASTRoot = 0
    
    InitKeywords
END SUB

SUB CleanupParser
    Parser.sourceText = ""
    TokenBufferCount = 0
    ASTNodeCount = 0
END SUB

'-------------------------------------------------------------------------------
' LEXICAL ANALYSIS (TOKENIZER)
'-------------------------------------------------------------------------------

SUB SetSource (source AS STRING)
    Parser.sourceText = source
    Parser.currentPos = 1
    Parser.currentLine = 1
    Parser.currentCol = 1
    Parser.lineCount = 1
    
    ' Count lines
    DIM i AS LONG
    FOR i = 1 TO LEN(source)
        IF MID$(source, i, 1) = CHR$(10) THEN
            Parser.lineCount = Parser.lineCount + 1
        END IF
    NEXT
END SUB

FUNCTION GetNextChar$
    IF Parser.currentPos > LEN(Parser.sourceText) THEN
        GetNextChar$ = ""
    ELSE
        GetNextChar$ = MID$(Parser.sourceText, Parser.currentPos, 1)
    END IF
END FUNCTION

SUB AdvanceChar
    IF Parser.currentPos <= LEN(Parser.sourceText) THEN
        IF MID$(Parser.sourceText, Parser.currentPos, 1) = CHR$(10) THEN
            Parser.currentLine = Parser.currentLine + 1
            Parser.currentCol = 1
        ELSE
            Parser.currentCol = Parser.currentCol + 1
        END IF
        Parser.currentPos = Parser.currentPos + 1
    END IF
END SUB

FUNCTION PeekChar$ (offset AS INTEGER)
    DIM pos AS LONG
    pos = Parser.currentPos + offset
    IF pos > LEN(Parser.sourceText) THEN
        PeekChar$ = ""
    ELSE
        PeekChar$ = MID$(Parser.sourceText, pos, 1)
    END IF
END FUNCTION

FUNCTION GetNextToken AS Token
    DIM tok AS Token
    DIM ch AS STRING
    DIM nextCh AS STRING
    
    tok.type = TOKEN_EOF
    tok.value = ""
    tok.lineNum = Parser.currentLine
    tok.colNum = Parser.currentCol
    tok.flags = 0
    
    ' Skip whitespace
    DO
        ch = GetNextChar$
        IF ch = "" THEN
            GetNextToken = tok
            EXIT FUNCTION
        END IF
        IF ch <> " " AND ch <> CHR$(9) THEN EXIT DO
        AdvanceChar
    LOOP
    
    ' Check for comment (')
    IF ch = "'" THEN
        tok.type = TOKEN_COMMENT
        tok.value = "'"
        AdvanceChar
        ' Read rest of line as comment
        DO
            ch = GetNextChar$
            IF ch = "" OR ch = CHR$(10) OR ch = CHR$(13) THEN EXIT DO
            tok.value = RTRIM$(tok.value) + ch
            AdvanceChar
        LOOP
        GetNextToken = tok
        EXIT FUNCTION
    END IF
    
    ' Check for REM statement
    IF UCASE$(ch + PeekChar$(1) + PeekChar$(2)) = "REM" THEN
        tok.type = TOKEN_COMMENT
        tok.value = "REM"
        AdvanceChar: AdvanceChar: AdvanceChar
        ' Read rest of line
        DO
            ch = GetNextChar$
            IF ch = "" OR ch = CHR$(10) OR ch = CHR$(13) THEN EXIT DO
            tok.value = RTRIM$(tok.value) + ch
            AdvanceChar
        LOOP
        GetNextToken = tok
        EXIT FUNCTION
    END IF
    
    ' Check for string literal
    IF ch = CHR$(34) THEN
        tok.type = TOKEN_STRING
        tok.value = ""
        AdvanceChar
        DO
            ch = GetNextChar$
            IF ch = "" THEN EXIT DO
            IF ch = CHR$(34) THEN
                AdvanceChar
                ' Check for double quote (escaped)
                IF GetNextChar$ = CHR$(34) THEN
                    tok.value = RTRIM$(tok.value) + CHR$(34)
                    AdvanceChar
                ELSE
                    EXIT DO
                END IF
            ELSE
                tok.value = RTRIM$(tok.value) + ch
                AdvanceChar
            END IF
        LOOP
        GetNextToken = tok
        EXIT FUNCTION
    END IF
    
    ' Check for number
    IF (ch >= "0" AND ch <= "9") OR (ch = "." AND PeekChar$(1) >= "0" AND PeekChar$(1) <= "9") THEN
        tok.type = TOKEN_NUMBER
        tok.value = ""
        DO
            ch = GetNextChar$
            IF ch = "" THEN EXIT DO
            IF (ch >= "0" AND ch <= "9") OR ch = "." OR UCASE$(ch) = "E" OR ch = "+" OR ch = "-" THEN
                tok.value = RTRIM$(tok.value) + ch
                AdvanceChar
            ELSE
                EXIT DO
            END IF
        LOOP
        GetNextToken = tok
        EXIT FUNCTION
    END IF
    
    ' Check for identifier
    IF (ch >= "A" AND ch <= "Z") OR (ch >= "a" AND ch <= "z") OR ch = "_" THEN
        tok.type = TOKEN_IDENTIFIER
        tok.value = ""
        DO
            ch = GetNextChar$
            IF ch = "" THEN EXIT DO
            IF (ch >= "A" AND ch <= "Z") OR (ch >= "a" AND ch <= "z") OR (ch >= "0" AND ch <= "9") OR ch = "_" OR ch = "$" OR ch = "%" OR ch = "!" OR ch = "#" OR ch = "&" THEN
                tok.value = RTRIM$(tok.value) + ch
                AdvanceChar
            ELSE
                EXIT DO
            END IF
        LOOP
        
        ' Check if it's a keyword
        IF IsKeyword%(RTRIM$(tok.value)) THEN
            tok.type = TOKEN_KEYWORD
        END IF
        
        GetNextToken = tok
        EXIT FUNCTION
    END IF
    
    ' Check for operators and symbols
    tok.type = TOKEN_SYMBOL
    tok.value = ch
    AdvanceChar
    
    ' Check for two-character operators
    nextCh = GetNextChar$
    SELECT CASE ch + nextCh
        CASE "<=", ">=", "<>", "->"
            tok.value = ch + nextCh
            AdvanceChar
    END SELECT
    
    ' Special case: operator characters
    SELECT CASE ch
        CASE "+", "-", "*", "/", "\\", "^", "=", "<", ">", "(", ")", ",", ";", ":", "."
            tok.type = TOKEN_OPERATOR
    END SELECT
    
    GetNextToken = tok
END FUNCTION

SUB AdvanceToken
    Parser.currentToken = GetNextToken
END SUB

FUNCTION CurrentTokenType%
    CurrentTokenType% = Parser.currentToken.type
END FUNCTION

FUNCTION CurrentTokenValue$
    CurrentTokenValue$ = RTRIM$(Parser.currentToken.value)
END FUNCTION

'-------------------------------------------------------------------------------
' TOKEN BUFFER (FOR LOOKAHEAD)
'-------------------------------------------------------------------------------

SUB BufferTokens (count AS INTEGER)
    DIM i AS INTEGER
    TokenBufferCount = 0
    TokenBufferPos = 1
    
    FOR i = 1 TO count
        IF TokenBufferCount < 100 THEN
            TokenBufferCount = TokenBufferCount + 1
            TokenBuffer(TokenBufferCount) = GetNextToken
        END IF
    NEXT
END SUB

FUNCTION PeekToken (offset AS INTEGER) AS Token
    DIM pos AS INTEGER
    pos = TokenBufferPos + offset - 1
    
    IF pos >= 1 AND pos <= TokenBufferCount THEN
        PeekToken = TokenBuffer(pos)
    ELSE
        PeekToken.type = TOKEN_EOF
        PeekToken.value = ""
    END IF
END FUNCTION

SUB ConsumeToken
    IF TokenBufferPos < TokenBufferCount THEN
        TokenBufferPos = TokenBufferPos + 1
    END IF
    IF TokenBufferPos <= TokenBufferCount THEN
        Parser.currentToken = TokenBuffer(TokenBufferPos)
    END IF
END SUB

'-------------------------------------------------------------------------------
' AST CONSTRUCTION
'-------------------------------------------------------------------------------

FUNCTION CreateASTNode% (nodeType AS INTEGER, value AS STRING)
    IF ASTNodeCount >= 10000 THEN
        CreateASTNode% = 0
        EXIT FUNCTION
    END IF
    
    ASTNodeCount = ASTNodeCount + 1
    ASTNodes(ASTNodeCount).nodeType = nodeType
    ASTNodes(ASTNodeCount).value = value
    ASTNodes(ASTNodeCount).lineNum = Parser.currentLine
    ASTNodes(ASTNodeCount).parent = 0
    ASTNodes(ASTNodeCount).firstChild = 0
    ASTNodes(ASTNodeCount).nextSibling = 0
    ASTNodes(ASTNodeCount).symbolRef = 0
    ASTNodes(ASTNodeCount).flags = 0
    
    CreateASTNode% = ASTNodeCount
END FUNCTION

SUB AddChildNode (parent AS LONG, child AS LONG)
    IF parent < 1 OR parent > ASTNodeCount THEN EXIT SUB
    IF child < 1 OR child > ASTNodeCount THEN EXIT SUB
    
    ASTNodes(child).parent = parent
    
    IF ASTNodes(parent).firstChild = 0 THEN
        ASTNodes(parent).firstChild = child
    ELSE
        ' Find last sibling
        DIM sibling AS LONG
        sibling = ASTNodes(parent).firstChild
        DO WHILE ASTNodes(sibling).nextSibling <> 0
            sibling = ASTNodes(sibling).nextSibling
        LOOP
        ASTNodes(sibling).nextSibling = child
    END IF
END SUB

'-------------------------------------------------------------------------------
' PARSING FUNCTIONS
'-------------------------------------------------------------------------------

FUNCTION ParseProgram%
    ' Create root node
    ASTRoot = CreateASTNode%(AST_PROGRAM, "PROGRAM")
    
    ' Parse statements until EOF
    DO WHILE CurrentTokenType% <> TOKEN_EOF
        DIM stmt AS LONG
        stmt = ParseStatement%
        IF stmt > 0 THEN
            AddChildNode ASTRoot, stmt
        END IF
        AdvanceToken
    LOOP
    
    ParseProgram% = ASTRoot
END FUNCTION

FUNCTION ParseStatement%
    DIM tokType AS INTEGER
    tokType = CurrentTokenType%
    
    SELECT CASE tokType
        CASE TOKEN_EOF
            ParseStatement% = 0
        CASE TOKEN_COMMENT
            ParseStatement% = CreateASTNode%(AST_PROGRAM, "COMMENT")
        CASE TOKEN_KEYWORD
            ParseStatement% = ParseKeywordStatement%
        CASE ELSE
            ' Try to parse as expression or assignment
            ParseStatement% = ParseExpression%
    END SELECT
END FUNCTION

FUNCTION ParseKeywordStatement%
    DIM kw AS STRING
    kw = UCASE$(CurrentTokenValue$)
    
    SELECT CASE kw
        CASE "SUB"
            ParseKeywordStatement% = ParseSub%
        CASE "FUNCTION"
            ParseKeywordStatement% = ParseFunction%
        CASE "DIM", "REDIM"
            ParseKeywordStatement% = ParseDeclaration%
        CASE "IF"
            ParseKeywordStatement% = ParseIf%
        CASE "FOR"
            ParseKeywordStatement% = ParseFor%
        CASE "DO"
            ParseKeywordStatement% = ParseDo%
        CASE "WHILE"
            ParseKeywordStatement% = ParseWhile%
        CASE "SELECT"
            ParseKeywordStatement% = ParseSelect%
        CASE ELSE
            ' Default: create generic node
            ParseKeywordStatement% = CreateASTNode%(AST_PROGRAM, kw)
    END SELECT
END FUNCTION

FUNCTION ParseSub%
    ' Parse SUB definition
    ' SUB name [(params)]
    ' ...
    ' END SUB
    DIM subNode AS LONG
    subNode = CreateASTNode%(AST_SUB, "SUB")
    ParseSub% = subNode
END FUNCTION

FUNCTION ParseFunction%
    ' Parse FUNCTION definition
    DIM funcNode AS LONG
    funcNode = CreateASTNode%(AST_FUNCTION, "FUNCTION")
    ParseFunction% = funcNode
END FUNCTION

FUNCTION ParseDeclaration%
    ' Parse DIM/REDIM statement
    DIM declNode AS LONG
    declNode = CreateASTNode%(AST_DECLARATION, "DECLARATION")
    ParseDeclaration% = declNode
END FUNCTION

FUNCTION ParseIf%
    ' Parse IF statement
    DIM ifNode AS LONG
    ifNode = CreateASTNode%(AST_IF, "IF")
    ParseIf% = ifNode
END FUNCTION

FUNCTION ParseFor%
    ' Parse FOR loop
    DIM forNode AS LONG
    forNode = CreateASTNode%(AST_FOR, "FOR")
    ParseFor% = forNode
END FUNCTION

FUNCTION ParseDo%
    ' Parse DO loop
    DIM doNode AS LONG
    doNode = CreateASTNode%(AST_DO, "DO")
    ParseDo% = doNode
END FUNCTION

FUNCTION ParseWhile%
    ' Parse WHILE loop
    DIM whileNode AS LONG
    whileNode = CreateASTNode%(AST_WHILE, "WHILE")
    ParseWhile% = whileNode
END FUNCTION

FUNCTION ParseSelect%
    ' Parse SELECT CASE
    DIM selectNode AS LONG
    selectNode = CreateASTNode%(AST_SELECT, "SELECT")
    ParseSelect% = selectNode
END FUNCTION

FUNCTION ParseExpression%
    ' Parse expression (simplified)
    DIM exprNode AS LONG
    exprNode = CreateASTNode%(AST_EXPRESSION, "EXPRESSION")
    ParseExpression% = exprNode
END FUNCTION

'-------------------------------------------------------------------------------
' HIGH-LEVEL PARSING INTERFACE
'-------------------------------------------------------------------------------

FUNCTION ParseSourceFile% (fileName AS STRING)
    DIM fileHandle AS LONG
    DIM fileContent AS STRING
    
    ' Read source file
    fileHandle = FREEFILE
    OPEN fileName FOR BINARY AS #fileHandle
    fileContent = INPUT$(LOF(fileHandle), fileHandle)
    CLOSE #fileHandle
    
    ' Set source and parse
    SetSource fileContent
    AdvanceToken
    
    ParseSourceFile% = ParseProgram%
END FUNCTION

FUNCTION GetASTRoot%
    GetASTRoot% = ASTRoot
END FUNCTION

FUNCTION GetASTNodeCount%
    GetASTNodeCount% = ASTNodeCount
END FUNCTION

SUB PrintAST (nodeIndex AS LONG, indent AS INTEGER)
    IF nodeIndex < 1 OR nodeIndex > ASTNodeCount THEN EXIT SUB
    
    DIM i AS INTEGER
    FOR i = 1 TO indent
        PRINT "  ";
    NEXT
    
    PRINT ASTNodes(nodeIndex).nodeType; ": "; RTRIM$(ASTNodes(nodeIndex).value)
    
    ' Print children
    DIM child AS LONG
    child = ASTNodes(nodeIndex).firstChild
    DO WHILE child <> 0
        PrintAST child, indent + 1
        child = ASTNodes(child).nextSibling
    LOOP
END SUB
