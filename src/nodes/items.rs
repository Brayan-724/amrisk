use std::collections::HashMap;
use std::fmt::{self, Write as _};

use amrisk_macros::Node;
use miette::Diagnostic;
use thiserror::Error;

use crate::analysis::{Analyze, AnalyzeResult, AnalyzeSummary};
use crate::generator::{Generate, GenerateBuf, Imm, Instruction, Offset, Register};
use crate::nodes::{Block, Ident};
use crate::parser::{IntoSpanned, Span, Spanned, Token};
use crate::pretty::{PrettyFormatter, PrettyPrint};
use crate::shared_store::{SharedStore, StoreContainer};

use super::{AnalyzeFunctionNotExistsMarker, Attribute, AttributeBuiltin, statements};

/// Declarations
#[derive(Debug, Clone, Node)]
#[node(analyzer, generator, pretty, spanned)]
pub enum Item {
    Function(ItemFunction),
}

/// Declaration of a function
#[derive(Debug, Clone, Node)]
#[node()]
pub struct ItemFunction {
    pub attrs: Vec<Attribute>,
    pub unsafe_: Option<Spanned<Token>>,
    pub fn_: Spanned<Token>,
    pub name: Spanned<Ident>,
    pub args: FnArgs,
    pub ret: Option<Spanned<Ident>>,
    pub body: Block,
}

pub struct FunctionDefinition {
    pub name: Spanned<Ident>,
    pub args: FnArgs,
    pub ret: Option<Spanned<Ident>>,
}

#[derive(Default)]
pub struct FunctionSharedStore {
    pub entry_fn: Option<Span>,
    pub functions: HashMap<Ident, FunctionDefinition>,
}

impl SharedStore for ItemFunction {
    type Store = FunctionSharedStore;
}

impl IntoSpanned for ItemFunction {
    fn span(&self) -> Span {
        self.unsafe_
            .unwrap_or(self.fn_)
            .span
            .merge(self.body.span())
    }
}

impl PrettyPrint for ItemFunction {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.node("Item::Function", self.span())?
            .field("name", &self.name)?
            .field("args", &self.args)?
            .field("ret", &self.ret)?
            .field("attrs", &self.attrs)?
            .field("unsafe", &self.unsafe_.is_some())?
            .children(&self.body.stmts)?
            .finish()
    }
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Function already exists")]
pub struct AnalyzeFunctionExistsError {
    #[label(primary)]
    pub location: Span,

    #[label("Previous declaration")]
    pub original: Span,
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Only one entry function can exists")]
pub struct AnalyzeFunctionUniqueEntry {
    #[label(primary, "Second declaration")]
    pub location: Span,

    #[label("Previous declaration")]
    pub original: Span,
}

impl Analyze for ItemFunction {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        summary.clear_store::<statements::StmtLet>();

        if let Some(previous) = summary.shared_store::<Self>().functions.insert(
            (&*self.name).clone(),
            FunctionDefinition {
                name: self.name.clone(),
                args: self.args.clone(),
                ret: self.ret.clone(),
            },
        ) {
            if previous.name.span != self.name.span {
                summary.error(AnalyzeFunctionExistsError {
                    location: self.name.span(),
                    original: previous.name.span,
                });

                return AnalyzeResult::Break(false);
            }
        } else if summary.remove_marked::<AnalyzeFunctionNotExistsMarker>() != 0 {
            return AnalyzeResult::Break(true);
        }

        self.attrs.analyze(summary)?;

        for attr in self
            .attrs
            .iter()
            .map(AttributeBuiltin::try_from)
            .filter_map(Result::ok)
        {
            match attr {
                AttributeBuiltin::Entry => {
                    if let Some(prev) = summary.shared_store::<Self>().entry_fn
                        && prev != self.name.span
                    {
                        summary.error(AnalyzeFunctionUniqueEntry {
                            location: self.name.span,
                            original: prev,
                        });
                    } else {
                        summary.shared_store::<Self>().entry_fn = Some(self.name.span);
                    }
                }
            }
        }

        for arg in &self.args.args {
            summary
                .store::<statements::StmtLet>()
                .local_vars
                .entry(arg.name.0.clone())
                .insert_entry(arg.name.span());
        }

        self.body.analyze(summary)
    }
}

impl Generate for ItemFunction {
    fn generate(&self, buf: &mut GenerateBuf) {
        buf.label_here(&**self.name);

        buf.clear_store::<statements::StmtIf>();
        buf.clear_store::<statements::StmtWhile>();

        let mut buf_child = GenerateBuf::default();

        let args_stack = self
            .args
            .args
            .iter()
            .map(|arg| buf_child.push_stack(&*arg.name.0, 4))
            .collect::<Vec<_>>();

        let child = self.body.generated_child(buf, buf_child);
        let child_stack = child.stack_size() as i32;

        let needs_return_pointer = buf.shared_store::<Self>().entry_fn != Some(self.name.span);

        // Plus return pointer
        let stack_size = child_stack + if needs_return_pointer { 4 } else { 0 };

        // Reserve required stack
        buf.push(Instruction::Addi(
            Register::Stack,
            Register::Stack,
            Imm(-stack_size),
        ));

        if needs_return_pointer {
            // Save return pointer
            buf.push(Instruction::Sw(
                Register::Return,
                Offset::Imm(child_stack),
                Register::Stack,
            ));
        }

        for (idx, arg_stack) in args_stack.into_iter().enumerate() {
            buf.push(Instruction::Sw(
                Register::Argument(idx as u8),
                Offset::Imm(arg_stack as i32),
                Register::Stack,
            ));
        }

        buf.extend_on(child, format!(".{}.", self.name.value.0));

        if needs_return_pointer {
            // Load return pointer
            buf.push(Instruction::Lw(
                Register::Return,
                Offset::Imm(child_stack),
                Register::Stack,
            ));
        }

        // Free used stack
        buf.push(Instruction::Addi(
            Register::Stack,
            Register::Stack,
            Imm(stack_size),
        ));

        if needs_return_pointer {
            buf.push(Instruction::Ret());
        } else {
            buf.push(Instruction::Ebreak());
        }
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
            .field("name", &self.name)?
            .field("ty", &self.ty)?
            .finish()
    }
}
