#![warn(missing_docs)]
//! A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.

use anyhow::{anyhow, Result};
use clap::Parser;

use typst_preprocess::args::CliArguments;
use typst_preprocess::config;
use typst_preprocess::preprocessor;

/// Entry point; reads the command line arguments, determines the input files and jobs to run, and
/// then executes the jobs.
#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArguments::parse();
    let typst_toml = args.resolve_typst_toml().await?;
    let config = config::Config::read(typst_toml).await?;

    println!("{args:?}");
    println!("{config:?}");

    let preprocessors: Vec<(_, Result<_>)> = config.jobs.into_iter()
        .map(|job| preprocessor::get_preprocessor(&args, job))
        .collect();

    let mut errors = preprocessors.iter()
        .filter_map(|(name, result)| {
            result.as_ref().err().map(|error| (name, error))
        })
        .peekable();

    if errors.peek().is_some() {
        eprintln!("at least one preprocessor has configuration errors:");
        for (name, error) in errors {
            eprintln!("[{name}] {error}");
        }
        return Err(anyhow!("at least one preprocessor has configuration errors"));
    }


    for (name, preprocessor) in preprocessors {
        let mut preprocessor = preprocessor.expect("error already handled");
        println!("[{name}] beginning job...");
        preprocessor.run().await?;
        println!("[{name}] finished job...");
    }

    Ok(())
}
