use std::fmt;

use miette::Diagnostic;
use thiserror::Error;

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::nodes::{Block, Expr, ExprBinary, Ident};
use crate::parser::{IntoSpanned, PrettyFormatter, PrettyPrint, Span, Spanned, Token};

#[derive(Debug, Clone)]
pub enum Statement {
    Expr(StmtExpr),
    Label(StmtLabel),
    Let(StmtLet),
    Loop(StmtLoop),
    While(StmtWhile),
}

impl IntoSpanned for Statement {
    fn span(&self) -> Span {
        match self {
            Statement::Expr(s) => s.span(),
            Statement::Label(item) => item.span(),
            Statement::Let(s) => s.span(),
            Statement::Loop(item) => item.span(),
            Statement::While(item) => item.span(),
        }
    }
}

impl PrettyPrint for Statement {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        match self {
            Statement::Expr(s) => s.pretty_print(f),
            Statement::Label(s) => s.pretty_print(f),
            Statement::Let(s) => s.pretty_print(f),
            Statement::Loop(s) => s.pretty_print(f),
            Statement::While(s) => s.pretty_print(f),
        }
    }
}

impl Analyze for Statement {
    fn analyze(&self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        match self {
            Statement::Expr(s) => s.analyze(summary),
            Statement::Label(s) => s.analyze(summary),
            Statement::Let(s) => s.analyze(summary),
            Statement::Loop(s) => s.analyze(summary),
            Statement::While(s) => s.analyze(summary),
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
            .begin_fields()
            .field("has_semi", &self.semi.is_some())?
            .end_fields()
            .child(&self.expr)?
            .finish()
    }
}

impl Analyze for StmtExpr {
    fn analyze(&self, _: &mut AnalyzeSummary) -> AnalyzeResult {
        AnalyzeResult::Continue(())
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
            .begin_fields()
            .field("name", &self.name)?
            .finish()
    }
}

impl Analyze for StmtLabel {
    fn analyze(&self, _: &mut AnalyzeSummary) -> AnalyzeResult {
        AnalyzeResult::Continue(())
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
            .begin_fields()
            .field("name", &self.name)?
            .field("has_semi", &self.semi.is_some())?
            .end_fields()
            .child(&self.expr)?
            .finish()
    }
}

impl Analyze for StmtLet {
    fn analyze(&self, _: &mut AnalyzeSummary) -> AnalyzeResult {
        AnalyzeResult::Continue(())
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
    fn analyze(&self, _: &mut AnalyzeSummary) -> AnalyzeResult {
        AnalyzeResult::Continue(())
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
            .begin_fields()
            .field_child("cond", &self.cond)?
            .end_fields()
            .child(&self.body)?
            .finish()
    }
}

#[derive(Debug, Error, Diagnostic)]
#[error("While loops must have comparation expression")]
pub struct AnalyzeWhileConditionError {
    #[label]
    location: Span,
}

impl Analyze for StmtWhile {
    fn analyze(&self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
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
            Expr::Binary {
                ref lhs, ref rhs, ..
            } => summary.error(AnalyzeWhileConditionError {
                location: lhs.span().merge(rhs.span()),
            }),
            _ => {}
        }

        AnalyzeResult::Continue(())
    }
}
