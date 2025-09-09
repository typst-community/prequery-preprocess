//! Interfaces for reporting progress and errors through the CLI

use std::error::Error;
use std::fmt;

pub trait ErrorExt {
    fn error_chain(&self) -> ErrorChain<&Self> {
        ErrorChain(self)
    }
}

impl<T: Error> ErrorExt for T {}

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
