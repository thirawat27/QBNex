use crate::{CodeGenerator, LLVMBuilder};
use core_types::{QError, QResult};
use syntax_tree::Program;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityLevel {
    Production,
    Preview,
}

impl StabilityLevel {
    pub fn label(self) -> &'static str {
        match self {
            StabilityLevel::Production => "production",
            StabilityLevel::Preview => "preview",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBackendKind {
    Rust,
    LlvmIr,
    CraneliftJit,
}

impl NativeBackendKind {
    pub const ALL: [NativeBackendKind; 3] = [
        NativeBackendKind::Rust,
        NativeBackendKind::LlvmIr,
        NativeBackendKind::CraneliftJit,
    ];

    pub fn name(self) -> &'static str {
        match self {
            NativeBackendKind::Rust => "rust",
            NativeBackendKind::LlvmIr => "llvm-ir",
            NativeBackendKind::CraneliftJit => "cranelift-jit",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            NativeBackendKind::Rust => "production Rust source generator and executable builder",
            NativeBackendKind::LlvmIr => "preview LLVM IR text backend",
            NativeBackendKind::CraneliftJit => {
                "preview Cranelift-backed runner for a narrow BASIC subset"
            }
        }
    }

    pub fn stability(self) -> StabilityLevel {
        match self {
            NativeBackendKind::Rust => StabilityLevel::Production,
            NativeBackendKind::LlvmIr | NativeBackendKind::CraneliftJit => StabilityLevel::Preview,
        }
    }

    pub fn is_production_ready(self) -> bool {
        self.stability() == StabilityLevel::Production
    }
}

impl std::str::FromStr for NativeBackendKind {
    type Err = QError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "rust" | "default" => Ok(Self::Rust),
            "llvm" | "llvm-ir" => Ok(Self::LlvmIr),
            "cranelift" | "cranelift-jit" | "jit" => Ok(Self::CraneliftJit),
            other => Err(QError::UnsupportedFeature(format!(
                "unknown native backend '{other}' (expected rust, llvm-ir, or cranelift-jit)"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NativeBackendOptions {
    pub graphics: bool,
}

pub trait NativeTextBackend {
    fn name(&self) -> &'static str;
    fn generate(&mut self, program: &Program, options: NativeBackendOptions) -> QResult<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RustTextBackend;

impl NativeTextBackend for RustTextBackend {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn generate(&mut self, program: &Program, options: NativeBackendOptions) -> QResult<String> {
        let mut codegen = CodeGenerator::new();
        if options.graphics {
            codegen.enable_graphics();
        }
        codegen.generate(program)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LlvmIrTextBackend;

impl NativeTextBackend for LlvmIrTextBackend {
    fn name(&self) -> &'static str {
        "llvm-ir"
    }

    fn generate(&mut self, program: &Program, _options: NativeBackendOptions) -> QResult<String> {
        let mut builder = LLVMBuilder::new(program.clone());
        builder.build()
    }
}

pub fn generate_with_backend(
    program: &Program,
    backend: NativeBackendKind,
    options: NativeBackendOptions,
) -> QResult<String> {
    match backend {
        NativeBackendKind::Rust => {
            let mut backend = RustTextBackend;
            backend.generate(program, options)
        }
        NativeBackendKind::LlvmIr => {
            let mut backend = LlvmIrTextBackend;
            backend.generate(program, options)
        }
        NativeBackendKind::CraneliftJit => Err(core_types::QError::UnsupportedFeature(
            "cranelift-jit is a runtime backend, not a text code generator".to_string(),
        )),
    }
}
