use std::{error::Error, fmt::Display};

use crate::config::SymbolTableError;

pub enum SemanticError {
    BetweenActionInUnboundedRule,
}

impl SemanticError {
    pub fn as_str(&self) -> &'static str {
        match self {
            SemanticError::BetweenActionInUnboundedRule => {
                "Use of BETWEEN operator in action inside an rule without a BETWEEN trigger."
            }
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
