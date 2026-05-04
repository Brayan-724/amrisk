use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use amrisk_macros::Node;
use miette::Diagnostic;
use thiserror::Error;

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::generator::{Generate, GenerateBuf, Instruction, Offset, Register};
use crate::nodes::{Block, Expr, ExprBinary, Ident};
use crate::parser::{IntoSpanned, Span, Spanned, Token};
use crate::pretty::{PrettyFormatter, PrettyPrint};
use crate::shared_store::SharedStore;

use super::generate_binary;

macro_rules! match_cond {
    ($buf:expr, $cond:expr, $label:expr,
     $($kind:ident => $ins:ident),+ $(,)?
    ) => {
        match $cond {
            $(
            Expr::Binary {
                kind: ExprBinary::$kind,
                lhs,
                rhs,
                swap_load,
            } => {
                let (a, b) = generate_binary($buf, &*lhs.borrow(), &*rhs.borrow(), *swap_load);
                $buf.push(Instruction::$ins(a, b, Offset::Label($label)));
            }
            )*
            _ => unreachable!("Discarded by analyzer"),

        }
    };
}

#[derive(Debug, Clone, Node)]
#[node(analyzer, generator, pretty, spanned)]
pub enum Statement {
    Expr(StmtExpr),
    If(StmtIf),
    Label(StmtLabel),
    Let(StmtLet),
    Loop(StmtLoop),
    While(StmtWhile),
}

impl Statement {
    pub fn has_semi(&self) -> Option<bool> {
        match self {
            Statement::Expr(stmt_expr) => Some(stmt_expr.semi.is_some()),
            Statement::Let(stmt_let) => Some(stmt_let.semi.is_some()),
            _ => None,
        }
    }
}

////////////////////
//////////////////// EXPRESSION
////////////////////

#[derive(Debug, Clone)]
pub struct StmtExpr {
    pub expr: Expr,
    pub semi: Option<Spanned<Token>>,
}

impl IntoSpanned for StmtExpr {
    fn span(&self) -> Span {
        if let Some(semi) = self.semi.as_ref() {
            self.expr.span().merge(semi.span)
        } else {
            self.expr.span()
        }
    }
}

impl PrettyPrint for StmtExpr {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.node("Statment::Expr", self.span())?
            .field("has_semi", &self.semi.is_some())?
            .child(&self.expr)?
            .finish()
    }
}

impl Analyze for StmtExpr {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        self.expr.analyze(summary)
    }
}

impl Generate for StmtExpr {
    fn generate(&self, buf: &mut GenerateBuf) {
        if self.semi.is_none() {
            buf.result = Register::Result;
        }

        self.expr.generate(buf);
    }
}

////////////////////
//////////////////// If
////////////////////

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("{statement} must have comparation expression")]
pub struct AnalyzeExpectedCondition {
    #[label]
    location: Span,

    statement: &'static str,
}

