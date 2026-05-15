mod attribute;
mod expr;
mod items;
mod statements;

pub use attribute::*;
pub use expr::*;
pub use items::*;
use miette::Diagnostic;
pub use statements::*;
use thiserror::Error;

use std::fmt::{self, Write as _};
use std::ops;
use std::rc::Rc;

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::generator::Generate;
use crate::parser::{IntoSpanned, Span, Spanned, Token};
use crate::pretty::{PrettyFormatter, PrettyPrint};

/// Identificator of functions or variables
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct Ident(pub Rc<str>);

impl PrettyPrint for Ident {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.write_str(&self.0)?;
        f.write_char('\n')
    }
}

impl ops::Deref for Ident {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &*self.0
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
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        self.0.analyze(summary)
    }
}

impl Generate for Program {
    fn generate(&self, buf: &mut crate::generator::GenerateBuf) {
        self.0.generate(buf)
    }
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Missing semicolon")]
pub struct AnalyzeMissingSemicolon {
    #[label("expected a semicolon")]
    pub location: Span,
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
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        if let Some(s) = self
            .stmts
            .iter()
            .rev()
            .skip(1)
            .find(|s| s.has_semi().is_some_and(|t| !t))
        {
            summary.error(AnalyzeMissingSemicolon {
                location: s.span().end_span(0),
            });
        }

        self.stmts.analyze(summary)
    }
}

impl Generate for Block {
    fn generate(&self, buf: &mut crate::generator::GenerateBuf) {
        self.stmts.generate(buf)
    }
}
