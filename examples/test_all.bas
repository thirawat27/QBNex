' ==========================================================
' 100% QBASIC / QB64 Test Suite (Ultimate Master Version)
' ครอบคลุมคีย์เวิร์ด คำสั่ง และฟังก์ชันของ QBASIC ครบทุกคำสั่ง!
' ==========================================================

' --- คำสั่งระดับคอมไพเลอร์ (Metacommands) ---
' $STATIC  ' บังคับให้อาร์เรย์มีขนาดคงที่ตอนคอมไพล์ (ถ้าไม่ใส่จะเป็น $DYNAMIC)
' $DYNAMIC ' บังคับให้อาร์เรย์จองหน่วยความจำตอนรันไทม์ (ใช้ REDIM ได้)
' $INCLUDE: 'qb.bi' ' (คอมเมนต์ไว้) ใช้ดึงไฟล์ภายนอกเข้ามา

' --- ส่วนที่ 1: ประกาศโครงสร้าง ขอบเขต และชนิดข้อมูล ---
DECLARE SUB DemoSub ()
DECLARE FUNCTION DemoFunc$ ()
DEFINT I-N   ' ตัวแปร I ถึง N เป็น Integer อัตโนมัติ (16-bit)
DEFSNG A, S  ' ตัวแปร A, S เป็น Single อัตโนมัติ (32-bit Float)
DEFDBL D     ' ตัวแปร D เป็น Double อัตโนมัติ (64-bit Float)
DEFSTR C     ' ตัวแปร C เป็น String อัตโนมัติ
DEFLNG L     ' ตัวแปร L เป็น Long อัตโนมัติ (32-bit Integer)

OPTION BASE 1 ' ให้อาร์เรย์เริ่มต้นที่ Index 1 (ปกติคือ 0)
COMMON SHARED GlobalVal AS INTEGER
DIM SHARED SharedArr(5) AS INTEGER ' อาร์เรย์ที่แชร์ใช้ได้ทุก Sub/Function
CONST MY_PI = 3.141592653589793

TYPE PlayerStats
    Score AS LONG
    Health AS SINGLE
    NAME AS STRING * 20 ' Fixed-length string
END TYPE

' ล้างหน่วยความจำ (CLEAR), ตั้งค่าคีย์บอร์ดลัด (KEY)
CLEAR , , 2048 ' ล้างตัวแปรและตั้งขนาด Stack
KEY 15, "TEST" + CHR$(13)
KEY ON: KEY LIST ' เปิดและแสดงปุ่ม F1-F10 ด้านล่างจอ

CLS
PRINT "=== เริ่มต้น 100% QBASIC / QB64 Test Suite ==="
PRINT "ระบบจัดการ: หน่วยความจำ String ว่าง: "; FRE(""); " Bytes"
PRINT "ระบบจัดการ: หน่วยความจำ Array ว่าง: "; FRE(-1); " Bytes"
PRINT "ระบบจัดการ: หน่วยความจำ Stack ว่าง: "; FRE(-2); " Bytes"
PRINT ">> กดปุ่มใดๆ เพื่อไปต่อ...": SLEEP: CLS

' --- ส่วนที่ 2: คณิตศาสตร์ ตรรกะระดับบิต และการแปลงชนิดข้อมูล ---
PRINT "--- บทที่ 2: Math, Logic & Type Casting ---"
dVar = -45.6789
PRINT "ค่าต้นทาง dVar = "; dVar
PRINT "ABS  (ค่าสัมบูรณ์): "; ABS(dVar)
PRINT "SGN  (เครื่องหมาย): "; SGN(dVar)
PRINT "INT  (ปัดลงเสมอ): "; INT(dVar)
PRINT "FIX  (ตัดทศนิยมทิ้ง): "; FIX(dVar)
PRINT "CINT (ปัดเศษเป็น Int): "; CINT(dVar)
PRINT "CLNG (แปลงเป็น Long): "; CLNG(dVar)
PRINT "CDBL (แปลงเป็น Double): "; CDBL(123.4)
PRINT "CSNG (แปลงเป็น Single): "; CSNG(MY_PI)
PRINT "HEX$ (ฐานสิบหก): "; HEX$(255); " | OCT$ (ฐานแปด): "; OCT$(255)

