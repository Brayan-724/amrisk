use std::rc::Rc;

use miette::Diagnostic;
use thiserror::Error;

use crate::analysis::AnalyzeSummary;
use crate::nodes::Expr;
use crate::parser::{IntoSpanned, Span};

#[derive(Debug, Clone)]
pub enum Offset {
    Label(Rc<str>),
    Imm(i32),
}

#[derive(Debug, Error, Diagnostic)]
#[error("Not an immediate")]
pub struct AnalyzeImmInvalid {
    #[label("expected an immediate")]
    location: Span,
}

#[derive(Debug, Error, Diagnostic)]
#[error("Not a offset")]
pub struct AnalyzeOffsetInvalid {
    #[label("expected a offset")]
    location: Span,
}

impl Offset {
    pub fn imm_from_expr(value: &Expr, summary: &mut AnalyzeSummary) -> Option<i32> {
        match value {
            Expr::Number(n) => Some(n.value),
            _ => {
                summary.error(AnalyzeImmInvalid {
                    location: value.span(),
                });
                None
            }
        }
    }

    pub fn from_expr(value: &Expr, summary: &mut AnalyzeSummary) -> Option<Self> {
        match value {
            Expr::Number(n) => Some(Offset::Imm(n.value)),
            Expr::Label(l) => Some(Offset::Label(Rc::from(l.value.0.clone()))),
            _ => {
                summary.error(AnalyzeOffsetInvalid {
                    location: value.span(),
                });
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Register {
    Zero,
    Custom(String),
}

// #[derive(Debug, Error, Diagnostic)]
// #[error("Unknown register")]
// pub struct AnalyzeRegNotFound {
//     #[label("expected a register")]
//     location: Span,
// }

#[derive(Debug, Error, Diagnostic)]
#[error("Not a register")]
pub struct AnalyzeRegInvalid {
    #[label("expected a register")]
    location: Span,
}

impl Register {
    pub fn from_expr(value: &Expr, summary: &mut AnalyzeSummary) -> Option<Self> {
        match value {
            Expr::Ident(i) => match i.value.0.as_str() {
                "x0" | "zero" => Some(Self::Zero),
                _ => Some(Self::Custom(i.value.0.clone())),
            },
            _ => {
                summary.error(AnalyzeRegInvalid {
                    location: value.span(),
                });
                None
            }
        }
    }
}
