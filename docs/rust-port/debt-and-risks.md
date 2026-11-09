# Debt Register (ระบบเดิม) + Risk Register (การ port)

> Baseline ที่ใช้ตัดสิน: (1) convention ของ C++/ISO (ODR, data race = UB) และ CMake, (2) pattern เด่นของโปรเจกต์เอง (self-host bootstrap, smoke tests, CHANGELOG/SemVer)
> ทุก correction เป็น **Proposed** ต้องอนุมัติก่อน

## A. Debt Register — ระบบเดิม

| # | Finding | Dimension | Evidence | Impact | Severity | Confidence |
| --- | --- | --- | --- | --- | --- | --- |
| F0 | **ยืนยันแล้ว (2026-11-09):** compiler รุ่นที่ 2 (HEAD compile ตัวเอง) compile hello world ไม่ได้ — self-host ทำงานได้เพราะ snapshot เก่าเท่านั้น (L-00 ใน progress.md) | Flow conflict | `.oracle/qb.exe compiler/qbnex.bas -o qb-gen2.exe` แล้ว `qb-gen2.exe hello.bas` → `[E9999] Too many errors` | regenerate snapshot ไม่ได้; ถ้า snapshot หาย build ไม่ได้เลย | Critical | Direct |
| F1 | Bootstrap snapshot `internal/source` ล้าหลัง source และไม่มีกลไก refresh | Debt / Flow conflict | snapshot ไม่มี `TOPLEVELRUNTIME_INJECTMAINHOOK` (classes.bas:44 มี); `evnt()` อ้างแค่ 11 ไฟล์ include; ไม่มีสคริปต์/CI step แตะ `internal/source`; แก้ล่าสุด commit d811e1a | stage0 ≠ stage1 → ถ้า source ต้องการ feature ที่ stage0 ไม่มี self-host จะพัง; ใช้เป็น oracle ไม่ได้ | High | Direct (drift) / Inferred (ผลกระทบ) |
| F2 | Extern ชนิดไม่ตรงข้าม translation unit | Framework drift (C++ ODR) | qbx.cpp:807 `extern ptrszint *qbs_tmp_list;` vs libqb.cpp:6839 `std::vector<ptrszint> qbs_tmp_list`; libqb.cpp:1384 `extern ontimer_struct *ontimer;` vs qbx.cpp:1821 `std::vector<ontimer_struct> ontimer` | UB; ทำงานได้เพราะ libstdc++ วาง begin-pointer เป็น field แรกและ Itanium ABI ไม่ mangle ชนิดของตัวแปร → พังทันทีถ้าเปลี่ยน STL/ABI/เปิด debug iterators | High | Direct |
| F3 | Flag ข้าม thread ไม่ใช้ atomic/lock | Framework drift (C++ memory model) | `qbevent`, `new_error`, `display_frame[].state`, `keyhit[]`, mouse queue, audio handles อ่าน/เขียนจากหลาย thread (libqb.cpp:450-464, 794-796, 1574-1578; qbx.cpp:1925-1959); ไม่มี mutex ระหว่าง `_SND*` กับ `snd_mainloop` | data race (UB); พึ่ง memory ordering ของ x86 → เสี่ยงบน ARM (Apple Silicon/Linux ARM) | High | Inferred · Verify first: Yes |
| F4 | Compiler ไม่มี AST: token เป็นสตริง, emit C++ ระหว่าง parse, CLASS lowering บน text | Separation of concerns | parsing.bas:943-1952; ~960 `PRINT #n` ใน qbnex.bas; classes.bas:626-1245 | แก้ semantic จุดเดียวต้องแตะ parse+emit พร้อมกัน; error position ของโค้ดที่ถูก rewrite คลาดเคลื่อน | High | Direct |
| F5 | Global state ~380 ชื่อ + driver ~1150 GOTO + recompile-until-fixpoint | Debt | ~333 บรรทัด `DIM SHARED`; qbnex.bas:932-976, 10937-10944 | reason ยาก, compile time ทวีคูณเมื่อ recompile หลายรอบ | Medium | Direct |
| F6 | Tests ไม่อยู่ใน CI และไม่มี golden output | Debt | ไม่มี `tests/` ใน `.github/workflows/*`; tests ใช้ grep เท่านั้น | regression หลุดง่าย; **ไม่มี oracle สำหรับ port** | High (สำหรับ port) | Direct |
| F7 | Runtime ผูกกับ codegen ผ่านชื่อ symbol และ layout C โดยตรง | Separation of concerns | generated code ใช้ `qbs->`, descriptor `[0] [2] [4i+1]`, `cmem`, `return_point[]`, `passN` (qbx.cpp:655-1113) | เปลี่ยน runtime ไม่ได้โดยไม่เปลี่ยน compiler; ขัดขวาง strangler migration | High | Direct |
| F8 | Error handling ด้วยการเรียก `QBMAIN(NULL)` ซ้อน; `end()` วนไม่รู้จบ | Flow conflict | libqb.cpp:6480, 6538-6543 | stack โตตาม error ที่ handle ซ้ำ (Inferred); ยากต่อการ port ตรงตัว | Medium | Direct (กลไก) / Inferred (stack growth) |
| F9 | Vendored third-party ซ้ำ (โฟลเดอร์ `download/`) ~16 MB ไม่ได้ compile | Debt | core/download/freeglut-2.8.0, core/glew/download, ttf/download, game_controller/download; stb_vorbis 2 ชุด | repo บวม, สับสนว่าตัวไหนใช้จริง | Low | Direct |
| F10 | License ไม่ครบ: GLEW, glues, mp3_mini ไม่มีใน `licenses/COPYING.TXT`; FreeGLUT ระบุ LGPL แต่ไฟล์ไม่ใช่ข้อความ LGPL | Debt (compliance) | licenses/COPYING.TXT | ความเสี่ยงด้าน redistribution | Medium | Direct · Verify first: Yes |
| F11 | Version drift: CHANGELOG 1.0.4 แต่ `Version$ = "1.0.3"`; installer 1.0.3; SECURITY.md รองรับแค่ 1.0.0 | Project drift | CHANGELOG.md:8; version.bas:5; qbnex_setup.iss:12; SECURITY.md:5-8 | binary รายงาน version ผิด | Low | Direct |
| F12 | `.eniac-plan.md` อ้าง `runtime/` และ `docs/architecture-audit-2026-07-24.md` ที่ไม่มีอยู่ | Project drift | `.eniac-plan.md:11,32`; ไม่มี `runtime/`, ไม่มีใน git history | agent/คนในอนาคตเข้าใจผิด | Low | Direct |
| F13 | TopLevelRuntime ย้ายโค้ดระดับบนไปเป็น SUB อย่างไม่มีเงื่อนไข | Flow conflict | classes.bas:1-124; prepass.bas:86-129; **ยืนยันแล้ว:** `ON ERROR GOTO` + handler ใน main module compile ไม่ผ่าน (L-01) | โปรแกรม QB ที่ใช้ error handler แบบคลาสสิก compile ไม่ได้ | High | Direct |
| F16 | Runtime console: `PRINT` แบบ `,` วน space ไม่รู้จบเมื่อ stdout ถูก redirect (L-02); `RESTORE label` ค้าง (L-03); relational ใน PRINT ถูก reject (L-04) | Flow conflict | `tests/conformance/cases/*/*.xfail` | โปรแกรมทั่วไปค้าง/compile ไม่ได้ | High | Direct |
| F14 | vwatch debugger ฝังใน compiler แต่ไม่มีฝั่ง IDE | Debt | qbnex.bas:1444-1501; vwatch.bm:684-695 | โค้ดตายที่ต้องดูแล | Low | Inferred |
| F15 | Docker runtime stage ไม่มี g++ | Flow conflict | docker/Dockerfile:38-47 | `qb file.bas` ใน image อาจใช้ไม่ได้ | Low | Inferred · Verify first: Yes |

