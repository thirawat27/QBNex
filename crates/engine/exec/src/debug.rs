//! Debugger hook (feature `debug`): the VM calls a [`Debugger`] at every
//! statement start (the `stmts` line table built from QIR `line N`), and the
//! debugger can read the call stack and variables through [`Stop`].
//!
//! Variable names come from the QIR declarations. The register (or memory
//! block) of each variable is found by repeating the allocation order of
//! `lower.rs` (globals, then statics, in frame 0; then each function's
//! parameters and locals); `tests` check it against real programs. Without the
//! feature none of this is compiled and the VM is unchanged.

use qb_ir as ir;
use qb_ir::{StoreTy, Ty as IT, VarKind};

use super::{Arr, ArrData, Vm};
use crate::bytecode::{MTy, Module};
use crate::mem::ARR_TAG;

/// Most array elements a [`Value::Array`] carries.
pub const MAX_ELEMS: usize = 1000;

/// A variable's value, read while the program is stopped.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    UInt(u64),
    Single(f32),
    Double(f64),
    /// `_FLOAT`, converted to f64 for display.
    Float(f64),
    Str(Vec<u8>),
    /// Elements in storage order (first subscript fastest), at most [`MAX_ELEMS`].
    Array { dims: Vec<(i64, i64)>, elems: Vec<Value>, total: u64 },
    Udt { ty: String, fields: Vec<(String, Value)> },
    /// Not dimensioned yet, or not readable.
    Unavailable(String),
}

/// A variable with its QIR name (`$__INTEGER_A`, `$_SUB_F_LONG_N`, ...).
#[derive(Clone, Debug, PartialEq)]
pub struct VarValue {
    pub qir_name: String,
    pub kind: VarScope,
    pub value: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarScope {
    Global,
    Static,
    Param,
    Local,
}

/// One entry of the call stack.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameInfo {
    /// QIR function name (`main`, `SUB_BUMP`, `FUNC_FACT`).
    pub func: String,
    /// Source line of the current statement of this frame.
    pub line: u32,
}

