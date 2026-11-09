# Open Questions, Decisions, AI Agent Notes

> **Decided 2026-11-09 (เจ้าของโปรเจกต์):** D10 = เก็บ compiler เป็น BASIC → D2 = runtime-first strangler ผ่าน C ABI, D1 ไม่ใช้ (backend ยังเป็น C++/g++). ดู Revision 2 ใน [`port-plan.md`](port-plan.md). D3–D9, D11, D12 ยังเปิด — ภายใต้ D10 ส่วนใหญ่ไม่ต้องตัดสินจนถึง phase ที่เกี่ยวข้อง

## 1. Decisions ที่ต้องอนุมัติก่อนเริ่ม Phase 1

| # | คำถาม | ตัวเลือก | ข้อแนะนำ | ผลถ้าเลือกต่าง |
| --- | --- | --- | --- | --- |
| **D1** | Backend ของ compiler ใหม่ | A Cranelift AOT · B Bytecode VM + stub exe · C emit C + g++ · D emit Rust | **B ก่อน แล้ว A** | C = ไม่ใช่ Rust ทั้งหมด, ยัง bundle GCC; D = ไม่แนะนำ |
| **D2** | กลยุทธ์ migration | Parallel rewrite + oracle · Runtime-first strangler ผ่าน C ABI | **Parallel rewrite** | strangler = ส่งมอบระหว่างทางได้ แต่ต้องเลียน layout C ที่จะถูกทิ้ง |
| **D3** | `_FLOAT` (80-bit) | f64 (deviation) · soft-float f80 | **f64 + ประกาศ deviation** (Assumed: ผู้ใช้น้อย) | f80 = ช้าและงานเพิ่ม แต่เข้ากันได้ 100% |
| **D4** | ระดับ DOS memory emulation | เต็ม 1 MB `cmem` + VARSEG · เฉพาะ `_MEM`/VARPTR · ไม่รองรับ | **เต็ม 1 MB** (ราคาไม่แพงใน runtime) | ตัดออก = โปรแกรม QB4.5 เก่าที่ PEEK/POKE พัง |
| **D5** | `DECLARE LIBRARY` | DYNAMIC เท่านั้น (`libloading`) · คงแบบ static + ให้ผู้ใช้มี C compiler · ตัด | **DYNAMIC เท่านั้นใน v2.0** | static ต้องคง toolchain C |
| **D6** | `SUB _GL` + OpenGL extension | ตัดใน v2.0 · ทำผ่าน `glow` | **ตัดใน v2.0** | ทำ = +1–2 PM |
| D7 | `OPEN "COMn:"` serial port | รองรับผ่าน `serialport` crate · ตัด | ตัดใน v2.0 | |
| D8 | Printer (`LPRINT`, `_PRINTIMAGE`) — เดิม Windows only | คง Windows · ตัด | คง Windows (เล็ก) | |
| D9 | vwatch / `$DEBUG` | ตัด · ทำ debugger ใหม่ (DAP) | **ตัด** (ไม่มีฝั่ง IDE อยู่แล้ว) | |
| D10 | ยอมเสีย self-hosting (compiler เขียนด้วย BASIC) หรือไม่ | ยอม · คง compiler เป็น BASIC แต่ port runtime อย่างเดียว | ขึ้นกับเป้าหมายของเจ้าของ | ถ้าคง BASIC = งานแค่ runtime (~10–14 PM, Assumed) แต่ต้องทำ C ABI (D2 = strangler) |
| D11 | Version/branding | QBNex 2.0 (breaking) · 1.x ต่อ | 2.0 | deviation (D3/D5/D6) เป็น breaking change |
| D12 | Repo layout | workspace ใหม่ใน repo เดิม (`rust/`) · repo ใหม่ | ใน repo เดิม เพื่อให้ oracle + corpus อยู่ที่เดียว | |

## 2. Open Questions (มีผลต่อสถาปัตยกรรม)

