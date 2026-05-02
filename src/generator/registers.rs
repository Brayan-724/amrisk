use std::rc::Rc;
use std::{fmt, ops};

use miette::Diagnostic;
use thiserror::Error;

use crate::analysis::AnalyzeSummary;
use crate::nodes::Expr;
use crate::parser::{IntoSpanned, Span};

#[derive(Clone)]
#[repr(transparent)]
pub struct Imm(pub i32);

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Not an immediate")]
pub struct AnalyzeImmInvalid {
    #[label("expected an immediate")]
    pub location: Span,
}

impl Imm {
    pub fn from_expr(value: &Expr, summary: &mut AnalyzeSummary) -> Option<Imm> {
        match value {
            Expr::Number(n) => Some(Self(n.value)),
            _ => {
                summary.error(AnalyzeImmInvalid {
                    location: value.span(),
                });
                None
            }
        }
    }
}

impl fmt::Debug for Imm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Imm({})", self.0))
    }
}

impl fmt::Display for Imm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_fmt(format_args!("\x1b[1;38;5;33m{}\x1b[0m", self.0))
        } else {
            f.write_fmt(format_args!("{}", self.0))
        }
    }
}

#[derive(Clone)]
pub enum Offset {
    Label(Rc<str>),
    Imm(i32),
    Relative(i32, Register),
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Not a offset")]
pub struct AnalyzeOffsetInvalid {
    #[label("expected a offset")]
    pub location: Span,
}

impl ops::Add<String> for Offset {
    type Output = Self;

    fn add(self, rhs: String) -> Self::Output {
        match self {
            Self::Label(l) => Self::Label(Rc::from(rhs + &l)),
            Self::Imm(i) => Self::Imm(i),
            Self::Relative(i, reg) => Self::Relative(i, reg),
        }
    }
}

impl Offset {
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
            Self::Label(l) => f.write_fmt(format_args!("Label({l})")),
            Self::Imm(i) => f.write_fmt(format_args!("Offset({i})")),
            Self::Relative(i, register) => f.write_fmt(format_args!("Relative({i}, {register:?})")),
        }
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Offset::Label(l) => f.write_str(&*l),
            Offset::Imm(i) | Offset::Relative(i, ..) => {
                if f.alternate() {
                    f.write_fmt(format_args!("\x1b[1;38;5;93m{i}\x1b[0m"))?;
                } else {
                    f.write_fmt(format_args!("{i}"))?;
                }

                if let Offset::Relative(_, reg) = self {
                    f.write_str("(")?;

                    reg.fmt(f)?;

                    f.write_str(")")
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Register {
    /// x0 | zero
    Zero,
    /// x1 | ra | Return address
    Return,
    /// x2 | sp | Stack pointer
    Stack,
    /// x3 | gp | Global pointer
    Global,
    /// x4 | tp | Thread pointer
    Thread,
    /// x5 | t0 | Return address temporal
    TemporalReturn,
    /// x6–7 | t1–2 | Temporal
    /// x28–31 | t3–6 | Temporal
    Local(u8),
    /// x8 | s0 | fp
    Result,
    /// x9 | s1 | Saved register
    /// x18–27 | s2–11 | Saved registers
    Saved(u8),
    /// x10–17 | a0–7 | Argumentos de función
    Argument(u8),
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Unknown register")]
pub struct AnalyzeRegNotFound {
    #[label("expected a valid register")]
    pub location: Span,
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Not a register")]
pub struct AnalyzeRegInvalid {
    #[label("expected a register")]
    pub location: Span,
}

impl Register {
    pub fn from_expr(value: &Expr, summary: &mut AnalyzeSummary) -> Option<Self> {
        macro_rules! register {
            ($value:expr => {
                $($($pattern:literal)|* => $static:expr,)*

                $(Self::$listed:ident ($prefix:literal) => {
                    $($x:literal => $integer:literal,)*
                })*
            }) => {
                match $value {
                    $($($pattern)|* => Some($static),)*

                    $($(
                    concat!("x", $x) | concat!($prefix, $integer) => Some(Self::$listed($integer)),
                    )*)*

                    _ => {
                        summary.error(AnalyzeRegNotFound {
                            location: value.span(),
                        });
                        None
                    },

                }
            };
        }

        match value {
            Expr::Ident(i) => {
                register!(&*i.value => {
                    "x0" | "zero" => Self::Zero,
                    "x1" | "ra" => Self::Return,
                    "x2" | "sp" => Self::Stack,
                    "x3" | "gp" => Self::Global,
                    "x4" | "tp" => Self::Thread,
                    "x5" | "t0" => Self::TemporalReturn,
                    "x8" | "s0" | "fp" => Self::Result,

                    Self::Local("t") => {
                        6 => 0,
                        7 => 1,
                        28 => 2,
                        29 => 4,
                        30 => 5,
                        31 => 6,
                    }

                    Self::Saved("s") => {
                        9 => 0,
                        18 => 1,
                        19 => 2,
                        20 => 4,
                        21 => 5,
                        22 => 6,
                        23 => 7,
                        24 => 8,
                        25 => 9,
                        26 => 10,
                        27 => 11,
                    }

                    Self::Argument("a") => {
                        10 => 0,
                        11 => 1,
                        12 => 2,
                        13 => 3,
                        14 => 4,
                        15 => 5,
                        16 => 6,
                        17 => 7,
                    }
                })
            }
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
        if f.alternate() {
            match self {
                Self::Zero => f.write_str("\x1b[1;38;5;220mx0"),
                Self::Return => f.write_str("\x1b[1;38;5;111mra"),
                Self::Stack => f.write_str("\x1b[1;38;5;112msp"),
                Self::Global => f.write_str("\x1b[1;38;5;81mgp"),
                Self::Thread => f.write_str("\x1b[1;38;5;43mtp"),
                Self::TemporalReturn => f.write_str("\x1b[1;38;5;217mt0"),
                Self::Local(n) => f.write_fmt(format_args!("\x1b[1;38;5;218mt{}", n + 1)),
                Self::Result => f.write_str("\x1b[1;38;5;223ms0"),
                Self::Saved(n) => f.write_fmt(format_args!("\x1b[1;38;5;224ms{}", n + 1)),
                Self::Argument(n) => f.write_fmt(format_args!("\x1b[1;38;5;214ma{n}")),
            }?;

            f.write_str("\x1b[0m")
        } else {
            match self {
                Self::Zero => f.write_str("x0"),
                Self::Return => f.write_str("ra"),
                Self::Stack => f.write_str("sp"),
                Self::Global => f.write_str("gp"),
                Self::Thread => f.write_str("tp"),
                Self::TemporalReturn => f.write_str("t0"),
                Self::Local(n) => f.write_fmt(format_args!("t{}", n + 1)),
                Self::Result => f.write_str("s0"),
                Self::Saved(n) => f.write_fmt(format_args!("s{}", n + 1)),
                Self::Argument(n) => f.write_fmt(format_args!("a{n}")),
            }
        }
    }
}