/// Called by the VM at each statement start while attached.
pub trait Debugger {
    fn statement(&mut self, stop: &mut Stop<'_, '_, '_>);
}

/// Where a variable lives.
#[derive(Clone, Copy, Debug)]
enum Loc {
    N(u32, IT),
    /// `_FLOAT` register pair
    X(u32),
    S(u32),
    A(u32),
    /// memory mode: register holding the address (or array descriptor)
    M(u32),
}

#[derive(Clone, Debug)]
struct Slot {
    decl: ir::VarDecl,
    loc: Loc,
    /// lives in frame 0 (globals and statics)
    frame0: bool,
    scope: VarScope,
}

pub(crate) struct DebugState {
    pub(crate) hook: Option<Box<dyn Debugger>>,
    starts: Vec<bool>,
    /// per lowered function: its variables
    vars: Vec<Vec<Slot>>,
    types: Vec<ir::UdtDef>,
}

#[derive(Default)]
struct Counter {
    n: u32,
    s: u32,
    a: u32,
}

/// Repeats `lower.rs`'s storage allocation to name registers.
fn layout(src: &ir::Module, m: &Module) -> Result<Vec<Vec<Slot>>, String> {
    let mem = !m.mvars.is_empty();
    let order: Vec<&ir::Func> = m
        .funcs
        .iter()
        .map(|f| src.func(&f.name).ok_or_else(|| format!("function {} not in QIR", f.name)))
        .collect::<Result<_, _>>()?;
    let mut taken = std::collections::HashSet::new();
    if mem {
        for (k, f) in order.iter().enumerate() {
            let own: std::collections::HashSet<&str> = f.params.iter().chain(&f.locals).map(|d| d.name.as_str()).collect();
            for i in &f.body {
                if let ir::Inst::Addr { var, .. } | ir::Inst::ARef { var, .. } | ir::Inst::AAddr { var, .. } = i {
                    let scope = if own.contains(var.as_str()) { k } else { usize::MAX };
                    taken.insert((scope, var.clone()));
                }
            }
        }
    }
    let alloc = |c: &mut Counter, d: &ir::VarDecl, scope: usize| -> Loc {
        if mem && (d.array || !matches!(d.ty, StoreTy::Scalar(_)) || taken.contains(&(scope, d.name.clone()))) {
            c.n += 1;
            return Loc::M(c.n - 1);
        }
        let t = match d.ty {
            StoreTy::Scalar(t) => t,
            _ => IT::Str,
        };
        if d.array {
            c.a += 1;
            Loc::A(c.a - 1)
        } else if t == IT::Str {
            c.s += 1;
            Loc::S(c.s - 1)
        } else if t == IT::F80 {
            c.n += 2;
            Loc::X(c.n - 2)
        } else {
            c.n += 1;
            Loc::N(c.n - 1, t)
        }
    };
    let mut out: Vec<Vec<Slot>> = vec![Vec::new(); order.len()];
    let mut c0 = Counter::default();
    for g in &src.globals {
        let loc = alloc(&mut c0, g, usize::MAX);
        out[0].push(Slot { decl: g.clone(), loc, frame0: true, scope: VarScope::Global });
    }
    for (k, f) in order.iter().enumerate() {
        for l in f.locals.iter().filter(|l| l.kind == VarKind::Static) {
            let loc = alloc(&mut c0, l, k);
            out[k].push(Slot { decl: l.clone(), loc, frame0: true, scope: VarScope::Static });
        }
    }
    for (k, f) in order.iter().enumerate() {
        let mut fresh = Counter::default();
        let c = if k == 0 { &mut c0 } else { &mut fresh };
        for p in &f.params {
            let byref = matches!(p.kind, VarKind::Param { byref: true }) || p.array;
            let loc = if mem && byref {
                c.n += 1;
                Loc::M(c.n - 1)
            } else if mem && (!matches!(p.ty, StoreTy::Scalar(_)) || taken.contains(&(k, p.name.clone()))) {
                // pointer register, then the incoming value's register(s)
                let ptr = c.n;
                c.n += 1;
                match p.ty {
                    StoreTy::Scalar(IT::Str) => c.s += 1,
                    StoreTy::Scalar(IT::F80) => c.n += 2,
                    _ => c.n += 1,
                }
                Loc::M(ptr)
            } else {
                alloc(c, p, k)
            };
            out[k].push(Slot { decl: p.clone(), loc, frame0: false, scope: VarScope::Param });
        }
        for l in f.locals.iter().filter(|l| l.kind != VarKind::Static) {
            let loc = alloc(c, l, k);
            out[k].push(Slot { decl: l.clone(), loc, frame0: k == 0, scope: VarScope::Local });
        }
    }
    Ok(out)
}

impl DebugState {
    pub(crate) fn new(src: &ir::Module, m: &Module, hook: Box<dyn Debugger>) -> Result<DebugState, String> {
        let mut starts = vec![false; m.code.len() + 1];
        for &(pc, _) in &m.stmts {
            if let Some(s) = starts.get_mut(pc as usize) {
                *s = true;
            }
        }
        Ok(DebugState { hook: Some(hook), starts, vars: layout(src, m)?, types: src.types.clone() })
    }

    #[inline(always)]
    pub(crate) fn is_start(&self, pc: u32) -> bool {
        self.starts.get(pc as usize).copied().unwrap_or(false)
    }
}

/// The stopped program, handed to [`Debugger::statement`].
pub struct Stop<'a, 'm, 'o> {
    pub(crate) vm: &'a mut Vm<'m, 'o>,
    /// pc of the statement about to run
    pub pc: u32,
    /// its source line
    pub line: u32,
}

fn mty(t: IT) -> MTy {
    match t {
        IT::I8 => MTy::I8,
        IT::U8 => MTy::U8,
        IT::I16 => MTy::I16,
        IT::U16 => MTy::U16,
        IT::I32 => MTy::I32,
        IT::U32 => MTy::U32,
        IT::I64 => MTy::I64,
        IT::U64 => MTy::U64,
        IT::F32 => MTy::F32,
        IT::F64 => MTy::F64,
        IT::F80 => MTy::F80,
        IT::Str => MTy::Str,
        IT::Ptr => MTy::Ptr,
    }
}