1. **กลุ่มผู้ใช้หลักคือใคร?** — ผู้ใช้โค้ด QB4.5 เก่า (ต้อง compat สูง: D3/D4) หรือผู้ใช้ใหม่ที่ใช้ส่วนขยาย QBNex (CLASS, IMPORT) — เปลี่ยนลำดับความสำคัญของ Phase 3/4
2. **ต้องรองรับโค้ด QB64 ทั่วไปแค่ไหน?** QBNex อ้าง "100% backward compatible" กับ QB4.5 (README.md:153) — ต้องการเท่าเดิมหรือยอมให้มี deviation list
3. **Performance target ของ VM** — ยอมช้ากว่า native เดิมได้กี่เท่าในช่วงก่อน Phase 5?
4. **Platform ขั้นต่ำ** — Windows 7 ยังต้องรองรับไหม (Rust stable รองรับ Win10+ เป็นหลัก; Win7 ต้องใช้ target tier 3)
5. **ทีม/งบเวลา** — 18–27 PM เป็นประมาณการสำหรับคนที่รู้ Rust+compiler; ทำคนเดียวหรือมีทีม? มีผลต่อการตัด scope (R9)
6. **ข้อ Verify first ใน semantics-catalog (C7, C16, L9, S5, N3) และ debt F3, F13, F15** — ต้องรันกับ binary เดิมก่อนเขียนสเปก Rust

## 3. Confirmed Facts / Inferences สำคัญ (สรุป)

**Direct**
- compiler = BASIC 39.6k บรรทัด self-host, emit C++ fragment ~20 ชนิดเข้า `internal/temp` แล้ว `qbx.cpp` `#include`
- runtime = libqb.cpp unity build 31k บรรทัด + qbx.cpp 2.2k + gui.cpp 2.2k + audio.cpp 2.8k
- builtin registry 405 `regid`, 354 ชื่อ, 348 C callname, 28 stub
- ผู้ใช้ต้องมี g++/clang++ ตอน compile
- CI 3 OS build อย่างเดียว ไม่รันเทสต์
- ไม่มี setjmp/longjmp; error handling ผ่าน `QBMAIN(NULL)` ซ้อน

**Inferred**
- data race หลายจุดใน runtime (F3)
- snapshot drift ทำให้ใช้เป็น oracle ไม่ได้ถ้าไม่ regenerate (F1)
- ประมาณการ PM ทั้งหมด (Assumed)

## 4. AI Agent Notes (สำหรับคน/agent ที่มาทำต่อ)

- **อ่านก่อน:** `README.md` → `semantics-catalog.md` → `port-plan.md` → ไฟล์นี้
- **ห้าม**ใช้ `internal/source/*.txt` เป็นตัวอย่าง codegen ปัจจุบัน — มันล้าหลัง (F1). ถ้าต้องการ output ปัจจุบันให้รัน `qb -z file.bas` แล้วดู `internal/temp/`
- **ห้าม**เชื่อ `.eniac-plan.md` เรื่อง `runtime/` — ไม่มีอยู่จริง (F12)
- `build/` เป็น output ที่ gitignore ไม่ใช่ source
- จุดเริ่มอ่านโค้ด compiler: `source/qbnex.bas:1431` (prepass), `:2542` (main pass), `:10937` (recompile), `:11485` (native build)
- จุดเริ่มอ่าน runtime: `internal/c/qbx.cpp:2100` (`evnt`), `:2170-2214` (QBMAIN shell), `internal/c/libqb.cpp:28334` (`main`), `:6432` (`fix_error`), `:6763-7140` (string engine)
- ทุกพฤติกรรมใหม่ที่ implement ต้องมี conformance test ที่เทียบกับ binary เดิม — ถ้าต่าง ให้บันทึกเป็น deviation พร้อมเหตุผล ไม่ใช่ "แก้ให้ถูก" เอง
- Proposed ทุกข้อใน `port-plan.md` ยังไม่อนุมัติ — อย่าสร้าง Cargo workspace ก่อน D1/D2/D10 ได้คำตอบ
