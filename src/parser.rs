mod grammar;

use std::ops;

use miette::SourceSpan;

use peg::str::LineCol;

use crate::nodes::Program;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Error parsing input")]
pub struct ParseError {
    #[source_code]
    pub src: String,

    pub expected: String,

    #[label("{expected}")]
    pub location: miette::SourceSpan,
}

impl ParseError {
    pub fn into_report(self) -> miette::Report {
        miette::Report::new(self)
    }

    pub fn report(self) {
        eprintln!("{:?}", self.into_report())
    }
}

pub fn parse(input: &str) -> Result<Program, ParseError> {
    grammar::program::program(input).map_err(|err| {
        let mut expected = String::from("expected");

        for exp in err.expected.tokens() {
            expected.push(' ');
            expected += exp;
            expected.push(',');
        }

        expected.pop();

        ParseError {
            src: input.to_string(),
            expected,
            location: miette::SourceSpan::new(err.location.offset.into(), 0),
        }
    })
}

#[derive(Hash, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub len: usize,
}

impl Span {
    pub fn new(start: usize, len: usize) -> Span {
        Span { start, len }
    }

    pub fn merge(mut self, mut other: Span) -> Span {
        if other.end() > self.start {
            self.len = other.end() - self.start;
            self
        } else {
            other.len = self.end() - other.start;
            other
        }
    }

    pub fn of<T>(self, value: T) -> Spanned<T> {
        Spanned { span: self, value }
    }

    pub fn end(&self) -> usize {
        self.start + self.len
    }

    /// Create a padding from the end and set area as span.
    /// If size is larger than span it will overflows to right.
    pub fn end_span(&self, size: usize) -> Span {
        Span::new(self.start + self.len.saturating_sub(size), size)
    }

    pub fn location(&self, source: &str) -> LineCol {
        <str as peg::Parse>::position_repr(source, self.start)
    }
}

impl From<Span> for SourceSpan {
    fn from(value: Span) -> Self {
        SourceSpan::new(value.start.into(), value.len)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Spanned<T> {
    pub span: Span,
    pub value: T,
}

impl<T> ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub trait IntoSpanned: Sized {
    fn span(&self) -> Span;
}

impl<T> IntoSpanned for Spanned<T> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<T: IntoSpanned> IntoSpanned for Vec<T> {
    fn span(&self) -> Span {
        match self.as_slice() {
            [] => Span::default(),
            [x] => x.span(),
            [x, y] => x.span().merge(y.span()),
            [x, .., y] => x.span().merge(y.span()),
        }
    }
}

macro_rules! def_tokens {
    (
        Keywords {
            $($k_name:ident = $k_str:literal);* $(;)?
        }

        Punctuation {
            $($p_name:ident = $p_str:literal);* $(;)?
        }
    ) => {
        #[derive(Debug, Clone, Copy)]
        pub enum Token {
            $($k_name,)*
            $($p_name,)*
        }

        impl Token {
            pub fn is_reverved(ident: &str) -> bool {
                match ident {
                    $($k_str => true,)*
                    _ => false
                }
            }

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$k_name => $k_str,)*
                    $(Self::$p_name => $p_str,)*
                }
            }

            pub fn is_punctuation(self) -> bool {
                match self {
                    $(Self::$k_name => false,)*
                    $(Self::$p_name => true,)*
                }
            }
        }

    };
}

def_tokens! (
Keywords {
    Fn = "fn";
    Let = "let";
    Loop = "loop";
    Unsafe = "unsafe";
    While = "while";
}

Punctuation {
    SQuot = "'";
    Arrow = "->";
    BraceO = "{";
    BraceC = "}";
    Colon = ":";
    Equal = "=";
    ParenO = "(";
    ParenC = ")";
    Semi = ";";
}
);
