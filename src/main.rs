#![warn(missing_docs)]
//! A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.

use anyhow::Result;
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

    for job in config.jobs {
        println!("working on {}", job.name);
        let (_name, preprocessor) = preprocessor::get_preprocessor(&args, job);
        let mut preprocessor = preprocessor?;
        preprocessor.run().await?;
    }

    Ok(())
}
