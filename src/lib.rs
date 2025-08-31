#![cfg_attr(not(feature = "test"), warn(missing_docs))]
//! A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.

pub mod args;
pub mod entry;
pub mod error;
pub mod manifest;
pub mod preprocessor;
mod preprocessors;
pub mod query;
mod utils;
pub mod world;

// re-export the actual preprocessors from the top level
pub use preprocessors::*;

#[cfg(feature = "test")]
pub use test_utils::*;

#[cfg(feature = "test")]
mod test_utils {
    use std::error::Error;
    use std::fmt::Display;

    /// Never type, see https://github.com/rust-lang/rust/issues/35121
    #[derive(Debug)]
    pub enum Never {}

    impl Display for Never {
        fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match *self {}
        }
    }

    impl Error for Never {}
}
