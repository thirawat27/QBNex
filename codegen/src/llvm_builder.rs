use core_types::QResult;
use syntax_tree::Program;

pub struct LLVMBuilder {
    _program: Program,
}

impl LLVMBuilder {
    pub fn new(program: Program) -> Self {
        Self { _program: program }
    }

    pub fn build(&mut self) -> QResult<String> {
        // Preview-only LLVM IR generation scaffold.
        let target_triple = if cfg!(target_os = "windows") {
            "x86_64-pc-windows-msvc"
        } else if cfg!(target_os = "macos") {
            "x86_64-apple-darwin"
        } else {
            "x86_64-unknown-linux-gnu"
        };
        let mut ir_output = String::new();
        ir_output.push_str("; QBNex Generated LLVM IR\n");
        ir_output.push_str("; Preview LLVM backend scaffold\n\n");
        ir_output.push_str("source_filename = \"qbcom_program\"\n");
        ir_output.push_str(&format!("target triple = \"{}\"\n\n", target_triple));
        ir_output.push_str("declare i32 @printf(i8*, ...)\n");
        ir_output.push_str("declare void @exit(i32)\n\n");
        ir_output.push_str("define i32 @main() {\n");
        ir_output.push_str("entry:\n");
        ir_output.push_str("  ret i32 0\n");
        ir_output.push_str("}\n");

        Ok(ir_output)
    }
}
