//! CLI argument parsing types

use std::path::PathBuf;

use clap::Parser;

/// A Preprocessor for prequery-style metadata embedded in Typst documents.
/// See https://typst.app/universe/package/prequery for more details.
///
/// Running this program looks for a `typst.toml` file and reads the contained
/// `[[tool.prequery.jobs]]` configuration to run any number of preprocessing jobs.
/// These jobs extract metadata from the INPUT file and can take actions accordingly,
/// usually saving information to files so that it can later be read by Typst.
#[derive(Parser, Debug, Clone, PartialEq, Eq)]
pub struct CliArguments {
    /// Configures the Typst executable to use
    #[clap(long, value_name = "EXE", default_value = "typst")]
    pub typst: PathBuf,

    /// Configures the project root (for absolute paths)
    #[clap(long = "root", value_name = "DIR", env = "TYPST_ROOT")]
    pub root: Option<PathBuf>,

    /// Path to the input Typst file. `prequery-preprocess` will look for a `typst.toml` file in
    /// directories upwards from that file to determine jobs.
    pub input: PathBuf,
}
