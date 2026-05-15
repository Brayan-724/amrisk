use std::cell::RefCell;
use std::fmt;

use amrisk_macros::Node;
use miette::Diagnostic;
use thiserror::Error;

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::generator::{Generate, GenerateBuf, Imm, Instruction, Offset, Register};
use crate::nodes::{Block, Ident, ItemFunction};
use crate::parser::{IntoSpanned, Span, Spanned};
use crate::pretty::{PrettyFormatter, PrettyPrint};
use crate::shared_store::{SharedStore, StoreContainer};

use super::StmtLet;

#[derive(Debug, Clone, Node)]
#[node()]
pub enum Expr {
    Assign(Box<RefCell<Expr>>, Box<RefCell<Expr>>),
    Block(Block),
    #[spanned(lhs, rhs)]
    Binary {
        kind: ExprBinary,
        lhs: Box<RefCell<Expr>>,
        rhs: Box<RefCell<Expr>>,
        swap_load: bool,
    },
    Call(Box<RefCell<Expr>>, Vec<Expr>),
    Deref(Spanned<ExprDeref>, Box<RefCell<Expr>>),
    Ident(Spanned<Ident>),
    Label(Spanned<Ident>),
    Number(Spanned<i32>),
}

impl IntoSpanned for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Assign(name, value) => name.span().merge(value.span()),
            Expr::Block(v) => v.span(),
            Expr::Binary { lhs, rhs, .. } => lhs.span().merge(rhs.span()),
            Expr::Call(callee, args) => callee.span().merge(args.span()),
            Expr::Deref(deref, v) => deref.span.merge(v.span()),
            Expr::Ident(v) => v.span,
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

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ExprDeref {
    Byte = 1,
    Half = 2,
    Word = 4,
}

impl PrettyPrint for Expr {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        match self {
            Expr::Assign(name, value) => f
                .node("Expr::Assign", name.span())?
                .field_child("name", name.as_ref())?
                .end_fields()
                .child(value.as_ref())?
                .finish(),
            Expr::Block(v) => f
                .node("Expr::Block", v.span())?
                .children(&v.stmts)?
                .finish(),
            Expr::Binary { kind, lhs, rhs, .. } => f
                .node("Expr::Binary", lhs.span())?
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
                .field_child("callee", callee.as_ref())?
                .field("args", args)?
                .finish(),
            Expr::Deref(deref, expr) => f
                .node("Expr::Deref", expr.span())?
                .field(
                    "kind",
                    &match &**deref {
                        ExprDeref::Byte => "Byte",
                        ExprDeref::Half => "Half",
                        ExprDeref::Word => "Word",
                    },
                )?
                .child(expr.as_ref())?
                .finish(),
            Expr::Ident(v) => f.node("Expr::Ident", v.span)?.child(&v.value)?.finish(),
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

#[derive(Default)]
pub struct ExprAnalyzeStore {
    binary_cost: usize,
}

impl SharedStore<AnalyzeSummary> for Expr {
    type Store = ExprAnalyzeStore;
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Expr is not assignable")]
#[diagnostic(help("try deref it: *b, *h, *w"))]
pub struct AnalyzeExprNotAssignable {
    #[label]
    pub location: Span,
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Callee should be a function name")]
pub struct AnalyzeCalleeIsNotIdent {
    #[label]
    pub location: Span,
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Variable does not exist")]
pub struct AnalyzeVariableNotExistsError {
    #[label]
    pub location: Span,
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Function does not exist")]
pub struct AnalyzeFunctionNotExistsError {
    #[label]
    pub location: Span,
}

pub struct AnalyzeFunctionNotExistsMarker;

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Function mismatch arguments count, expected {expected}")]
pub struct AnalyzeFunctionMismatchArgsCountError {
    #[label(primary, "Provided {provided}...")]
    pub location: Span,
    pub provided: usize,