impl AnalyzeExpectedCondition {
    pub fn analyze(cond: &Expr, statement: &'static str) -> Option<Self> {
        match cond {
            Expr::Binary {
                kind:
                    ExprBinary::Eq
                    | ExprBinary::Ne
                    | ExprBinary::Gt
                    | ExprBinary::Ge
                    | ExprBinary::Lt
                    | ExprBinary::Le,
                ..
            } => None,
            _ => Some(AnalyzeExpectedCondition {
                location: cond.span(),
                statement,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StmtIf {
    pub if_: Spanned<Token>,
    pub cond: Expr,
    pub body: Block,
    pub otherwise: Option<(Spanned<Token>, Block)>,
}

#[derive(Default)]
pub struct StmtIfStore {
    pub count: usize,
}

impl SharedStore<GenerateBuf> for StmtIf {
    type Store = StmtIfStore;
}

impl IntoSpanned for StmtIf {
    fn span(&self) -> Span {
        if let Some(otherwise) = &self.otherwise {
            self.if_.span.merge(otherwise.1.span())
        } else {
            self.if_.span.merge(self.body.span())
        }
    }
}

impl PrettyPrint for StmtIf {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        let mut node = f
            .node("Statment::If", self.span())?
            .field_child("cond", &self.cond)?
            .field_child("body", &self.body)?;

        if let Some((_, otherwise)) = &self.otherwise {
            node = node.field_child("otherwise", otherwise)?;
        }

        node.finish()
    }
}

impl Analyze for StmtIf {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        self.cond.analyze(summary)?;

        if let Some(err) = AnalyzeExpectedCondition::analyze(&self.cond, "If conditional") {
            summary.error(err);
        }

        self.body.analyze(summary)?;

        if let Some((_, otherwise)) = &mut self.otherwise {
            otherwise.analyze(summary)?;
        }

        AnalyzeResult::Continue(())
    }
}

impl Generate for StmtIf {
    fn generate(&self, buf: &mut GenerateBuf) {
        let id = Self::store(buf).count;
        Self::store(buf).count += 1;

        let label_end: Rc<str> = Rc::from(format!("if.end.{id}").as_str());
        let label_otherwise: Rc<str> = Rc::from(format!("if.otherwise.{id}").as_str());

        match_cond!(buf, &self.cond, label_otherwise.clone(),
            Eq => Bne,
            Ne => Beq,
            Gt => Ble,
            Ge => Blt,
            Lt => Bge,
            Le => Bgt,
        );

        self.body.generate(buf);

        if let Some(otherwise) = &self.otherwise {
            buf.push(Instruction::J(Offset::Label(label_end.clone())));

            buf.label_here(&*label_otherwise);
            otherwise.1.generate(buf);

            buf.label_here(&*label_end);
        } else {
            buf.label_here(&*label_otherwise);
        }
    }
}

////////////////////
//////////////////// LABEL
////////////////////

#[derive(Debug, Clone)]
pub struct StmtLabel {
    pub squot: Spanned<Token>,
    pub name: Spanned<Ident>,
    pub colon: Spanned<Token>,
}

impl IntoSpanned for StmtLabel {
    fn span(&self) -> Span {
        self.squot.span.merge(self.colon.span)
    }
}

impl PrettyPrint for StmtLabel {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.node("Statment::Label", self.span())?
            .field("name", &self.name)?
            .finish()
    }
}

impl Analyze for StmtLabel {
    fn analyze(&mut self, _: &mut AnalyzeSummary) -> AnalyzeResult {
        AnalyzeResult::Continue(())
    }
}

impl Generate for StmtLabel {
    fn generate(&self, buf: &mut GenerateBuf) {
        buf.label_here(&**self.name);
    }
}

////////////////////
//////////////////// LET
////////////////////

#[derive(Debug, Clone)]
pub struct StmtLet {
    pub let_: Spanned<Token>,
    pub name: Spanned<Ident>,
    pub expr: Expr,
    pub semi: Option<Spanned<Token>>,
}

#[derive(Default)]
pub struct StmtLetStore {
    pub local_vars: HashMap<Rc<str>, Span>,
}

impl IntoSpanned for StmtLet {
    fn span(&self) -> Span {
        self.let_.span.merge(
            self.semi
                .map(|s| s.span)
                .unwrap_or_else(|| self.expr.span()),
        )
    }
}

impl PrettyPrint for StmtLet {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.node("Statment::Let", self.span())?
            .field("name", &self.name)?
            .field("has_semi", &self.semi.is_some())?
            .child(&self.expr)?
            .finish()
    }
}

impl SharedStore<AnalyzeSummary> for StmtLet {
    type Store = StmtLetStore;
}

impl Analyze for StmtLet {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        Self::store(summary)
            .local_vars
            .insert(self.name.0.clone(), self.name.span());

        self.expr.analyze(summary)
    }
}

impl Generate for StmtLet {
    fn generate(&self, buf: &mut GenerateBuf) {
        self.expr.generate(buf);
        let offset = buf.push_stack(&**self.name, 4);
        buf.push(Instruction::Sw(
            buf.result,
            Offset::Imm(offset as i32),
            Register::Stack,
        ));
    }
}

////////////////////
//////////////////// LOOP
////////////////////

#[derive(Debug, Clone)]
pub struct StmtLoop {
    pub loop_: Spanned<Token>,
    pub body: Block,
}

impl IntoSpanned for StmtLoop {
    fn span(&self) -> Span {
        self.loop_.span.merge(self.body.span())
    }
}

impl PrettyPrint for StmtLoop {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.node("Statment::Loop", self.span())?
            .children(&self.body.stmts)?
            .finish()
    }
}

impl Analyze for StmtLoop {
    fn analyze(&mut self, _: &mut AnalyzeSummary) -> AnalyzeResult {
        AnalyzeResult::Continue(())
    }
}

impl Generate for StmtLoop {
    fn generate(&self, _buf: &mut GenerateBuf) {
        todo!("loop statement")
    }
}

////////////////////
//////////////////// While
////////////////////

#[derive(Debug, Clone)]
pub struct StmtWhile {
    pub while_: Spanned<Token>,
    pub cond: Expr,
    pub body: Block,
}

impl IntoSpanned for StmtWhile {
    fn span(&self) -> Span {
        self.while_.span.merge(self.body.span())
    }
}

impl PrettyPrint for StmtWhile {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.node("Statment::While", self.span())?
            .field_child("cond", &self.cond)?
            .child(&self.body)?
            .finish()
    }
}

impl Analyze for StmtWhile {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        self.cond.analyze(summary)?;

        if let Some(err) = AnalyzeExpectedCondition::analyze(&self.cond, "While loop") {
            summary.error(err);
        }

        AnalyzeResult::Continue(())
    }
}

impl Generate for StmtWhile {
    fn generate(&self, _buf: &mut GenerateBuf) {
        todo!("while statement")
    }
}