fn num(t: IT, bits: u64) -> Value {
    match t {
        IT::I8 | IT::I16 | IT::I32 | IT::I64 => Value::Int(bits as i64),
        IT::U8 | IT::U16 | IT::U32 | IT::U64 | IT::Ptr => Value::UInt(bits),
        IT::F32 => Value::Single(f64::from_bits(bits) as f32),
        IT::F64 => Value::Double(f64::from_bits(bits)),
        IT::F80 | IT::Str => Value::Unavailable("bad register type".into()),
    }
}

fn reg_array(a: &Arr) -> Value {
    let total: u64 = a.dims.iter().map(|(lo, hi)| (hi - lo + 1) as u64).product();
    let take = |n: usize| n.min(MAX_ELEMS);
    let elems: Vec<Value> = match &a.data {
        ArrData::I16(v) => v[..take(v.len())].iter().map(|&x| Value::Int(i64::from(x))).collect(),
        ArrData::I32(v) => v[..take(v.len())].iter().map(|&x| Value::Int(i64::from(x))).collect(),
        ArrData::I64(v) => v[..take(v.len())].iter().map(|&x| Value::Int(x)).collect(),
        ArrData::F32(v) => v[..take(v.len())].iter().map(|&x| Value::Single(x)).collect(),
        ArrData::F64(v) => v[..take(v.len())].iter().map(|&x| Value::Double(x)).collect(),
        ArrData::Str(v) => v[..take(v.len())].iter().map(|x| Value::Str(x.clone())).collect(),
    };
    Value::Array { dims: a.dims.clone(), elems, total }
}

