use std::path::PathBuf;

use clap::Parser;

/// prequery-preprocess args
#[derive(Debug, Clone, Parser)]
pub struct CliArguments {
    /// Configures the project root (for absolute paths)
    #[clap(long, value_name = "EXE", default_value = "typst")]
    pub typst: PathBuf,

    /// Configures the project root (for absolute paths)
    #[clap(long = "root", value_name = "DIR", env = "TYPST_ROOT")]
    pub root: Option<PathBuf>,

    /// Path to input Typst file. `prequery-preprocess` will look for a `typst.toml` file in
    /// directories upwards from that file to determine queries.
    pub input: PathBuf,
}
