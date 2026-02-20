use std::cell::RefCell;
use std::fmt;

use amrisk_macros::Node;

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::generator::{Generate, GenerateBuf, Imm, Instruction, Offset, Register};
use crate::nodes::{Block, Ident};
use crate::parser::{IntoSpanned, Span, Spanned};
use crate::pretty::{PrettyFormatter, PrettyPrint};

#[derive(Debug, Clone, Node)]
#[node()]
pub enum Expr {
    Assign(Spanned<Ident>, Box<RefCell<Expr>>),
    Ident(Spanned<Ident>),
    Block(Block),
    #[spanned(lhs, rhs)]
    Binary {
        kind: ExprBinary,
        lhs: Box<RefCell<Expr>>,
        rhs: Box<RefCell<Expr>>,
    },
    Call(Box<RefCell<Expr>>, Vec<Expr>),
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

impl IntoSpanned for RefCell<Expr> {
    fn span(&self) -> Span {
        self.borrow().span()
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

impl PrettyPrint for RefCell<Expr> {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        self.borrow().pretty_print(f)
    }
}

impl Analyze for Expr {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        match self {
            Expr::Assign(_, expr) => {
                expr.analyze(summary)?;
            }
            Expr::Block(block) => {
                block.analyze(summary)?;
            }
            Expr::Binary { kind, lhs, rhs } => {
                lhs.analyze(summary)?;
                rhs.analyze(summary)?;

                let (a, b) = match (&*lhs.borrow(), &*rhs.borrow()) {
                    (Expr::Number(a), Expr::Number(b)) => (a.clone(), b.clone()),
                    _ => return AnalyzeResult::Continue(()),
                };

                let result = match kind {
                    ExprBinary::Add => a.value + b.value,
                    ExprBinary::Sub => a.value - b.value,
                    _ => return AnalyzeResult::Continue(()),
                };

                *self = Expr::Number(a.span.merge(b.span).of(result));
            }
            Expr::Call(expr, exprs) => {
                expr.analyze(summary)?;
                exprs.analyze(summary)?;
            }
            _ => {}
        }

        AnalyzeResult::Continue(())
    }
}

impl Analyze for RefCell<Expr> {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        self.borrow_mut().analyze(summary)
    }
}

impl Generate for Expr {
    fn generate(&self, buf: &mut GenerateBuf) {
        match self {
            Expr::Assign(var, expr) => {
                if let Some(offset) = buf.get_stack(var.0.as_ref()) {
                    expr.borrow().generate(buf);

                    buf.push(Instruction::Sw(
                        Register::Result,
                        Offset::Imm(-(offset as i32)),
                        Register::Stack,
                    ));
                }
            }
            Expr::Ident(spanned) => todo!(),
            Expr::Block(block) => todo!(),
            Expr::Binary { kind, lhs, rhs } => match (kind, &*lhs.borrow(), &*rhs.borrow()) {
                (ExprBinary::Add, expr, Expr::Number(n)) => {
                    expr.generate(buf);
                    buf.push(Instruction::Addi(
                        Register::Result,
                        Register::Result,
                        Imm(n.value),
                    ));
                }
                (ExprBinary::Add, Expr::Number(n), expr) => {
                    expr.generate(buf);
                    buf.push(Instruction::Addi(
                        Register::Result,
                        Register::Result,
                        Imm(-n.value),
                    ));
                }
                _ => {}
            },
            Expr::Call(expr, exprs) => todo!(),
            Expr::Label(l) => {
                buf.label_here(&*l.value);
            }
            Expr::Number(n) => {
                buf.push(Instruction::Li(Register::Result, Imm(n.value)));
            }
        }
    }
}
