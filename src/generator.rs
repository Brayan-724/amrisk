mod instructions;
mod linked_vector;
mod registers;

use std::fmt::{self, Write};

use miette::Diagnostic;
use thiserror::Error;

pub use instructions::*;
pub use registers::*;

use crate::nodes::Ident;
use crate::parser::Span;

#[derive(Debug, Error, Diagnostic)]
#[error("Instruction does not exist")]
pub struct AnalyzeInsNotExist {
    #[label]
    location: Span,
}

#[derive(Default, Debug)]
pub struct GenerateCtx {
    vars: Vec<(Ident, usize)>,
}

#[derive(Debug)]
pub struct GenerateBuf {
    buf: Vec<Instruction>,
    ctx: GenerateCtx,
    labels: Vec<(Box<str>, usize)>,
    stack: Vec<(Box<str>, usize)>,
}

impl GenerateBuf {
    pub fn new(ctx: GenerateCtx) -> Self {
        Self {
            buf: Vec::new(),
            ctx,
            labels: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub fn pointer(&self) -> usize {
        self.buf.len()
    }

    pub fn push(&mut self, ins: Instruction) {
        self.buf.push(ins);
    }

    pub fn stack_size(&self) -> usize {
        self.stack.iter().fold(0, |offset, c| offset + c.1)
    }

    pub fn push_stack(&mut self, name: impl Into<Box<str>>, size: usize) -> usize {
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
}

impl fmt::Display for GenerateBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut labels = self.labels.iter();
        let mut next_label = labels.next();

        for (idx, ins) in self.buf.iter().enumerate() {
            if let Some(label) = next_label
                && label.1 == idx
            {
                f.write_str(&*label.0)?;
                f.write_str(":\n")?;
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

    fn generated(&self, ctx: GenerateCtx) -> GenerateBuf {
        let mut buf = GenerateBuf::new(ctx);

        self.generate(&mut buf);

        buf
    }
}

impl<T: Generate> Generate for Vec<T> {
    fn generate(&self, buf: &mut GenerateBuf) {
        for i in self {
            i.generate(buf);
        }
    }
}