impl Stop<'_, '_, '_> {
    /// Number of SUB/FUNCTION frames (1 = module level).
    pub fn depth(&self) -> usize {
        self.vm.frames.len()
    }

    /// Writes buffered program output.
    pub fn flush(&mut self) {
        self.vm.rt.flush();
    }

    /// Call stack, innermost first.
    pub fn stack(&self) -> Vec<FrameInfo> {
        let n = self.vm.frames.len();
        (0..n)
            .rev()
            .map(|i| {
                let pc = if i + 1 == n { self.pc } else { self.vm.frames[i + 1].ret.saturating_sub(1) };
                let f = self.vm.frames[i].func as usize;
                FrameInfo { func: self.vm.m.funcs[f].name.clone(), line: self.vm.line_of(pc) }
            })
            .collect()
    }

    /// Module-level variables (globals, `DIM SHARED`).
    pub fn globals(&mut self) -> Vec<VarValue> {
        let slots: Vec<Slot> = self.state().vars[0].iter().filter(|s| s.scope == VarScope::Global).cloned().collect();
        slots.iter().map(|s| self.read_slot(s, 0)).collect()
    }

    /// Parameters, locals and statics of stack entry `index` (0 = innermost,
    /// as in [`Stop::stack`]). Compiler temporaries (`$~T...`) are left out.
    pub fn locals(&mut self, index: usize) -> Vec<VarValue> {
        let n = self.vm.frames.len();
        let Some(fi) = n.checked_sub(index + 1) else { return Vec::new() };
        let k = self.vm.frames[fi].func as usize;
        let slots: Vec<Slot> = self.state().vars[k]
            .iter()
            .filter(|s| s.scope != VarScope::Global && !s.decl.name.starts_with("$~"))
            .cloned()
            .collect();
        slots.iter().map(|s| self.read_slot(s, fi)).collect()
    }

    fn state(&self) -> &DebugState {
        self.vm.dbg.as_ref().expect("debugger attached")
    }

    fn read_slot(&mut self, s: &Slot, frame: usize) -> VarValue {
        let f = if s.frame0 { self.vm.frames[0] } else { self.vm.frames[frame] };
        let value = match s.loc {
            Loc::N(r, t) => self.vm.nums.get(f.nb + r as usize).map_or(Value::Unavailable("register".into()), |&b| num(t, b)),
            Loc::X(r) => match (self.vm.nums.get(f.nb + r as usize), self.vm.nums.get(f.nb + r as usize + 1)) {
                (Some(&m), Some(&se)) => Value::Float(qb_float80::F80 { mant: m, se: se as u16 }.to_f64()),
                _ => Value::Unavailable("register".into()),
            },
            Loc::S(r) => self.vm.strs.get(f.sb + r as usize).map_or(Value::Unavailable("register".into()), |s| Value::Str(s.clone())),
            Loc::A(r) => match self.vm.arrs.get(f.ab + r as usize) {
                Some(Some(a)) => reg_array(a),
                _ => Value::Unavailable("not dimensioned".into()),
            },
            Loc::M(r) => match self.vm.nums.get(f.nb + r as usize).copied() {
                Some(p) => self.read_mem(&s.decl.ty, s.decl.array, p),
                None => Value::Unavailable("register".into()),
            },
        };
        VarValue { qir_name: s.decl.name.clone(), kind: s.scope, value }
    }

    fn read_mem(&mut self, ty: &StoreTy, array: bool, p: u64) -> Value {
        if array {
            return self.read_mem_array(ty, p);
        }
        self.read_mem_scalar(ty, p, 0)
    }

    fn read_mem_array(&mut self, ty: &StoreTy, desc: u64) -> Value {
        if desc & ARR_TAG == 0 {
            return Value::Unavailable("array".into());
        }
        let mut dims = Vec::new();
        for k in 1..=60 {
            match (self.vm.mem.bound(desc, k, false), self.vm.mem.bound(desc, k, true)) {
                (Ok(lo), Ok(hi)) => dims.push((lo, hi)),
                _ => break,
            }
        }
        if dims.is_empty() {
            return Value::Unavailable("not dimensioned".into());
        }
        let total: u64 = dims.iter().map(|(lo, hi)| (hi - lo + 1) as u64).product();
        let mut elems = Vec::new();
        let mut idx: Vec<i64> = dims.iter().map(|d| d.0).collect();
        'all: while (elems.len() as u64) < total.min(MAX_ELEMS as u64) {
            let v = match self.vm.mem.aaddr(desc, &idx) {
                Ok(a) => self.read_mem_scalar(ty, a, 0),
                Err(_) => Value::Unavailable("element".into()),
            };
            elems.push(v);
            // first subscript varies fastest
            for (i, d) in dims.iter().enumerate() {
                if idx[i] < d.1 {
                    idx[i] += 1;
                    continue 'all;
                }
                idx[i] = d.0;
            }
            break;
        }
        Value::Array { dims, elems, total }
    }

    fn read_mem_scalar(&mut self, ty: &StoreTy, p: u64, depth: u32) -> Value {
        let r = match ty {
            StoreTy::Scalar(IT::Str) => self.vm.mem.ld_str(p).map(|s| Value::Str(s.to_vec())),
            StoreTy::Scalar(IT::F80) => self.vm.mem.bytes(p, 10).map(|b| {
                let mant = u64::from_le_bytes(b[..8].try_into().unwrap());
                let se = u16::from_le_bytes([b[8], b[9]]);
                Value::Float(qb_float80::F80 { mant, se }.to_f64())
            }),
            StoreTy::Scalar(t) => self.vm.mem.ld(mty(*t), p).map(|b| num(*t, b)),
            StoreTy::FStr(n) => self.vm.mem.ldfs(p, *n).map(Value::Str),
            StoreTy::Udt(name) => {
                let Some(def) = self.state().types.iter().find(|t| &t.name == name).cloned() else {
                    return Value::Unavailable(format!("TYPE {name}"));
                };
                if depth > 32 {
                    return Value::Unavailable("nesting".into());
                }
                let fields = def
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), self.read_mem_scalar(&f.ty, p.wrapping_add(u64::from(f.offset)), depth + 1)))
                    .collect();
                Ok(Value::Udt { ty: name.clone(), fields })
            }
        };
        r.unwrap_or_else(|_| Value::Unavailable("memory".into()))
    }
}

impl<'m, 'o> Vm<'m, 'o> {
    /// Attaches a debugger: the JIT is turned off and `hook` runs at every
    /// statement start. `src` is the QIR module the bytecode was lowered from.
    pub fn attach_debugger(&mut self, src: &ir::Module, hook: Box<dyn Debugger>) -> Result<(), String> {
        self.dbg = Some(DebugState::new(src, self.m, hook)?);
        self.jit = None;
        Ok(())
    }

