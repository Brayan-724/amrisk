mod instructions;
mod registers;

use miette::Diagnostic;
use thiserror::Error;

pub use instructions::*;
pub use registers::*;

use crate::parser::Span;

pub struct GenerateBuf {
    buf: Vec<Instruction>,
}

pub trait Generate {
    fn generate(&self, buf: &mut GenerateBuf);
}

#[derive(Debug, Error, Diagnostic)]
#[error("Instruction does not exist")]
pub struct AnalyzeInsNotExist {
    #[label]
    location: Span,
}
