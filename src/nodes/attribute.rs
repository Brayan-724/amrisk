use amrisk_macros::Node;
use miette::Diagnostic;
use thiserror::Error;

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::parser::{IntoSpanned, Span, Spanned, Token};
use crate::pretty::{PrettyFormatter, PrettyPrint};

use super::Ident;

#[derive(Debug, Clone, Node)]
#[node()]
pub struct Attribute {
    pub pound: Spanned<Token>,
    pub open: Spanned<Token>,
    pub path: Spanned<Ident>,
    pub style: AttributeStyle,
    pub close: Spanned<Token>,
}

#[derive(Debug, Clone)]
pub enum AttributeStyle {
    Path,
}

impl IntoSpanned for Attribute {
    fn span(&self) -> Span {
        self.pound.span.merge(self.close.span)
    }
}

impl PrettyPrint for Attribute {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> std::fmt::Result {
        match self.style {
            AttributeStyle::Path => f
                .node("Attribute::Path", self.span())?
                .field("path", &self.path)?
                .finish(),
        }
    }
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Unrecognized attribute")]
pub struct AnalyzeUnknownAttribute {
    #[label("unknown attribute")]
    pub location: Span,
}

impl Analyze for Attribute {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        if let Err(_) = AttributeBuiltin::try_from(&*self) {
            summary.error(AnalyzeUnknownAttribute {
                location: self.path.span(),
            })
        }

        AnalyzeResult::Continue(())
    }
}

pub enum AttributeBuiltin {
    Entry,
}

impl TryFrom<&Attribute> for AttributeBuiltin {
    type Error = ();

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        match &**value.path {
            "entry" => Ok(AttributeBuiltin::Entry),
            _ => Err(()),
        }
    }
}