PRINT "ตรรกะแบบบิต: (15 AND 7) = "; (15 AND 7); " | (1 OR 2) = "; (1 OR 2)
PRINT "ตรรกะแบบบิต: (5 XOR 3) = "; (5 XOR 3); " | NOT 0 = "; (NOT 0)
PRINT "ตรรกะแบบ IMP: "; (-1 IMP 0); " | EQV: "; (-1 EQV -1)

PRINT "ตรีโกณ: SIN="; SIN(1); " COS="; COS(1); " TAN="; TAN(1); " ATN="; ATN(1)
PRINT "อื่นๆ: EXP="; EXP(1); " LOG="; LOG(10); " SQR="; SQR(16)
PRINT "MOD (หารเอาเศษ): "; 10 MOD 3; " | การหารจำนวนเต็ม (\): "; 10 \ 3
RANDOMIZE TIMER
PRINT "RND (สุ่มตัวเลข): "; RND
PRINT ">> กดปุ่มใดๆ เพื่อไปต่อ...": SLEEP: CLS

' --- ส่วนที่ 3: ตัวแปร ข้อความ (Strings) การจัดรูปแบบ และ Input ---
PRINT "--- บทที่ 3: Strings, Formatting & I/O ---"
LET legacyVar = 99 ' การกำหนดค่าแบบโบราณ
DEF FNArea (r) = MY_PI * (r ^ 2) ' ประกาศฟังก์ชันแบบบรรทัดเดียว (Legacy)
PRINT "พื้นที่จาก DEF FN (r=5): "; FNArea(5)

sTxt$ = "  QBasic 100%  "
PRINT "LEN="; LEN(sTxt$); " LTRIM$=["; LTRIM$(sTxt$); "] RTRIM$=["; RTRIM$(sTxt$); "]"
PRINT "UCASE$="; UCASE$(sTxt$); " LCASE$="; LCASE$(sTxt$)
PRINT "LEFT$="; LEFT$(LTRIM$(sTxt$), 2); " RIGHT$="; RIGHT$(RTRIM$(sTxt$), 4); " MID$="; MID$(sTxt$, 5, 5)

' คำสั่ง MID$ แบบ Statement (เขียนทับข้อความ)
tempStr$ = "Hello World"
MID$(tempStr$, 7, 5) = "QBas!"
PRINT "MID$ Statement: "; tempStr$

PRINT "INSTR="; INSTR(sTxt$, "100"); " STR$="; STR$(123); " VAL="; VAL("456")
PRINT "STRING$="; STRING$(3, "@"); " SPACE$=["; SPACE$(2); "]"
PRINT "ASC="; ASC("A"); " CHR$="; CHR$(65)

' การสลับค่า
var1 = 10: var2 = 20
SWAP var1, var2
PRINT "SWAP: var1="; var1; " var2="; var2

' การแสดงผลและจัดรูปแบบ (PRINT USING, TAB, SPC)
PRINT "PRINT USING: ";
PRINT USING "**$#,###.##"; 1234.5;
PRINT TAB(40); "ใช้ TAB(40)"
PRINT "คำว่า"; SPC(5); "ห่าง 5 เคาะด้วย SPC(5)"

' ทดสอบ INKEY$
PRINT "กรุณากด 1 ปุ่ม (ทดสอบ INKEY$)..."
DO
    k$ = INKEY$
LOOP UNTIL k$ <> ""
PRINT "คุณกดปุ่ม: "; k$