    #[label("...but expected {expected}")]
    pub original: Span,

    pub expected: usize,
}

impl Analyze for Expr {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        match self {
            Expr::Assign(var, expr) => {
                var.analyze(summary)?;

                match &*var.borrow() {
                    Self::Deref(..) | Self::Ident(..) => {}
                    _ => {
                        summary.error(AnalyzeExprNotAssignable {
                            location: var.span(),
                        });
                    }
                }

                expr.analyze(summary)?;
            }
            Expr::Block(block) => {
                block.analyze(summary)?;
            }
            Expr::Binary {
                kind,
                lhs,
                rhs,
                swap_load,
            } => {
                let base_cost = Self::store(summary).binary_cost;

                lhs.analyze(summary)?;
                let lhs_cost = Self::store(summary).binary_cost;
                Self::store(summary).binary_cost = base_cost;

                rhs.analyze(summary)?;
                let rhs_cost = Self::store(summary).binary_cost;
                Self::store(summary).binary_cost = base_cost;

                let next_expr = match (&*lhs.borrow(), &*rhs.borrow()) {
                    (Expr::Number(a), Expr::Number(b)) => {
                        let (a, b) = (a.clone(), b.clone());
                        let span = a.span.merge(b.span);

                        let result = match kind {
                            ExprBinary::Add => a.value + b.value,
                            ExprBinary::Sub => a.value - b.value,
                            _ => return AnalyzeResult::Continue(()),
                        };

                        Some(Expr::Number(span.of(result)))
                    }
                    _ => {
                        Self::store(summary).binary_cost += 1;

                        if rhs_cost > lhs_cost {
                            *swap_load = true;
                        }

                        None
                    }
                };

                if let Some(next_expr) = next_expr {
                    *self = next_expr;
                }
            }
            Expr::Call(callee, exprs) => {
                let Expr::Ident(i) = &*callee.borrow() else {
                    summary.error(AnalyzeCalleeIsNotIdent {
                        location: callee.span(),
                    });
                    return AnalyzeResult::Continue(());
                };

                let Some(func) = summary.shared_store::<ItemFunction>().functions.get(i) else {
                    summary.error_marked::<AnalyzeFunctionNotExistsMarker>(
                        AnalyzeFunctionNotExistsError { location: i.span() },
                    );
                    return AnalyzeResult::Continue(());
                };

                let args = &func.args;

                if args.args.len() != exprs.len() {
                    let original = args.span();
                    let expected = args.args.len();

                    summary.error(AnalyzeFunctionMismatchArgsCountError {
                        location: exprs.span(),
                        provided: exprs.len(),
                        original,
                        expected,
                    });
                }

                exprs.analyze(summary)?;
            }
            Expr::Deref(_, expr) => {
                expr.analyze(summary)?;
            }
            Expr::Ident(var) => {
                if !summary.store::<StmtLet>().local_vars.contains_key(&var.0) {
                    summary.error(AnalyzeVariableNotExistsError {
                        location: var.span(),
                    });
                }
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
                let (size, res, offset) = match &*var.borrow() {
                    Expr::Ident(var) => {
                        let (offset, size) = buf
                            .get_stack(var.0.as_ref())
                            .expect("Analyzer must ensure variable exists");

                        expr.borrow().generate(buf);

                        (size, buf.result, Offset::Imm(offset as i32))
                    }
                    Expr::Deref(size, value) => {
                        value.borrow().generate(buf);

                        let reg = buf.result;

                        buf.next_result();

                        expr.borrow().generate(buf);

                        let res = buf.prev_result();
                        let offset = Offset::Relative(0, reg);

                        (size.value, res, offset)
                    }
                    _ => {
                        unreachable!("Analyzer must ensure variants")
                    }
                };

                let ins = match size {
                    ExprDeref::Byte => Instruction::Sb(res, offset, Register::Stack),
                    ExprDeref::Half => Instruction::Sh(res, offset, Register::Stack),
                    ExprDeref::Word => Instruction::Sw(res, offset, Register::Stack),
                };

                buf.push(ins);
            }
            Expr::Block(_block) => todo!("block expression"),
            Expr::Binary {
                kind,
                lhs,
                rhs,
                swap_load,
            } => match (kind, &*lhs.borrow(), &*rhs.borrow()) {
                (ExprBinary::Add, expr, Expr::Number(n))
                | (ExprBinary::Add, Expr::Number(n), expr)
                | (ExprBinary::Sub, expr, Expr::Number(n)) => {
                    let value = if matches!(kind, ExprBinary::Sub) {
                        -n.value
                    } else {
                        n.value
                    };

                    expr.generate(buf);

                    buf.push(Instruction::Addi(buf.result, buf.result, Imm(value)));
                }
                (ExprBinary::Add, var @ Expr::Ident(..), expr)
                | (ExprBinary::Add, expr, var @ Expr::Ident(..)) => {
                    let base_res = buf.result;
                    let (a_res, b_res) = generate_binary(buf, var, expr, *swap_load);

                    buf.push(Instruction::Add(base_res, a_res, b_res));
                }
                (ExprBinary::Sub, a, b) => {
                    let base_res = buf.result;
                    let (a_res, b_res) = generate_binary(buf, a, b, *swap_load);

                    buf.push(Instruction::Sub(base_res, a_res, b_res));
                }
                _ => todo!(),
            },
            Expr::Call(expr, exprs) => {
                let Expr::Ident(name) = &*expr.borrow() else {
                    unreachable!("[analyzer] Call expr filters ident callee")
                };
                let name = &**name;

                let Some(func) = buf.shared_store::<ItemFunction>().functions.get(name) else {
                    unreachable!("[analyzer] Call expr ensures callee exists")
                };

                let func = func.name.0.clone();

                for (idx, expr) in exprs.iter().enumerate() {
                    buf.result = Register::Argument(idx as u8);
                    expr.generate(buf);
                }
                buf.result = Register::Result;

                buf.push(Instruction::Call(Offset::Label(func)));
            }
            Expr::Deref(size, expr) => {
                expr.borrow().generate(buf);

                let offset = Offset::Relative(0, buf.result);
                let inst = match size.value {
                    ExprDeref::Byte => Instruction::Lb(buf.result, offset, Register::Stack),
                    ExprDeref::Half => Instruction::Lh(buf.result, offset, Register::Stack),
                    ExprDeref::Word => Instruction::Lw(buf.result, offset, Register::Stack),
                };

                buf.push(inst);
            }
            Expr::Ident(var) => {
                let (offset, size) = buf
                    .get_stack(var.0.as_ref())
                    .expect("Analyzer must ensure variable exists");

                let offset = Offset::Imm(offset as i32);
                let inst = match size {
                    ExprDeref::Byte => Instruction::Lb(buf.result, offset, Register::Stack),
                    ExprDeref::Half => Instruction::Lh(buf.result, offset, Register::Stack),
                    ExprDeref::Word => Instruction::Lw(buf.result, offset, Register::Stack),
                };

                buf.push(inst);
            }
            Expr::Label(l) => {
                buf.label_here(&*l.value);
            }
            Expr::Number(n) => {
                buf.push(Instruction::Li(buf.result, Imm(n.value)));
            }
        }
    }
}

pub fn generate_binary(
    buf: &mut GenerateBuf,
    lhs: &Expr,
    rhs: &Expr,
    swap_load: bool,
) -> (Register, Register) {
    let base_res = buf.result;

    if swap_load {
        rhs.generate(buf);

        buf.next_result();
        lhs.generate(buf);

        (buf.prev_result(), base_res)
    } else {
        lhs.generate(buf);

        buf.next_result();
        rhs.generate(buf);

        (base_res, buf.prev_result())
    }
}
