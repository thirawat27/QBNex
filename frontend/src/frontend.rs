use crate::{Parser, Program};
use core_types::{QError, QResult};

pub trait Frontend {
    fn name(&self) -> &'static str;
    fn parse_program(&self, input: String) -> QResult<Program>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrontendKind {
    #[default]
    Classic,
}

impl FrontendKind {
    pub fn all() -> &'static [FrontendKind] {
        static ALL: [FrontendKind; 1] = [FrontendKind::Classic];
        &ALL
    }

    pub fn name(self) -> &'static str {
        match self {
            FrontendKind::Classic => "classic",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            FrontendKind::Classic => "production recursive-descent frontend",
        }
    }

    pub fn is_production_ready(self) -> bool {
        matches!(self, FrontendKind::Classic)
    }

    pub fn parse(self, input: String) -> QResult<Program> {
        match self {
            FrontendKind::Classic => ClassicFrontend.parse_program(input),
        }
    }
}

impl std::str::FromStr for FrontendKind {
    type Err = QError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "classic" | "rd" | "recursive-descent" => Ok(Self::Classic),
            other => Err(QError::UnsupportedFeature(format!(
                "unknown frontend '{other}' (expected classic)"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ClassicFrontend;

impl Frontend for ClassicFrontend {
    fn name(&self) -> &'static str {
        FrontendKind::Classic.name()
    }

    fn parse_program(&self, input: String) -> QResult<Program> {
        let mut parser = Parser::new(input)?;
        parser.parse()
    }
}

pub fn production_frontend() -> FrontendKind {
    FrontendKind::Classic
}

pub fn parse_with_frontend(kind: FrontendKind, input: String) -> QResult<Program> {
    kind.parse(input)
}