' (คอมเมนต์ไว้เพื่อไม่ให้โปรแกรมหยุดรอ)
' INPUT "กรุณากรอกชื่อ: ", Name$
' LINE INPUT "กรุณากรอกประโยค: ", Sentence$
PRINT ">> กดปุ่มใดๆ เพื่อไปต่อ...": SLEEP: CLS

' --- ส่วนที่ 4: โครงสร้างควบคุมขั้นสูง (Advanced Flow Control) ---
PRINT "--- บทที่ 4: Control Flow (Loops, Branches) ---"
' FOR...NEXT และ STEP
FOR idx = 1 TO 10 STEP 2
    IF idx > 5 THEN EXIT FOR ' ทดสอบ EXIT FOR
NEXT idx
PRINT "ออกจาก FOR ที่ idx = "; idx

' WHILE...WEND (ยุคเก่า)
count = 0
WHILE count < 2
    count = count + 1
WEND

' DO...LOOP (แบบเช็คเงื่อนไขก่อนและหลัง)
DO WHILE count < 5
    count = count + 1
    IF count = 4 THEN EXIT DO ' ทดสอบ EXIT DO
LOOP
DO
    count = count - 1
LOOP UNTIL count = 0

' IF...THEN...ELSEIF...ELSE
IF count = 1 THEN
    PRINT "Count is 1"
ELSEIF count = 0 THEN
    PRINT "IF...ELSEIF ทำงานถูกต้อง: Count = 0"
ELSE
    PRINT "Unknown"
END IF

' SELECT CASE
selVar = 2
SELECT CASE selVar
CASE 1
    PRINT "Case 1"
CASE 2, 3
    PRINT "SELECT CASE ทำงานถูกต้อง (Case 2,3)"
CASE 4 TO 10
    PRINT "Case 4 to 10"
CASE IS > 10
    PRINT "Case > 10"
CASE ELSE
    PRINT "Case Else"
END SELECT

' Computed GOTO / GOSUB (ON ... GOTO / ON ... GOSUB)
choice = 2
ON choice GOTO Lbl1, Lbl2, Lbl3
Lbl1: PRINT "ไม่ถูกเรียก": GOTO SkipGotos
Lbl2: PRINT "ON...GOTO ทำงานถูกต้อง!": GOTO SkipGotos
Lbl3: PRINT "ไม่ถูกเรียก"
SkipGotos:

ON choice GOSUB Sub1, Sub2, Sub3
GOTO SkipGoSub
Sub1: RETURN
Sub2: PRINT "ON...GOSUB ทำงานถูกต้อง!": RETURN
Sub3: RETURN
SkipGoSub:

PRINT ">> กดปุ่มใดๆ เพื่อไปต่อ...": SLEEP: CLS

' --- ส่วนที่ 5: การจัดการไฟล์ และ File System (100% I/O) ---
PRINT "--- บทที่ 5: File System & File I/O ---"
MKDIR "TESTDIR" ' สร้างโฟลเดอร์
CHDIR "TESTDIR" ' เข้าไปในโฟลเดอร์
PRINT "ไฟล์ในโฟลเดอร์ (FILES): ": FILES

fNum = FREEFILE ' หาช่องไฟล์ที่ว่าง
' 1. แบบ OUTPUT (เขียนใหม่ทับของเดิม)
OPEN "temp1.txt" FOR OUTPUT AS #fNum
PRINT #fNum, "Hello", 123
WRITE #fNum, "World", 456
CLOSE #fNum

' 2. แบบ APPEND (เขียนต่อท้าย)
OPEN "temp1.txt" FOR APPEND AS #1
PRINT #1, "Append Data"
CLOSE #1

NAME "temp1.txt" AS "temp2.txt" ' เปลี่ยนชื่อไฟล์

' 3. แบบ INPUT (อ่าน)
OPEN "temp2.txt" FOR INPUT AS #fNum
PRINT "ขนาดไฟล์ (LOF): "; LOF(fNum); " Bytes"
WHILE NOT EOF(fNum)
    LINE INPUT #fNum, lData$
    PRINT "อ่าน (LOC="; LOC(fNum); "): "; lData$
