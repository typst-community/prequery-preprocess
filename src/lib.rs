#![warn(missing_docs)]
//! A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.

pub mod args;
pub mod error;
pub mod manifest;
pub mod preprocessor;
mod preprocessors;
pub mod query;

// re-export the actual preprocessors from the top level
pub use preprocessors::*;
