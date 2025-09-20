use std::fmt::Write as _;
use std::{fmt, ops};

use peg::str::LineCol;

use super::*;

pub struct PrettyPrinted<'a, 's, T: PrettyPrint>(&'a T, &'s str);

impl<T: PrettyPrint> fmt::Display for PrettyPrinted<'_, '_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.pretty_print(&mut PrettyFormatter {
            buf: f,
            level: 0,
            source: self.1,
        })
    }
}

macro_rules! header {
    ($code:literal) => {
        (
            concat!("5;", $code),
            concat!("\x1b[38;5;", $code, "m🭨\x1b[0;48;5;", $code, "m"),
            concat!("\x1b[0;38;5;", $code, "m🭪\x1b[0m "),
        )
    };
}

const HEADER: [(&'static str, &'static str, &'static str); 12] = [
    header!("129"),
    header!("128"),
    header!("127"),
    header!("126"),
    header!("125"),
    header!("124"),
    header!("88"),
    header!("89"),
    header!("90"),
    header!("91"),
    header!("92"),
    header!("93"),
];

const fn get_header(level: usize) -> (&'static str, &'static str, &'static str) {
    HEADER[level % HEADER.len()]
}

const fn get_header_color(level: usize) -> &'static str {
    get_header(level).0
}

const fn get_header_delim(level: usize) -> (&'static str, &'static str) {
    let h = get_header(level);
    (h.1, h.2)
}

pub struct PrettyFormatter<'s, 'a, 'f: 'a> {
    buf: &'a mut fmt::Formatter<'f>,
    source: &'s str,
    pub level: usize,
}

impl<'s, 'a, 'f: 'a> PrettyFormatter<'s, 'a, 'f> {
    #[inline(always)]
    pub fn location(&mut self, span: Span) -> LineCol {
        span.location(self.source)
    }

    #[inline(always)]
    pub fn node(
        &mut self,
        header: &'static str,
        span: Span,
    ) -> Result<PrettyNode<'_, 's, 'a, 'f>, fmt::Error> {
        self.write_header(header, span)?;
        Ok(PrettyNode {
            buf: self,
            levels: 0,
        })
    }

    #[inline(always)]
    pub fn write_header(&mut self, text: &'static str, span: Span) -> fmt::Result {
        let (start, end) = get_header_delim(self.level);
        self.write_str(start)?;
        self.write_str(text)?;
        self.write_str(end)?;
        self.location(span).pretty_print(self)?;
        self.write_char('\n')
    }

    #[inline(always)]
    pub fn write_indent_empty(&mut self) -> fmt::Result {
        for l in 0..self.level {
            let color = get_header_color(l);

            self.write_str("\x1b[38;")?;
            self.write_str(color)?;
            self.write_str("m🮈  ")?;
        }

        self.write_str("\x1b[0m")?;

        Ok(())
    }

    #[inline(always)]
    pub fn write_indent(&mut self) -> fmt::Result {
        if self.level > 1 {
            for l in 0..self.level - 1 {
                let color = get_header_color(l);

                self.write_str("\x1b[38;")?;
                self.write_str(color)?;
                self.write_str("m🮈  ")?;
            }
        }

        let color = get_header_color(self.level - 1);

        self.write_str("\x1b[38;")?;
        self.write_str(color)?;
        self.write_str("m🮈🭹 ")?;
        self.write_str("\x1b[0m")
    }

    #[inline(always)]
    pub fn write_children<T: PrettyPrint>(&mut self, children: &[T]) -> fmt::Result {
        self.level += 1;

        let last = children.len() - 1;
        for (idx, child) in children.into_iter().enumerate() {
            self.write_indent()?;
            child.pretty_print(self)?;

            if idx != last {
                self.write_indent_empty()?;
                self.write_char('\n')?;
            }
        }

        self.level -= 1;

        Ok(())
    }
}

impl fmt::Write for PrettyFormatter<'_, '_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.write_str(s)
    }

    fn write_char(&mut self, s: char) -> fmt::Result {
        self.buf.write_char(s)
    }

    fn write_fmt(&mut self, s: fmt::Arguments) -> fmt::Result {
        self.buf.write_fmt(s)
    }
}

pub struct PrettyNode<'p, 's: 'p, 'a: 'p, 'f: 'a> {
    buf: &'p mut PrettyFormatter<'s, 'a, 'f>,
    levels: usize,
}

