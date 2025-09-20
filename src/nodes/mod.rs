mod expr;
mod items;
mod statements;

pub use expr::*;
pub use items::*;
pub use statements::*;

use std::fmt::{self, Write as _};

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::parser::{IntoSpanned, PrettyFormatter, PrettyPrint, Span, Spanned, Token};

/// Identificator of functions or variables
#[derive(Debug, Clone)]
pub struct Ident(pub String);

impl PrettyPrint for Ident {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.write_str(&self.0)?;
        f.write_char('\n')
    }
}

#[derive(Debug, Clone)]
pub struct Program(pub Vec<Item>);

impl IntoSpanned for Program {
    fn span(&self) -> Span {
        self.0.first().map(|i| i.span()).unwrap_or_default()
    }
}

impl PrettyPrint for Program {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.node("Program", self.span())?.children(&self.0)?.finish()
    }
}

impl Analyze for Program {
    fn analyze(&self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        self.0.analyze(summary)
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub open: Spanned<Token>,
    pub stmts: Vec<Statement>,
    pub close: Spanned<Token>,
}

impl IntoSpanned for Block {
    fn span(&self) -> Span {
        self.open.span.merge(self.close.span)
    }
}

impl PrettyPrint for Block {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.write_header("Block", self.span())?;
        f.write_children(&self.stmts)
    }
}

impl Analyze for Block {
    fn analyze(&self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        self.stmts.analyze(summary)
    }
}
