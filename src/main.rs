#![warn(missing_docs)]
//! A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.

use anyhow::{Context, Result};
use clap::Parser;

use typst_preprocess::args::CliArguments;
use typst_preprocess::config;
use typst_preprocess::preprocessor::PREPROCESSORS;

/// Entry point; reads the command line arguments, determines the input files and jobs to run, and
/// then executes the jobs.
#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArguments::parse();
    let typst_toml = args.resolve_typst_toml().await?;
    let config = config::Config::read(typst_toml).await?;

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