### Proposed Corrections (ระบบเดิม — ทำได้ก่อน/ระหว่าง port)

| # | Smallest safe correction | ป้องกันอะไรในอนาคต |
| --- | --- | --- |
| F1 | เพิ่ม `xtask`/สคริปต์ regenerate snapshot (`qb -z compiler/qbnex.bas` → copy `internal/temp` → `internal/source`) + CI job ที่ self-host 2 รอบแล้วเทียบ | stage drift |
| F2 | แก้ `extern` ให้ตรงชนิด (`extern std::vector<...>`) — 2 บรรทัด | UB เมื่อเปลี่ยน toolchain |
| F6 | เพิ่ม job `tests/*.sh` ใน `pull_request.yml` | regression ระหว่าง port |
| F11 | ตั้ง `Version$` จากแหล่งเดียว | version drift |
| F12 | ลบหรือ mark `.eniac-plan.md` ว่า superseded | context ผิดสำหรับ agent |
| F3, F4, F5, F7, F8 | **ไม่แก้ในระบบเดิม** — แก้โดยการออกแบบใหม่ใน Rust (port-plan §2) | — |

## B. Risk Register — การ port

| # | Risk | โอกาส | ผลกระทบ | Mitigation (Proposed) |
| --- | --- | --- | --- | --- |
| R1 | **ไม่มีสเปกภาษา** — พฤติกรรมจริงกระจายอยู่ในโค้ด 80k บรรทัด | สูง | สูง | Phase 0 corpus + semantics-catalog; differential testing กับ binary เดิม |
| R2 | `_FLOAT` = 80-bit `long double`; Rust ไม่มี f80 | แน่นอน | กลาง | D3: ใช้ f64 (ประกาศ deviation) หรือ soft-float f80 |
| R3 | Formatting ตัวเลข (`STR$`, `PRINT`, `PRINT USING`) ต่างแม้แต่ 1 digit = output ต่าง | สูง | สูง | port อัลกอริทึมเดิมตรงตัว + property tests จาก oracle |
| R4 | RND/TIMER/ปัดเศษ ต่างจากเดิม → เกม/โปรแกรมเก่าทำงานต่าง | กลาง | กลาง | port LCG และ rounding ตรงตัว; test sequence |
| R5 | Graphics rasterization (LINE/CIRCLE/PAINT/DRAW) ต่าง pixel | สูง | กลาง | port อัลกอริทึมจาก libqb.cpp ไม่ใช่ใช้ library 2D ทั่วไป; เทียบ framebuffer hash |
| R6 | `DECLARE LIBRARY` ที่ include C header ใช้ไม่ได้ถ้าไม่มี C compiler | แน่นอน | กลาง–สูง (ผู้ใช้ที่พึ่ง C lib) | D5: รองรับเฉพาะ DYNAMIC LIBRARY ผ่าน `libloading` + ABI ที่ประกาศชนิดชัด |
| R7 | `SUB _GL` / OpenGL extension มี API ใหญ่ | กลาง | กลาง | D6: ตัดใน v2.0 หรือทำผ่าน `glow` ภายหลัง |
| R8 | DOS memory emulation (`PEEK/POKE/DEF SEG/VARSEG`) — โปรแกรมเก่าพึ่ง layout | กลาง | กลาง | D4: emulate 1 MB + ตำแหน่งที่รู้จัก (BIOS tick, keyboard flags) |
| R9 | Scope creep: เขียนใหม่ "ทั้งหมด" ใช้ 18–27 PM | สูง | สูง | phase gate ชัด, spike ก่อน, ตัด D6/D8/D9 ออกจาก v2.0 |
| R10 | Performance VM ต่ำกว่า native C++ เดิม | สูง (ช่วง VM) | กลาง | Phase 5 Cranelift; benchmark gate |
| R11 | Font rasterization (fontdue vs FreeType) ต่าง 1px | สูง | ต่ำ | tolerance ใน test; หรือ FFI FreeType ชั่วคราว |
| R12 | macOS: หน้าต่างต้องอยู่ main thread; โครงสร้าง thread เดิมต่างกันตาม OS | กลาง | กลาง | ออกแบบ runtime ให้ main thread = event loop ทุก OS ตั้งแต่ต้น |
| R13 | Oracle (qb.exe เดิม) เองมี bug (F1, F13, audio smoke) | กลาง | กลาง | ทุก diff ต้อง triage: bug เดิม vs bug ใหม่; บันทึก intentional deviation |
| R14 | Self-host หายไป: compiler ไม่ได้เขียนด้วย BASIC อีก | แน่นอน | ต่ำ–กลาง (เชิงอัตลักษณ์โปรเจกต์) | D10 — ตัดสินใจโดยเจ้าของโปรเจกต์ |
