mod instructions;
mod registers;

use std::fmt::{self, Write};
use std::mem;
use std::ops::ControlFlow;

use miette::Diagnostic;
use thiserror::Error;

pub use instructions::*;
pub use registers::*;

use crate::nodes::ExprDeref;
use crate::parser::Span;
use crate::shared_store::{StoreContainer, StoresContainer};

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
#[error("Instruction does not exist")]
pub struct AnalyzeInsNotExist {
    #[label]
    pub location: Span,
}

#[derive(Default, Debug)]
pub struct GenerateCtx {}

#[derive(Debug)]
pub struct GenerateBuf {
    buf: Vec<Instruction>,
    #[expect(unused)]
    ctx: GenerateCtx,
    labels: Vec<(Box<str>, usize)>,
    stack: Vec<(Box<str>, ExprDeref)>,
    pub result: Register,
    stores: StoresContainer,
}

impl Default for GenerateBuf {
    fn default() -> Self {
        Self {
            buf: Vec::new(),
            ctx: GenerateCtx::default(),
            labels: Vec::new(),
            stack: Vec::new(),
            result: Register::Local(0),
            stores: StoresContainer::default(),
        }
    }
}

impl StoreContainer for GenerateBuf {
    fn container(&mut self) -> &mut StoresContainer {
        &mut self.stores
    }
}

impl GenerateBuf {
    pub fn new(ctx: GenerateCtx, stores: StoresContainer) -> Self {
        Self {
            ctx,
            stores,
            ..Default::default()
        }
    }

    pub fn pointer(&self) -> usize {
        self.buf.len()
    }

    pub fn push(&mut self, ins: Instruction) {
        self.buf.push(ins);
    }

    pub fn stack_size(&self) -> usize {
        self.stack.iter().fold(0, |offset, c| offset + c.1 as usize)
    }

    pub fn push_stack(&mut self, name: impl Into<Box<str>>, size: ExprDeref) -> usize {
        let offset = self.stack_size();

        self.stack.push((name.into(), size));

        offset
    }

    pub fn label_here(&mut self, name: impl Into<Box<str>>) {
        self.labels.push((name.into(), self.pointer()))
    }

    pub fn extend_on(&mut self, rhs: Self, parent: String) {
        let current_pointer = self.pointer();

        self.labels.reserve(rhs.labels.len());

        for label in rhs.labels {
            self.labels.push((
                Box::from(parent.clone() + &label.0),
                label.1 + current_pointer,
            ))
        }

        self.buf.reserve(rhs.buf.len());

        for ins in rhs.buf {
            let ins = ins.scope(&parent);

            self.buf.push(ins);
        }
    }

    pub fn get_stack(&self, name: &str) -> Option<(usize, ExprDeref)> {
        self.stack
            .iter()
            .rev()
            .try_fold(0usize, |offset, (var, size)| {
                if &**var == name {
                    ControlFlow::Break((offset, *size))
                } else {
                    ControlFlow::Continue(offset + *size as usize)
                }
            })
            .break_value()
            .map(|(rev_offset, size)| (self.stack_size() - rev_offset - size as usize, size))
    }

    pub fn next_result_peek(&self) -> Register {
        match self.result {
            Register::Result => Register::Local(0),
            Register::Local(n @ 0..=5) => Register::Local(n + 1),
            Register::Local(6..) => todo!(),
            Register::Argument(..) => Register::Local(0),
            _ => unreachable!(),
        }
    }

    pub fn next_result(&mut self) -> Register {
        let next = self.next_result_peek();

        self.result = next;

        next
    }

    pub fn prev_result(&mut self) -> Register {
        let prev = self.result;
        let next = match self.result {
            Register::Result | Register::Local(0) => Register::Local(0),
            Register::Local(n @ 1..) => Register::Local(n - 1),
            _ => unreachable!(),
        };

        self.result = next;

        prev
    }
}

impl fmt::Display for GenerateBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut labels = self.labels.iter();
        let mut next_label = labels.next();

        for (idx, ins) in self.buf.iter().enumerate() {
            while let Some(label) = next_label
                && label.1 == idx
            {
                if f.alternate() {
                    f.write_str("\x1b[1;38;5;171m")?;
                }

                f.write_str(&*label.0)?;
                f.write_str(":\n")?;

                if f.alternate() {
                    f.write_str("\x1b[0m")?;
                }

                next_label = labels.next();
            }

            f.write_str("  ")?;
            fmt::Display::fmt(ins, f)?;
            f.write_char('\n')?;
        }

        Ok(())
    }
}

pub trait Generate {
    fn generate(&self, buf: &mut GenerateBuf);

    fn generated(&self, stores: StoresContainer) -> GenerateBuf {
        let mut buf = GenerateBuf::new(GenerateCtx::default(), stores);

        self.generate(&mut buf);

        buf
    }

    fn generated_child(&self, buf: &mut GenerateBuf, base: GenerateBuf) -> GenerateBuf {
        let mut child = base;

        mem::swap(&mut child.stores, &mut buf.stores);

        self.generate(&mut child);

        mem::swap(&mut child.stores, &mut buf.stores);

        child
    }
}

impl<T: Generate> Generate for Vec<T> {
    fn generate(&self, buf: &mut GenerateBuf) {
        for i in self {
            i.generate(buf);
        }
    }
}
