//! Interfaces for reporting progress and errors through the CLI

use std::error::Error;
use std::fmt;

pub trait ErrorExt {
    fn error_chain(&self) -> ErrorChain<&Self> {
        ErrorChain(self)
    }
}

impl<T: Error> ErrorExt for T {}

pub trait WriteExt {
    fn indents<F, H>(&mut self, first: F, hanging: H) -> IndentWriter<'_, F, H, Self> {
        IndentWriter {
            first: Some(first),
            hanging,
            f: self,
        }
    }

    // fn indent<I: Clone>(&mut self, indent: I) -> IndentWriter<'_, I, I, Self> {
    //     self.indents(indent.clone(), indent)
    // }

    // fn first_line_indent<I>(&mut self, indent: I) -> IndentWriter<'_, I, &'static str, Self> {
    //     self.indents(indent, "")
    // }

    fn hanging_indent<I>(&mut self, indent: I) -> IndentWriter<'_, &'static str, I, Self> {
        self.indents("", indent)
    }
}

impl<T: fmt::Write> WriteExt for T {}

pub struct ErrorChain<T>(T);

impl<T> fmt::Display for ErrorChain<T>
where
    T: Error,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)?;
        let mut error: Option<&dyn Error> = self.0.source();
        while let Some(e) = error {
            writeln!(f)?;
            write!(f, "{}", e)?;
            error = e.source();
        }
        Ok(())
    }
}

pub struct IndentWriter<'a, F, H, W: ?Sized> {
    first: Option<F>,
    hanging: H,
    f: &'a mut W,
}

impl<F, H, W> fmt::Write for IndentWriter<'_, F, H, W>
where
    F: fmt::Display,
    H: fmt::Display,
    W: fmt::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Some(first) = self.first.take() {
            write!(self.f, "{}", first)?;
        }
        let mut lines = s.split('\n');
        write!(self.f, "{}", lines.next().unwrap())?;
        for line in lines {
            write!(self.f, "\n{}{}", self.hanging, line)?;
        }
        Ok(())
    }
}
