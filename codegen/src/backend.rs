use crate::CodeGenerator;
use core_types::{QError, QResult};
use syntax_tree::Program;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBackendKind {
    Rust,
}

impl NativeBackendKind {
    pub fn all() -> &'static [NativeBackendKind] {
        static ALL: [NativeBackendKind; 1] = [NativeBackendKind::Rust];
        &ALL
    }

    pub fn name(self) -> &'static str {
        match self {
            NativeBackendKind::Rust => "rust",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            NativeBackendKind::Rust => "production Rust source generator and executable builder",
        }
    }

    pub fn is_production_ready(self) -> bool {
        matches!(self, NativeBackendKind::Rust)
    }
}

impl std::str::FromStr for NativeBackendKind {
    type Err = QError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "rust" | "default" => Ok(Self::Rust),
            other => Err(QError::UnsupportedFeature(format!(
                "unknown native backend '{other}' (expected rust)"
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
    }
}
