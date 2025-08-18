//! CLI argument parsing types

use std::path::PathBuf;

use clap::Parser;
use once_cell::sync::Lazy;

/// Map of preprocessors defined in this crate
pub static ARGS: Lazy<CliArguments> = Lazy::new(CliArguments::parse);

/// prequery-preprocess args
#[derive(Parser, Debug, Clone, PartialEq, Eq)]
pub struct CliArguments {
    /// Configures the types executable to use
    #[clap(long, value_name = "EXE", default_value = "typst")]
    pub typst: PathBuf,

    /// Configures the project root (for absolute paths)
    #[clap(long = "root", value_name = "DIR", env = "TYPST_ROOT")]
    pub root: Option<PathBuf>,

    /// Path to input Typst file. `prequery-preprocess` will look for a `typst.toml` file in
    /// directories upwards from that file to determine queries.
    pub input: PathBuf,
}
