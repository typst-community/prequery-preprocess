#![warn(missing_docs)]
//! A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.

use std::path::{self, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use tokio::fs;

use typst_preprocess::args::CliArguments;
use typst_preprocess::config;
use typst_preprocess::preprocessor::PREPROCESSORS;


/// Returns the path of the `typst.toml` file that is closest to the specified input file. The input
/// path should be an actual file (the main `.typ` file that will be queried by the preprocessor);
/// if it is a directory, that directory itself would not be searched!
async fn resolve_typst_toml<P: AsRef<Path>>(input: P) -> Result<PathBuf> {
    const TYPST_TOML: &str = "typst.toml";

    let input = path::absolute(&input)
        .with_context(|| {
            let input_str = input.as_ref().to_string_lossy();
            format!("cannot resolve {TYPST_TOML} because input file {input_str} can't be resolved")
        })?;
    let mut p = input.clone();

    // the input path needs to refer to a file. refer to typst.toml instead
    p.set_file_name(TYPST_TOML);
    // repeat as long as the path does not point to an accessible regular file
    while !fs::metadata(&p).await.map_or(false, |m| m.is_file()) {
        // remove the file name
        let result = p.pop();
        assert!(result, "the path should have had a final component of `{TYPST_TOML}`");
        // go one level up
        let result = p.pop();
        if !result {
            // if there is no level up, not typst.toml was found
            let input_str = input.to_string_lossy();
            return Err(anyhow!("no {TYPST_TOML} file was found in any ancestor directory of {input_str}"));
        }
        // re-add the file name
        p.push(TYPST_TOML);
    }
    Ok(p)
}

/// Resolves and reads the `typst.toml` file relevant for the given input file.
async fn read_typst_toml<P: AsRef<Path>>(input: P) -> Result<config::Config> {
    let typst_toml = resolve_typst_toml(input).await?;
    let typst_toml = fs::read_to_string(typst_toml).await?;
    let typst_toml = config::read_typst_toml(&typst_toml)?;
    Ok(typst_toml)
}

/// Entry point; reads the command line arguments, determines the input files and jobs to run, and
/// then executes the jobs.
#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArguments::parse();
    let config = read_typst_toml(&args.input).await?;

    println!("{args:?}");
    println!("{config:?}");

    for config::Job { name, kind, query, config } in config.jobs {
        println!("working on {name}");
        let mut preprocessor = PREPROCESSORS.get(kind.as_str())
            .with_context(|| format!("unknown job kind: {kind}"))?
            .configure(&args, config, query)?;
        preprocessor.run().await?;
    }

    Ok(())
}
