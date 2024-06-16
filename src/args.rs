use std::path::PathBuf;

use clap::Parser;

/// prequery-preprocess args
#[derive(Debug, Clone, Parser)]
pub struct CliArguments {
    /// Configures the project root (for absolute paths)
    #[clap(long = "root", env = "TYPST_ROOT", value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Path to input Typst file. `prequery-preprocess` will look for a `typst.toml` file in
    /// directories upwards from that file to determine queries.
    pub input: PathBuf,
}
