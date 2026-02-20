use std::rc::Rc;
use std::{fmt, ops};

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

impl ops::Add<String> for Offset {
    type Output = Self;

    fn add(self, rhs: String) -> Self::Output {
        match self {
            Self::Label(l) => Self::Label(Rc::from(rhs + &l)),
            Self::Imm(i) => Self::Imm(i),
        }
    }
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
            Expr::Label(l) => Some(Offset::Label(l.value.0.clone())),
            _ => {
                summary.error(AnalyzeOffsetInvalid {
                    location: value.span(),
                });
                None
            }
        }
    }
}

impl fmt::Debug for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {

        }
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Offset::Label(l) => f.write_str(&*l),
            Offset::Imm(i) => f.write_fmt(format_args!("{i}")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Register {
    /// x0 | zero
    Zero,
    /// x1 | ra
    Return,
    /// x2 | sp
    Stack,
    /// x8 | s0 | fp
    Result,
    Custom(Rc<str>),
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
            Expr::Ident(i) => match &*i.value {
                "x0" | "zero" => Some(Self::Zero),
                "x1" | "ra" => Some(Self::Return),
                "x2" | "sp" => Some(Self::Stack),
                "x8" | "s0" | "fp" => Some(Self::Result),
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

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Register::Zero => f.write_str("x0"),
            Register::Return => f.write_str("ra"),
            Register::Stack => f.write_str("sp"),
            Register::Result => f.write_str("s0"),
            Register::Custom(c) => f.write_str(&c),
        }
    }
}
