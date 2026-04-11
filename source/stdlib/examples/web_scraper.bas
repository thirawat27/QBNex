' ============================================================================
' QBNex Standard Library - Web Scraper Example
' ============================================================================
' Demonstrates URL encoding, string parsing, and data extraction
' ============================================================================

'$INCLUDE:'../qbnex_stdlib.bas'

' ============================================================================
' Simulated HTML parsing (simplified for demonstration)
' ============================================================================

SUB ExtractLinks (html AS STRING, links AS QBNex_List)
    DIM POS AS LONG
    DIM endPos AS LONG
    DIM link AS STRING
    
    List_Clear links
    POS = 1
    
    ' Find all href="..." patterns
    DO
        POS = INSTR(POS, html, "href=" + CHR$(34))
        IF POS = 0 THEN EXIT DO
        
        POS = POS + 6 ' Skip 'href="'
        endPos = INSTR(POS, html, CHR$(34))
        
        IF endPos > POS THEN
            link = MID$(html, POS, endPos - POS)
            List_Add links, link
        END IF
        
        POS = endPos + 1
    LOOP
END SUB

SUB ExtractText (html AS STRING, text AS QBNex_StringBuilder)
    DIM i AS LONG
    DIM inTag AS LONG
    DIM c AS STRING
    
    SB_Clear text
    inTag = 0
    
    FOR i = 1 TO LEN(html)
        c = MID$(html, i, 1)
        
        IF c = "<" THEN
            inTag = -1
        ELSEIF c = ">" THEN
            inTag = 0
        ELSEIF NOT inTag THEN
            SB_Append text, c
        END IF
    NEXT i
END SUB

' ============================================================================
' URL Builder
' ============================================================================

FUNCTION BuildURL$ (BASE AS STRING, params AS QBNex_Dict)
    DIM result AS STRING
    DIM sb AS QBNex_StringBuilder
    DIM first AS LONG
    DIM i AS LONG
    
    SB_Init sb
    SB_Append sb, BASE
    SB_Append sb, "?"
    
    ' Note: In real implementation, would iterate dict entries
    ' This is simplified for demonstration
    result = SB_ToString(sb)
    SB_Free sb
    
    BuildURL = result
END FUNCTION

' ============================================================================
' Main Program
' ============================================================================

CLS
PRINT "========================================================================"
PRINT "QBNex Standard Library - Web Scraper Example"
PRINT "========================================================================"
PRINT

' ============================================================================
' 1. URL Encoding Demo
' ============================================================================
PRINT "--- URL Encoding ---"
PRINT

DIM searchQuery AS STRING
DIM encodedQuery AS STRING

searchQuery = "QBNex programming language"
encodedQuery = UrlEncode(searchQuery)

PRINT "Original query: "; searchQuery
PRINT "Encoded query:  "; encodedQuery
PRINT

DIM searchURL AS STRING
searchURL = "https://example.com/search?q=" + encodedQuery
PRINT "Full URL: "; searchURL
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 2. HTML Parsing Demo
' ============================================================================
PRINT "--- HTML Parsing ---"
PRINT

' Simulated HTML content
DIM htmlContent AS STRING
htmlContent = "<html><head><title>Test Page</title></head>"
htmlContent = htmlContent + "<body>"
htmlContent = htmlContent + "<h1>Welcome to QBNex</h1>"
htmlContent = htmlContent + "<p>This is a <a href=" + CHR$(34) + "https://example.com" + CHR$(34) + ">link</a>.</p>"
htmlContent = htmlContent + "<p>Another <a href=" + CHR$(34) + "/page2.html" + CHR$(34) + ">link</a> here.</p>"
htmlContent = htmlContent + "<a href=" + CHR$(34) + "https://github.com/qbnex" + CHR$(34) + ">GitHub</a>"
htmlContent = htmlContent + "</body></html>"

PRINT "HTML Content (truncated):"
PRINT LEFT$(htmlContent, 100); "..."
PRINT

' Extract links
DIM links AS QBNex_List
List_Init links
ExtractLinks htmlContent, links

PRINT "Extracted Links ("; links.Count; "):"
DIM i AS LONG
FOR i = 0 TO links.Count - 1
    PRINT "  ["; i + 1; "] "; List_Get(links, i)
NEXT i
PRINT

' Extract text
DIM pageText AS QBNex_StringBuilder
SB_Init pageText
ExtractText htmlContent, pageText

PRINT "Extracted Text:"
PRINT SB_ToString(pageText)
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 3. Data Extraction and Storage
' ============================================================================
PRINT "--- Data Extraction and Storage ---"
PRINT

' Store extracted data in dictionary
DIM pageData AS QBNex_Dict
Dict_Init pageData

Dict_Set pageData, "url", "https://example.com"
Dict_Set pageData, "title", "Test Page"
Dict_Set pageData, "link_count", LTRIM$(STR$(links.Count))

