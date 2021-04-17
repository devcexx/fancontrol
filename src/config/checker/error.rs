use std::{borrow::Cow, error::Error, fmt::Display};

use crate::config::SymbolTableError;

#[allow(dead_code)]
pub enum NumBoundary {
    Greater(i32),
    GreaterOrEqual(i32),
    LessOrEqual(i32),
    Less(i32),
    BetweenStartExclusive(i32, i32),
    BetweenEndExclusive(i32, i32),
    BetweenBothExclusive(i32, i32),
}

impl NumBoundary {
    pub fn as_str(&self) -> String {
        match self {
            NumBoundary::Greater(lo) => format!("greater than {}", lo),
            NumBoundary::GreaterOrEqual(lo) => format!("greater or equal than {}", lo),
            NumBoundary::LessOrEqual(hi) => format!("less or equal than {}", hi),
            NumBoundary::Less(hi) => format!("less than {}", hi),
            NumBoundary::BetweenStartExclusive(lo, hi) => {
                format!("greater than {} and less or equal than {}", lo, hi)
            }
            NumBoundary::BetweenEndExclusive(lo, hi) => {
                format!("greater or equal than {} and less than {}", lo, hi)
            }
            NumBoundary::BetweenBothExclusive(lo, hi) => format!("between {} and {}", lo, hi),
        }
    }
}

pub enum SemanticError {
    BetweenActionInUnboundedRule,
    NumberOutOfBounds(NumBoundary, i32),
    InvalidPercent(i32),
}

impl SemanticError {
    pub fn as_str(&self) -> Cow<str> {
        match self {
            SemanticError::BetweenActionInUnboundedRule => {
                "Use of BETWEEN operator in action inside an rule without a BETWEEN trigger.".into()
            }
            SemanticError::NumberOutOfBounds(boundary, got) => {
                format!("Expected a number {}, but {} got.", boundary.as_str(), got).into()
            }
            SemanticError::InvalidPercent(got) => format!(
                "Invalid percent value. Expected a value between 0% and 100%, but {}% got.",
                got
            )
            .into(),
        }
    }
}

impl Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Semantic Error: {}", self.as_str())
    }
}

pub enum ProgramCheckError {
    SymbolTableError(SymbolTableError),
    SemanticError(SemanticError),
    Other(Box<dyn Error>),
}

impl From<SymbolTableError> for ProgramCheckError {
    fn from(e: SymbolTableError) -> Self {
        ProgramCheckError::SymbolTableError(e)
    }
}

pub type ProgramCheckResult<T> = Result<T, ProgramCheckError>;

impl Display for ProgramCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            &ProgramCheckError::SymbolTableError(error) => error.fmt(f),
            &ProgramCheckError::Other(error) => error.fmt(f),
            &ProgramCheckError::SemanticError(error) => error.fmt(f),
        }
    }
}
