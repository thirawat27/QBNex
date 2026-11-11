//! Ahead-of-time compilation (D15): the same code generator writes an
//! object file instead of executable memory.
//!
//! - [`compile_object`]: functions only; `qb-link` turns the object into a
//!   loadable image that the payload runner loads at start-up.
//! - [`compile_native_object`]: functions plus the program and an entry
//!   table, for a standard linker (rust-lld) to link with the `qb-native`
//!   runtime library into an ordinary executable.

use crate::codegen::{define_all, make_isa_pic, CompileError, Target};
use crate::mir::Module;
use cranelift_module::{default_libcall_names, DataDescription, Linkage, Module as _};
use cranelift_object::{ObjectBuilder, ObjectModule};

/// Name of the entry table symbol of a native object (see [`NativeEntry`]).
pub const ENTRY_SYMBOL: &str = "qb_native_entry";

/// Layout of the `qb_native_entry` table a native object exports.
#[repr(C)]
pub struct NativeEntry {
    /// identifies program + runtime layout (checked by the runtime)
    pub fingerprint: u64,
    /// number of entries in `funcs`
    pub nfuncs: u64,
    /// the program (QIR text)
    pub program: *const u8,
    pub program_len: u64,
    /// `qb_func_<i>` for every MIR function, in order
    pub funcs: *const crate::NativeFn,
}

/// Native objects are position-independent except on Windows (COFF code is
/// relocated by the loader; Linux PIEs and every macOS executable need PIC).
fn native_pic(target: Target) -> bool {
    match target {
        Target::Host => !cfg!(windows),
        _ => true,
    }
}

fn new_module(target: Target) -> Result<ObjectModule, CompileError> {
    new_module_pic(target, false)
}

fn new_module_pic(target: Target, pic: bool) -> Result<ObjectModule, CompileError> {
    let isa = make_isa_pic(target, pic);
    let ob = ObjectBuilder::new(isa, "qbnex_program", default_libcall_names()).map_err(|e| CompileError(e.to_string()))?;
    Ok(ObjectModule::new(ob))
}

fn err(e: impl std::fmt::Display) -> CompileError {
    CompileError(e.to_string())
}

/// Compiles every function of `m` into an object file for `target`
/// (functions exported as `qb_func_<i>`, runtime calls as undefined symbols).
pub fn compile_object(m: &Module, target: Target) -> Result<Vec<u8>, CompileError> {
    let mut om = new_module(target)?;
    define_all(&mut om, m, Linkage::Export)?;
    om.finish().emit().map_err(err)
}

/// Like [`compile_object`], plus read-only data: `program` (the QIR text) and
/// the entry table [`ENTRY_SYMBOL`] ([`NativeEntry`]) pointing at it and at
/// every function. The runtime library finds the program through that table.
pub fn compile_native_object(m: &Module, target: Target, program: &[u8], fingerprint: u64) -> Result<Vec<u8>, CompileError> {
    let mut om = new_module_pic(target, native_pic(target))?;
    let ids = define_all(&mut om, m, Linkage::Local)?;

    let prog = om.declare_data("qb_native_program", Linkage::Local, false, false).map_err(err)?;
    let mut d = DataDescription::new();
    let mut bytes = program.to_vec();
    bytes.push(0);
    d.define(bytes.into_boxed_slice());
    om.define_data(prog, &d).map_err(err)?;

    let funcs = om.declare_data("qb_native_funcs", Linkage::Local, false, false).map_err(err)?;
    let mut d = DataDescription::new();
    d.define(vec![0u8; 8 * ids.len().max(1)].into_boxed_slice());
    for (i, id) in ids.iter().enumerate() {
        let f = om.declare_func_in_data(*id, &mut d);
        d.write_function_addr((8 * i) as u32, f);
    }
    om.define_data(funcs, &d).map_err(err)?;

    let entry = om.declare_data(ENTRY_SYMBOL, Linkage::Export, false, false).map_err(err)?;
    let mut d = DataDescription::new();
    let mut t = Vec::with_capacity(40);
    t.extend_from_slice(&fingerprint.to_le_bytes());
    t.extend_from_slice(&(ids.len() as u64).to_le_bytes());
    t.extend_from_slice(&[0; 8]);
    t.extend_from_slice(&(program.len() as u64).to_le_bytes());
    t.extend_from_slice(&[0; 8]);
    d.define(t.into_boxed_slice());
    let gp = om.declare_data_in_data(prog, &mut d);
    d.write_data_addr(16, gp, 0);
    let gf = om.declare_data_in_data(funcs, &mut d);
    d.write_data_addr(32, gf, 0);
    om.define_data(entry, &d).map_err(err)?;

    om.finish().emit().map_err(err)
}
