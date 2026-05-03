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

#[derive(Debug, Clone, Node)]
#[node(analyzer, generator, pretty, spanned)]
pub enum Statement {
    Expr(StmtExpr),
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
        self.expr.generate(buf);
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
        let offset = buf.push_stack(&**self.name, 4);
        self.expr.generate(buf);
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

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("While loops must have comparation expression")]
pub struct AnalyzeWhileConditionError {
    #[label]
    location: Span,
}

impl Analyze for StmtWhile {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        self.cond.analyze(summary)?;

        match self.cond {
            Expr::Binary {
                kind:
                    ExprBinary::Eq
                    | ExprBinary::Ne
                    | ExprBinary::Gt
                    | ExprBinary::Ge
                    | ExprBinary::Lt
                    | ExprBinary::Le,
                ..
            } => {}
            _ => summary.error(AnalyzeWhileConditionError {
                location: self.cond.span(),
            }),
        }

        AnalyzeResult::Continue(())
    }
}

impl Generate for StmtWhile {
    fn generate(&self, _buf: &mut GenerateBuf) {
        todo!("while statement")
    }
}
