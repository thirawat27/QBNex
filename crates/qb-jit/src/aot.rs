//! Ahead-of-time compilation (D15): the same code generator writes an
//! object file instead of executable memory. `qb-link` turns the object into
//! a loadable image; the executable resolves runtime symbols when it starts.

use crate::codegen::{define_all, make_isa, CompileError, Target};
use crate::mir::Module;
use cranelift_module::{default_libcall_names, Linkage};
use cranelift_object::{ObjectBuilder, ObjectModule};

/// Compiles every function of `m` into an object file for `target`
/// (functions exported as `qb_func_<i>`, runtime calls as undefined symbols).
pub fn compile_object(m: &Module, target: Target) -> Result<Vec<u8>, CompileError> {
    let isa = make_isa(target);
    let ob = ObjectBuilder::new(isa, "qbnex_program", default_libcall_names()).map_err(|e| CompileError(e.to_string()))?;
    let mut om = ObjectModule::new(ob);
    define_all(&mut om, m, Linkage::Export)?;
    om.finish().emit().map_err(|e| CompileError(e.to_string()))
}