use crate::{Parser, Program};
use core_types::{QError, QResult};

mod chumsky_frontend;

pub trait Frontend {
    fn name(&self) -> &'static str;
    fn parse_program(&self, input: String) -> QResult<Program>;
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrontendKind {
    #[default]
    Classic,
    Chumsky,
}

impl FrontendKind {
    pub const ALL: [FrontendKind; 2] = [FrontendKind::Classic, FrontendKind::Chumsky];

    pub fn name(self) -> &'static str {
        match self {
            FrontendKind::Classic => "classic",
            FrontendKind::Chumsky => "chumsky",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            FrontendKind::Classic => "full production recursive-descent frontend",
            FrontendKind::Chumsky => "preview recovery-oriented frontend for a narrow BASIC subset",
        }
    }

    pub fn stability(self) -> StabilityLevel {
        match self {
            FrontendKind::Classic => StabilityLevel::Production,
            FrontendKind::Chumsky => StabilityLevel::Preview,
        }
    }

    pub fn is_production_ready(self) -> bool {
        self.stability() == StabilityLevel::Production
    }

    pub fn parse(self, input: String) -> QResult<Program> {
        match self {
            FrontendKind::Classic => ClassicFrontend.parse_program(input),
            FrontendKind::Chumsky => ChumskyFrontend.parse_program(input),
        }
    }
}

impl std::str::FromStr for FrontendKind {
    type Err = QError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "classic" | "rd" | "recursive-descent" => Ok(Self::Classic),
            "chumsky" => Ok(Self::Chumsky),
            other => Err(QError::UnsupportedFeature(format!(
                "unknown frontend '{other}' (expected classic or chumsky)"
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

#[derive(Debug, Clone, Copy, Default)]
pub struct ChumskyFrontend;

impl Frontend for ChumskyFrontend {
    fn name(&self) -> &'static str {
        FrontendKind::Chumsky.name()
    }

    fn parse_program(&self, input: String) -> QResult<Program> {
        chumsky_frontend::parse_program(input)
    }
}

pub fn production_frontend() -> FrontendKind {
    FrontendKind::Classic
}

pub fn parse_with_frontend(kind: FrontendKind, input: String) -> QResult<Program> {
    kind.parse(input)
}