    #[inline(never)]
    pub(crate) fn debug_statement(&mut self, pc: u32) {
        let Some(st) = self.dbg.as_mut() else { return };
        if !st.is_start(pc) {
            return;
        }
        let Some(mut hook) = st.hook.take() else { return };
        let line = self.line_of(pc);
        hook.statement(&mut Stop { vm: self, pc, line });
        if let Some(st) = self.dbg.as_mut() {
            st.hook = Some(hook);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// (line, stack, globals, innermost locals)
    type Entry = (u32, Vec<FrameInfo>, Vec<VarValue>, Vec<VarValue>);
    type Log = Arc<Mutex<Vec<Entry>>>;

    struct Recorder(Log, Vec<u32>);

    impl Debugger for Recorder {
        fn statement(&mut self, stop: &mut Stop<'_, '_, '_>) {
            if self.1.contains(&stop.line) {
                let stack = stop.stack();
                let g = stop.globals();
                let l = stop.locals(0);
                self.0.lock().unwrap().push((stop.line, stack, g, l));
            }
        }
    }

    fn run(qir: &str, lines: &[u32]) -> Vec<Entry> {
        let src = ir::parse(qir).unwrap();
        let m = crate::lower(&src).unwrap();
        let mut out = Vec::new();
        let log: Log = Arc::default();
        let mut vm = Vm::new(&m, &mut out);
        vm.attach_debugger(&src, Box::new(Recorder(log.clone(), lines.to_vec()))).unwrap();
        vm.run().unwrap();
        drop(vm);
        Arc::try_unwrap(log).unwrap().into_inner().unwrap()
    }

    fn find<'a>(v: &'a [VarValue], name: &str) -> &'a Value {
        &v.iter().find(|x| x.qir_name == name).unwrap_or_else(|| panic!("{name} not in {v:?}")).value
    }

    const SUBS: &str = include_str!("../tests/fixtures/corpus/control_subs_functions.qir");
    const UDT: &str = include_str!("../tests/fixtures/corpus/data_udt.qir");

    #[test]
    fn registers_stack_and_statics() {
        // line 16: `x = x + 1` inside SUB Bump; line 29: `total = total + c` in Counter
        let log = run(SUBS, &[16, 29]);
        let (line, stack, globals, locals) = &log[0];
        assert_eq!(*line, 16);
        assert_eq!(stack.iter().map(|f| (f.func.as_str(), f.line)).collect::<Vec<_>>(), vec![("SUB_BUMP", 16), ("main", 5)]);
        assert_eq!(*find(globals, "$__INTEGER_A"), Value::Int(5));
        assert_eq!(*find(locals, "$_SUB_BUMP_INTEGER_X"), Value::Int(5));
        // Counter runs three times; its STATIC c counts 1, 2, 3 (read before line 29 runs).
        let counter: Vec<&Value> = log.iter().filter(|e| e.0 == 29).map(|e| find(&e.3, "$_SUB_COUNTER_SINGLE_C")).collect();
        assert_eq!(counter, vec![&Value::Single(1.0), &Value::Single(2.0), &Value::Single(3.0)]);
    }

    #[test]
    fn memory_mode_udt_and_arrays() {
        // line 20: `PRINT pts(1).x; pts(0).x` after the assignments
        let log = run(UDT, &[20]);
        let g = &log[0].2;
        let Value::Udt { fields, .. } = find(g, "$__UDT_B") else { panic!() };
        let Value::Udt { fields: lo, .. } = &fields[0].1 else { panic!() };
        assert_eq!(lo[0], ("X".to_string(), Value::Int(1)));
        assert_eq!(lo[1], ("Y".to_string(), Value::Single(1.5)));
        assert_eq!(lo[2], ("TAG".to_string(), Value::Str(b"lo  ".to_vec())));
        assert_eq!(fields[2], ("ID".to_string(), Value::Int(1_234_567_890_123)));
        let Value::Array { dims, elems, total } = find(g, "$__ARRAY_UDT_PTS") else { panic!() };
        assert_eq!((dims.as_slice(), *total), (&[(0, 2)][..], 3));
        let Value::Udt { fields: p1, .. } = &elems[1] else { panic!() };
        assert_eq!(p1[0].1, Value::Int(42));
    }
}
