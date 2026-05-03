use amrisk_macros::Node;
use miette::Diagnostic;
use thiserror::Error;

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::parser::{IntoSpanned, Span, Spanned, Token};
use crate::pretty::{PrettyFormatter, PrettyPrint};

use super::{Expr, Ident};

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
    Expr { equal: Spanned<Token>, value: Expr },
}

impl IntoSpanned for Attribute {
    fn span(&self) -> Span {
        self.pound.span.merge(self.close.span)
    }
}

impl PrettyPrint for Attribute {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> std::fmt::Result {
        match &self.style {
            AttributeStyle::Path => f
                .node("Attribute::Path", self.span())?
                .field("path", &self.path)?
                .finish(),
            AttributeStyle::Expr { value, .. } => f
                .node("Attribute::Expr", self.span())?
                .field("path", &self.path)?
                .field_child("value", value)?
                .finish(),
        }
    }
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Unrecognized attribute")]
pub struct AnalyzeAttributeUnknown {
    #[label("unknown attribute")]
    pub location: Span,
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("'{name}' only accepts path style")]
pub struct AnalyzeAttributeExpectedPath {
    #[label("help: remove this")]
    pub location: Span,

    pub name: Box<str>,
}

impl Analyze for Attribute {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        if let Err(err) = AttributeBuiltin::try_from(&*self) {
            match err {
                AttributeBuiltinError::UnknownAttribute => summary.error(AnalyzeAttributeUnknown {
                    location: self.path.span(),
                }),
                AttributeBuiltinError::ExpectedPathStyle => {
                    summary.error(AnalyzeAttributeExpectedPath {
                        location: self
                            .path
                            .span()
                            .end_span(0)
                            .merge(self.close.span.start_span(0)),

                        name: Box::from(&*self.path.0),
                    })
                }
            }
        }

        AnalyzeResult::Continue(())
    }
}

pub enum AttributeBuiltin {
    Entry,
}

pub enum AttributeBuiltinError {
    UnknownAttribute,
    ExpectedPathStyle,
}

impl TryFrom<&Attribute> for AttributeBuiltin {
    type Error = AttributeBuiltinError;

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        match &**value.path {
            "entry" if let AttributeStyle::Path = &value.style => Ok(AttributeBuiltin::Entry),
            "entry" => Err(AttributeBuiltinError::ExpectedPathStyle),

            _ => Err(AttributeBuiltinError::UnknownAttribute),
        }
    }
}
