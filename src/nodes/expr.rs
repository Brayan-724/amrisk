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
    Assign(Spanned<Ident>, Box<RefCell<Expr>>),
    Ident(Spanned<Ident>),
    Block(Block),
    #[spanned(lhs, rhs)]
    Binary {
        kind: ExprBinary,
        lhs: Box<RefCell<Expr>>,
        rhs: Box<RefCell<Expr>>,
        swap_load: bool,
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
                .field("name", name)?
                .end_fields()
                .child(value.as_ref())?
                .finish(),
            Expr::Ident(v) => f.node("Expr::Ident", v.span)?.child(&v.value)?.finish(),
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
    in_callee: bool,
}

impl SharedStore<AnalyzeSummary> for Expr {
    type Store = ExprAnalyzeStore;
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
                if !summary.store::<StmtLet>().local_vars.contains_key(&var.0) {
                    summary.error(AnalyzeVariableNotExistsError {
                        location: var.span(),
                    });
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
                {
                    let Expr::Ident(i) = &*callee.borrow() else {
                        todo!("Expr is not ident")
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

                        return AnalyzeResult::Continue(());
                    }
                }

                let prev_in_callee = summary.store::<Expr>().in_callee;
                summary.store::<Expr>().in_callee = true;
                callee.analyze(summary)?;
                summary.store::<Expr>().in_callee = prev_in_callee;

                exprs.analyze(summary)?;
            }
            Expr::Ident(var) => {
                if !summary.store::<StmtLet>().local_vars.contains_key(&var.0)
                    && !summary.store::<Expr>().in_callee
                {
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
                let (offset, _) = buf
                    .get_stack(var.0.as_ref())
                    .expect("Analyzer must ensure variable exists");

                expr.borrow().generate(buf);

                buf.push(Instruction::Sw(
                    buf.result,
                    Offset::Imm(offset as i32),
                    Register::Stack,
                ));
            }
            Expr::Ident(var) => {
                let (offset, size) = buf
                    .get_stack(var.0.as_ref())
                    .expect("Analyzer must ensure variable exists");

                let offset = Offset::Imm(offset as i32);
                let inst = match size {
                    1 => Instruction::Lb(buf.result, offset, Register::Stack),
                    2 => Instruction::Lh(buf.result, offset, Register::Stack),
                    4 => Instruction::Lw(buf.result, offset, Register::Stack),
                    _ => todo!(),
                };

                buf.push(inst);
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
                    unreachable!("[analyzer] Call expr filters ident callee ")
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

                buf.push(Instruction::Call(Offset::Label(func)));
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

fn generate_binary(
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