impl PrettyNode<'_, '_, '_, '_> {
    #[inline(always)]
    pub fn begin_fields(mut self) -> Self {
        self.buf.level += 1;
        self.levels += 1;
        self
    }

    #[inline(always)]
    pub fn write_field_key(mut self, name: &'static str) -> Result<Self, fmt::Error> {
        self.buf.write_indent()?;
        self.write_str(name)?;
        self.write_str(": ")?;
        Ok(self)
    }

    #[inline(always)]
    pub fn field(
        mut self,
        name: &'static str,
        value: &impl PrettyPrint,
    ) -> Result<Self, fmt::Error> {
        self = self.write_field_key(name)?;
        value.pretty_print(&mut self)?;
        Ok(self)
    }
    #[inline(always)]
    pub fn field_child(
        mut self,
        name: &'static str,
        value: &impl PrettyPrint,
    ) -> Result<Self, fmt::Error> {
        self = self.write_field_key(name)?;
        self.write_str("\n")?;
        self.write_indent()?;
        value.pretty_print(&mut self)?;
        Ok(self)
    }

    #[inline(always)]
    pub fn end_fields(mut self) -> Self {
        self.buf.level -= 1;
        self.levels -= 1;
        self
    }

    #[inline(always)]
    pub fn children(mut self, children: &[impl PrettyPrint]) -> Result<Self, fmt::Error> {
        self.write_children(children)?;
        Ok(self)
    }

    #[inline(always)]
    pub fn child(mut self, child: &impl PrettyPrint) -> Result<Self, fmt::Error> {
        self.buf.level += 1;
        self.levels += 1;

        self.write_indent()?;
        child.pretty_print(&mut self)?;

        self.buf.level -= 1;
        self.levels -= 1;

        Ok(self)
    }

    #[inline(always)]
    pub fn finish(self) -> fmt::Result {
        self.buf.level -= self.levels;
        Ok(())
    }
}

impl fmt::Write for PrettyNode<'_, '_, '_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.write_str(s)
    }

    fn write_char(&mut self, s: char) -> fmt::Result {
        self.buf.write_char(s)
    }

    fn write_fmt(&mut self, s: fmt::Arguments) -> fmt::Result {
        self.buf.write_fmt(s)
    }
}

impl<'s, 'a, 'f: 'a> ops::Deref for PrettyNode<'_, 's, 'a, 'f> {
    type Target = PrettyFormatter<'s, 'a, 'f>;

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

impl ops::DerefMut for PrettyNode<'_, '_, '_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buf
    }
}

pub trait PrettyPrint: Sized {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result;

    fn pretty_printed<'a, 's: 'a>(&'a self, source: &'s str) -> PrettyPrinted<'a, 's, Self> {
        PrettyPrinted(&self, source)
    }
}

impl PrettyPrint for &str {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.write_str(self)?;
        f.write_char('\n')
    }
}

impl PrettyPrint for bool {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        match *self {
            true => f.write_str("\x1b[1;32mTrue\x1b[0m\n"),
            false => f.write_str("\x1b[1;31mFalse\x1b[0m\n"),
        }
    }
}

impl<T: PrettyPrint> PrettyPrint for Vec<T> {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        if self.is_empty() {
            return Ok(());
        }

        f.write_char('\n')?;
        let last = self.len() - 1;
        for (idx, child) in self.into_iter().enumerate() {
            f.write_indent()?;
            child.pretty_print(f)?;

            if idx != last {
                f.write_indent_empty()?;
                f.write_char('\n')?;
            }
        }

        Ok(())
    }
}

impl PrettyPrint for fmt::Arguments<'_> {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.write_fmt(*self)
    }
}

impl<T: PrettyPrint> PrettyPrint for Spanned<T> {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.location(self.span).pretty_print(f)?;
        f.write_char(' ')?;
        self.value.pretty_print(f)
    }
}

impl<T: PrettyPrint> PrettyPrint for Option<T> {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        match self {
            Some(v) => v.pretty_print(f),
            None => f.write_str("\x1b[2;31mNone\x1b[0m\n"),
        }
    }
}

impl PrettyPrint for LineCol {
    fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "\x1b[38;5;48m[{}:{}]\x1b[0m",
            self.line, self.column
        ))
    }
}