WEND
CLOSE #fNum

' 4. แบบ BINARY (อ่าน/เขียนระดับไบต์)
OPEN "temp2.txt" FOR BINARY AS #1
SEEK #1, 1 ' เลื่อน Cursor ไปที่ไบต์แรก
bData$ = INPUT$(5, #1) ' อ่านทีละ n ไบต์
PRINT "Binary Read (5 bytes): "; bData$
PUT #1, LOF(1) + 1, "END" ' เขียนต่อท้ายในโหมด Binary
CLOSE #1

KILL "temp2.txt" ' ลบไฟล์

' 5. แบบ RANDOM (ฐานข้อมูลแบบ Record) และ Data Conversion
OPEN "rand.dat" FOR RANDOM AS #1 LEN = 14
FIELD #1, 4 AS F1$, 6 AS F2$, 4 AS F3$ ' กำหนดฟิลด์ (ยุคเก่า)
LSET F1$ = MKS$(12.34) ' แปลง Single เป็น String ขนาด 4 Byte
RSET F2$ = "TEST  "
LSET F3$ = MKI$(99)    ' แปลง Integer เป็น String ขนาด 2 Byte
PUT #1, 1 ' เขียน Record ที่ 1
GET #1, 1 ' อ่าน Record ที่ 1
PRINT "อ่านค่า Random: MKS(CVS)="; CVS(F1$); " | ข้อความ="; F2$; " | MKI(CVI)="; CVI(F3$)
CLOSE #1
KILL "rand.dat"

CHDIR ".." ' ถอยโฟลเดอร์
RMDIR "TESTDIR" ' ลบโฟลเดอร์
PRINT "ทดสอบ File System เสร็จสิ้น (ลบไฟล์ขยะแล้ว)"
PRINT ">> กดปุ่มใดๆ เพื่อไปต่อ...": SLEEP: CLS

' --- ส่วนที่ 6: อาร์เรย์ (Arrays) ขั้นสูง และ DATA ---
PRINT "--- บทที่ 6: Arrays (REDIM, PRESERVE, ERASE) & DATA ---"
DIM staticArr(1 TO 5) AS INTEGER
REDIM dynamicArr(5) AS INTEGER ' เปลี่ยนขนาดได้
dynamicArr(1) = 999
REDIM PRESERVE dynamicArr(10) AS INTEGER ' ขยายขนาดแต่เก็บค่าเดิมไว้
PRINT "LBOUND="; LBOUND(dynamicArr); " UBOUND="; UBOUND(dynamicArr)
PRINT "ค่าในอาร์เรย์เดิมหลัง REDIM PRESERVE: "; dynamicArr(1)
ERASE dynamicArr, staticArr ' ลบอาร์เรย์คืนหน่วยความจำ

RESTORE DataBlock2 ' ข้ามไปอ่าน DATA บล็อกที่ 2
READ a1, a2$: PRINT "READ DATA Block 2: "; a1, a2$
RESTORE DataBlock1 ' กลับมาอ่านบล็อกแรก
READ num1, num2: PRINT "READ DATA Block 1: "; num1, num2

DataBlock1:
DATA 10, 20, 30
DataBlock2:
DATA 1024, "QBASIC"

PRINT ">> กดปุ่มใดๆ เพื่อเข้าสู่ระบบ Hardware & Event..."
SLEEP: CLS

' --- ส่วนที่ 7: ระบบหน่วยความจำ ฮาร์ดแวร์ และ Event Trapping ---
PRINT "--- บทที่ 7: Memory, Hardware & Events ---"
' คำเตือน: PEEK/POKE เป็นคำสั่งเจาะจงกับสถาปัตยกรรม DOS
DEF SEG = 0 ' ชี้ Segment หน่วยความจำไปที่ 0
kbFlag = PEEK(&H417) ' อ่านสถานะไฟคีย์บอร์ดใน DOS
' POKE &H417, kbFlag ' (คอมเมนต์ไว้เพื่อความปลอดภัย)
DEF SEG ' คืนค่า Segment เริ่มต้น
PRINT "Keyboard Status Byte (Memory 0000:0417): "; kbFlag
PRINT "Memory Address ของ sTxt$: VARPTR="; VARPTR(sTxt$); " VARSEG="; VARSEG(sTxt$)
PRINT "Memory Address แบบข้อความ: SADD="; SADD(sTxt$); " VARPTR$="; VARPTR$(sTxt$)

' ทดสอบ Event Trapping (Timer)
ON TIMER(1) GOSUB TimerTick
TIMER ON ' เปิดการจับเวลา
PRINT "รอ 2 วินาทีเพื่อให้ Timer ทำงาน..."
SLEEP 2
TIMER OFF ' ปิดการจับเวลา
TIMER STOP ' หยุดชั่วคราว (ใช้ TIMER ON เพื่อเริ่มใหม่)

' สาธิตคำสั่งฮาร์ดแวร์และการดักจับเหตุการณ์ (ไม่ประมวลผลจริงเพื่อหลีกเลี่ยงการ Crash)
' OUT &H3F8, 65 : v = INP(&H3F8) ' อ่าน/เขียน I/O Port
' WAIT &H3DA, 8 ' รอสถานะ VSYNC (มักใช้ในเกม)
' ON COM(1) GOSUB ComEvent: COM(1) ON ' พอร์ตซีเรียล
' ON PEN GOSUB PenEvent: PEN ON ' ปากกาแสง
' ON STRIG(1) GOSUB JoyEvent: STRIG(1) ON ' จอยสติ๊กปุ่มกด
' ON PLAY(1) GOSUB MusicEvent: PLAY ON ' ตรวจจับเพลงจบ
' jX = STICK(0): jY = STICK(1) ' อ่านแกนจอยสติ๊ก
PRINT ">> กดปุ่มเพื่อเข้าสู่กราฟิกขั้นสูง...": SLEEP: CLS

' --- ส่วนที่ 8: กราฟิกหน้าจอขั้นสูง และเสียง ---
SCREEN 12 ' เข้าสู่โหมด VGA กราฟิก 16 สี 640x480
COLOR 10, 1 ' สีข้อความเขียวอ่อน พื้นน้ำเงิน
CLS

' VIEW PRINT ใช้จำกัดพื้นที่แสดงข้อความ
VIEW PRINT 1 TO 5
PRINT "--- บทที่ 8: Advanced Graphics & Sound ---"
PRINT "พื้นที่นี้ถูกกำหนดด้วย VIEW PRINT"
VIEW PRINT ' คืนค่าแสดงผลทั้งจอ

' ตั้งตารางสี (PALETTE) 
PALETTE 10, 63 ' เปลี่ยนสี 10 (เขียวอ่อน) ให้กลายเป็นสีแดงสดในหน้าจอ
' หมายเหตุ: มี PALETTE USING สำหรับโหลดอาเรย์สีทั้งชุดด้วย

' วาดกราฟิกพื้นฐาน
LINE (10, 50)-(300, 200), 15, B ' วาดกรอบสี่เหลี่ยมสีขาว (B = Box)
LINE (10, 50)-(300, 200), 12 ' ลากเส้นทแยงมุม
LINE (10, 200)-(300, 50), 12, BF ' BF = Box Filled (สี่เหลี่ยมทึบ)
CIRCLE (150, 125), 50, 14 ' วงกลมเหลือง
PAINT (150, 125), 9, 14 ' ระบายสีฟ้า (9) ในกรอบวงกลมเหลือง (14)

' PSET และ PRESET
PSET (150, 125), 15 ' จุดสีขาวตรงกลาง
PRESET (155, 125) ' ลบพิกัด (เทียบเท่า PSET สี 0)
LOCATE 6, 1: PRINT "POINT(150,125) Color = "; POINT(150, 125)

' DRAW (กราฟิกด้วยภาษามาโครแบบเต่า Logo)
PSET (350, 100), 13
DRAW "C13 U50 R50 D50 L50 F25 E25 H25 G25" ' วาดสี่เหลี่ยมมีกากบาทสีชมพู

' ระบบพิกัดสมมติ (WINDOW / VIEW / PMAP)
VIEW (400, 100)-(600, 300), 0, 15 ' กำหนด Viewport ย่อย
WINDOW (-10, -10)-(10, 10) ' สร้างพิกัดสมมติใน Viewport
CIRCLE (0, 0), 8, 11
LOCATE 7, 1: PRINT "PMAP Logical to Physical Y: "; PMAP(0, 3)
VIEW ' คืนค่า Viewport

' GET และ PUT ภาพกราฟิก
DIM gfxBuf(1 TO 200) AS INTEGER
GET (150, 125)-(170, 145), gfxBuf ' ก๊อปปี้ภาพหน้าจอบางส่วน
PUT (50, 250), gfxBuf, PSET ' แปะภาพทับแบบเป๊ะๆ
PUT (80, 250), gfxBuf, XOR ' แปะภาพแบบสลับสี (ใช้ทำ Sprite เคลื่อนที่ลบตัวเองได้)
PUT (110, 250), gfxBuf, OR  ' แปะแบบรวมสี
PUT (140, 250), gfxBuf, AND ' แปะแบบตัดสี

' BLOAD / BSAVE (บันทึกภาพหน้าจอลงไฟล์ไบนารี / คอมเมนต์ไว้เพื่อความปลอดภัย)
' DEF SEG = &HA000 ' ชี้ไปยังหน่วยความจำการ์ดจอ VGA
' BSAVE "screen.bsv", 0, 65535 ' เซฟภาพทั้งจอ
' BLOAD "screen.bsv", 0 ' โหลดภาพกลับ
' DEF SEG

BEEP ' ส่งเสียง Beep 1 ครั้ง
LOCATE 25, 1: PRINT ">> เล่นเสียงดนตรี... (PLAY)";
SOUND 440, 5 ' โน้ต A (440Hz) นาน 5 Ticks
PLAY "MFT180 O3 L8 C E G > C < G E C" ' MF = Music Foreground
SLEEP 2

' ออกจากกราฟิก
SCREEN 0: WIDTH 80, 25: COLOR 7, 0: CLS

' --- ส่วนที่ 9: วันที่ ระบบ การจัดการ Error และการปิดโปรแกรม ---
PRINT "--- บทที่ 9: System, Debugging & Errors ---"
PRINT "DATE$: "; DATE$; " | TIME$: "; TIME$
PRINT "TIMER (วินาทีตั้งแต่เที่ยงคืน): "; TIMER
PRINT "COMMAND$ (พารามิเตอร์รันโปรแกรม): "; COMMAND$
PRINT "ENVIRON$ (PATH): "; LEFT$(ENVIRON$("PATH"), 50); "..."
PRINT "CSRLIN (บรรทัดปัจจุบัน): "; CSRLIN; " POS (คอลัมน์): "; POS(0)

' คำสั่งสำหรับ Printer (คอมเมนต์ไว้เพื่อไม่ให้ล็อกระบบ)
' LPRINT "ส่งข้อความนี้ไปที่เครื่องพิมพ์พอร์ต LPT1"
' PRINT "เช็คสถานะ Printer (LPOS): "; LPOS(1)

' คำสั่ง Debugging (มองไม่เห็นผลบนจอปกติ แต่ใช้ตอนรันทีละบรรทัด)
TRON  ' Trace ON เปิดโหมดแสดงหมายเลขบรรทัดที่กำลังทำงาน
TROFF ' Trace OFF ปิดโหมด

' การดักจับ Error ขั้นสูง
ON ERROR GOTO ErrTrap
ERROR 255 ' สั่งให้เกิด Error สมมติเบอร์ 255
ErrResumePoint:
' ลองทำ Error อีกแบบ
ERROR 11 ' Division by zero
ON ERROR GOTO 0 ' ปิดการดักจับ Error

CALL DemoSub
PRINT "เรียกฟังก์ชัน: "; DemoFunc$()

' --- ส่วนที่ 10: เฉพาะสำหรับโปรแกรม QB64 (QB64 Extensions) ---
' (โค้ดส่วนนี้คอมเมนต์ไว้ทั้งหมด เพื่อรักษาความเข้ากันได้ 100% กับ QBASIC ดั้งเดิม)
' หากนำไปรันใน QB64 สามารถเอาคอมเมนต์ออกได้เลย
PRINT "--- บทที่ 10: QB64 Specific Keywords (Commented) ---"
' _TITLE "My Ultimate App" ' เปลี่ยนชื่อ Title Bar ของหน้าต่าง
' _DELAY 0.5 ' แทนที่ SLEEP แต่ละเอียดระดับทศนิยมวินาที
' _DISPLAY ' อัปเดตหน้าจอ (ใช้คู่กับ _AUTODISPLAY และ _LIMIT สำหรับเกม)
' _SNDPLAY _SNDOPEN("music.mp3") ' เล่นไฟล์เสียง MP3/WAV
' img& = _LOADIMAGE("pic.png") : _PUTIMAGE (0,0), img& ' โหลดรูปภาพยุคใหม่
' mx = _MOUSEX: my = _MOUSEY: mb = _MOUSEBUTTON(1) ' อ่านเมาส์ง่ายขึ้น

PRINT
PRINT "=== เสร็จสิ้นการทดสอบ 100% === "
' ตัวเลือกการจบโปรแกรม:
' STOP ' หยุดชั่วคราว (เข้าสู่โหมด Debug)
' SHELL "DIR /W" ' เรียกคำสั่ง DOS หรือ Command Prompt
' SYSTEM ' ปิดโปรแกรมและกลับสู่ OS ดั้งเดิม (ปิดหน้าต่าง)
' CHAIN "OTHER.BAS" ' โหลดและรันไฟล์ BAS อื่นต่อทันที
END ' จบโปรแกรมและค้างหน้าต่าง IDE ไว้


' ===============================================
' ------------- L A B E L S / E V E N T S -------
' ===============================================
TimerTick:
PRINT "  [*** Timer Event เกิดขึ้น! ***]"
RETURN

ErrTrap:
IF ERR = 255 THEN
    PRINT "  [ดักจับ Error เบอร์ "; ERR; " ที่บรรทัด ERL="; ERL; " (Error สมมติ)]"
    RESUME ErrResumePoint ' กลับไปจุดที่ระบุ
ELSE
    PRINT "  [ดักจับ Error เบอร์ "; ERR; ": "; ERDEV$; " โค้ดอุปกรณ์: "; ERDEV; "]"
    RESUME NEXT ' ข้ามบรรทัดที่ Error แล้วทำบรรทัดถัดไป
END IF

' ===============================================
' ----------- S U B P R O G R A M S -------------
' ===============================================
SUB DemoSub STATIC ' STATIC ทำให้ตัวแปรจำค่าไว้ในรอบถัดไปได้ (ไม่รีเซ็ต)
    SHARED GlobalVal ' ดึงตัวแปรจากโครงสร้างหลักมาใช้
    GlobalVal = GlobalVal + 1
    PRINT "ภายใน SUB: เรียกครั้งที่ "; GlobalVal
END SUB

FUNCTION DemoFunc$ ()
    ' ฟังก์ชันสำหรับคืนค่า String
    DemoFunc$ = "Hello from FUNCTION"
END FUNCTION