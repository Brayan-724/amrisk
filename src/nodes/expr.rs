use std::fmt;

use crate::nodes::{Block, Ident};
use crate::parser::{IntoSpanned, PrettyFormatter, PrettyPrint, Span, Spanned};

#[derive(Debug, Clone)]
pub enum Expr {
    Assign(Spanned<Ident>, Box<Expr>),
    Ident(Spanned<Ident>),
    Block(Block),
    Binary {
        kind: ExprBinary,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call(Box<Expr>, Vec<Expr>),
    Label(Spanned<Ident>),
    Number(Spanned<i32>),
}

impl IntoSpanned for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Assign(name, value) => name.span().merge(value.span()),
            Expr::Ident(v) => v.span,
            Expr::Block(v) => v.span(),
            Expr::Binary { lhs, rhs, .. } => lhs.span().merge(rhs.span()),
            Expr::Call(callee, args) => callee.span().merge(args.span()),
            Expr::Label(v) => v.span,
            Expr::Number(v) => v.span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExprBinary {
    Add,
    Sub,

    // Cond
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

impl PrettyPrint for Expr {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        match self {
            Expr::Assign(name, value) => f
                .node("Expr::Assign", name.span)?
                .begin_fields()
                .field("name", name)?
                .end_fields()
                .child(value.as_ref())?
                .finish(),
            Expr::Ident(v) => f.node("Expr::Ident", v.span)?.child(&v.value)?.finish(),
            Expr::Block(v) => f
                .node("Expr::Block", v.span())?
                .children(&v.stmts)?
                .finish(),
            Expr::Binary { kind, lhs, rhs } => f
                .node("Expr::Binary", lhs.span())?
                .begin_fields()
                .field(
                    "kind",
                    &match kind {
                        ExprBinary::Add => "Add",
                        ExprBinary::Sub => "Sub",
                        ExprBinary::Eq => "Eq",
                        ExprBinary::Ne => "Ne",
                        ExprBinary::Gt => "Gt",
                        ExprBinary::Ge => "Ge",
                        ExprBinary::Lt => "Lt",
                        ExprBinary::Le => "Le",
                    },
                )?
                .field_child("lhs", lhs.as_ref())?
                .field_child("rhs", rhs.as_ref())?
                .finish(),
            Expr::Call(callee, args) => f
                .node("Expr::Call", callee.span())?
                .begin_fields()
                .field_child("callee", callee.as_ref())?
                .field("args", args)?
                .end_fields()
                .finish(),
            Expr::Label(v) => f.node("Expr::Label", v.span)?.child(&v.value)?.finish(),
            Expr::Number(v) => f
                .node("Expr::Number", v.span)?
                .child(&format_args!("{}\n", v.value))?
                .finish(),
        }
    }
}