' Store links in a formatted string
DIM linkList AS STRING
linkList = ""
FOR i = 0 TO links.Count - 1
    IF i > 0 THEN linkList = linkList + "|"
    linkList = linkList + List_Get(links, i)
NEXT i
Dict_Set pageData, "links", linkList

PRINT "Page Data Dictionary:"
PRINT "  URL: "; Dict_Get(pageData, "url")
PRINT "  Title: "; Dict_Get(pageData, "title")
PRINT "  Link Count: "; Dict_Get(pageData, "link_count")
PRINT "  Links: "; Dict_Get(pageData, "links")
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' 4. Export to CSV
' ============================================================================
PRINT "--- Export to CSV ---"
PRINT

DIM csvFile AS STRING
csvFile = "scraped_data.csv"

DIM writer AS QBNex_CsvWriter
CSV_WriterInit writer, csvFile

' Write header
CSV_AddField writer, "URL"
CSV_AddField writer, "Title"
CSV_AddField writer, "Link Count"
CSV_AddField writer, "Timestamp"
CSV_WriteRow writer

' Write data
DIM dt AS QBNex_DateTime
DT_Now dt

CSV_AddField writer, Dict_Get(pageData, "url")
CSV_AddField writer, Dict_Get(pageData, "title")
CSV_AddField writer, Dict_Get(pageData, "link_count")
CSV_AddField writer, DT_Format(dt, "YYYY-MM-DD HH:MI:SS")
CSV_WriteRow writer

PRINT "Data exported to: "; csvFile
PRINT

' ============================================================================
' 5. Generate Report
' ============================================================================
PRINT "--- Generate Report ---"
PRINT

DIM report AS QBNex_StringBuilder
SB_Init report

SB_AppendLine report, "========================================="
SB_AppendLine report, "WEB SCRAPING REPORT"
SB_AppendLine report, "========================================="
SB_AppendLine report, ""
SB_AppendLine report, "Generated: " + DT_Format(dt, "YYYY-MM-DD HH:MI:SS")
SB_AppendLine report, ""
SB_AppendLine report, "SOURCE:"
SB_AppendLine report, "  URL: " + Dict_Get(pageData, "url")
SB_AppendLine report, "  Title: " + Dict_Get(pageData, "title")
SB_AppendLine report, ""
SB_AppendLine report, "STATISTICS:"
SB_AppendLine report, "  Total Links: " + Dict_Get(pageData, "link_count")
SB_AppendLine report, "  HTML Size: " + LTRIM$(STR$(LEN(htmlContent))) + " bytes"
SB_AppendLine report, ""
SB_AppendLine report, "EXTRACTED LINKS:"

FOR i = 0 TO links.Count - 1
    SB_AppendLine report, "  " + LTRIM$(STR$(i + 1)) + ". " + List_Get(links, i)
NEXT i

SB_AppendLine report, ""
SB_AppendLine report, "========================================="

PRINT SB_ToString(report)
PRINT

' ============================================================================
' 6. URL Pattern Matching
' ============================================================================
PRINT "--- URL Pattern Matching ---"
PRINT

PRINT "Checking link patterns:"
FOR i = 0 TO links.Count - 1
    DIM link AS STRING
    link = List_Get(links, i)
    
    PRINT "  "; link
    
    IF GlobMatch(link, "https://*") THEN
        PRINT "    ✓ HTTPS link"
    ELSEIF GlobMatch(link, "http://*") THEN
        PRINT "    ✓ HTTP link"
    ELSEIF GlobMatch(link, "/*") THEN
        PRINT "    ✓ Relative link (absolute path)"
    ELSE
        PRINT "    ✓ Relative link"
    END IF
    
    IF GlobMatch(link, "*.html") THEN
        PRINT "    ✓ HTML file"
    END IF
    
    IF INSTR(link, "github") > 0 THEN
        PRINT "    ✓ GitHub link"
    END IF
    
    PRINT
NEXT i

' ============================================================================
' Cleanup
' ============================================================================
List_Free links
SB_Free pageText
Dict_Free pageData
SB_Free report

PRINT "========================================================================"
PRINT "Web Scraper Example Complete!"
PRINT "========================================================================"
PRINT
PRINT "This example demonstrated:"
PRINT "  • URL encoding for query parameters"
PRINT "  • HTML parsing and link extraction"
PRINT "  • Text extraction from HTML"
PRINT "  • Data storage in Dictionary"
PRINT "  • CSV export"
PRINT "  • Report generation with StringBuilder"
PRINT "  • Pattern matching with Glob"
PRINT
PRINT "Note: This is a simplified demonstration. Real web scraping would"
PRINT "require HTTP client capabilities and more robust HTML parsing."
PRINT
PRINT "Press any key to exit..."
SLEEP
