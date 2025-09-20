use std::fmt::{self, Write as _};

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::nodes::{Block, Ident};
use crate::parser::{IntoSpanned, PrettyFormatter, PrettyPrint, Span, Spanned, Token};

/// Declarations
#[derive(Debug, Clone)]
pub enum Item {
    Function(ItemFunction),
}

impl IntoSpanned for Item {
    fn span(&self) -> Span {
        match self {
            Item::Function(item) => item.span(),
        }
    }
}

impl PrettyPrint for Item {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        match self {
            Item::Function(item) => item.pretty_print(f),
            // _ => todo!(),
        }
    }
}

impl Analyze for Item {
    fn analyze(&self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        match self {
            Item::Function(item) => item.analyze(summary),
        }
    }
}

/// Declaration of a function
#[derive(Debug, Clone)]
pub struct ItemFunction {
    pub fn_: Spanned<Token>,
    pub name: Spanned<Ident>,
    pub args: FnArgs,
    pub ret: Option<Spanned<Ident>>,
    pub body: Block,
}

impl IntoSpanned for ItemFunction {
    fn span(&self) -> Span {
        self.fn_.span.merge(self.body.span())
    }
}

impl PrettyPrint for ItemFunction {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.node("Item::Function", self.span())?
            .begin_fields()
            .field("name", &self.name)?
            .field("args", &self.args)?
            .field("ret", &self.ret)?
            .end_fields()
            .children(&self.body.stmts)?
            .finish()
    }
}

impl Analyze for ItemFunction {
    fn analyze(&self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        self.body.analyze(summary)
    }
}

////////////////////
//////////////////// FUNCTION
////////////////////

/// Collection of function arguments
#[derive(Debug, Clone)]
pub struct FnArgs {
    pub open: Spanned<Token>,
    pub args: Vec<FnArg>,
    pub close: Spanned<Token>,
}

impl IntoSpanned for FnArgs {
    fn span(&self) -> Span {
        self.open.span.merge(self.close.span)
    }
}

impl PrettyPrint for FnArgs {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        if self.args.is_empty() {
            f.write_str("\x1b[2;31mEmpty\x1b[0m\n")
        } else {
            f.write_char('\n')?;
            f.write_children(&self.args)
        }
    }
}

/// Function argument declaration
#[derive(Debug, Clone)]
pub struct FnArg {
    pub name: Spanned<Ident>,
    pub ty: Spanned<Ident>,
}

impl IntoSpanned for FnArg {
    fn span(&self) -> Span {
        self.name.span.merge(self.ty.span)
    }
}

impl PrettyPrint for FnArg {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.node("FnArg", self.span())?
            .begin_fields()
            .field("name", &self.name)?
            .field("ty", &self.ty)?
            .finish()
    }
}
