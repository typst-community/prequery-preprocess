#![warn(missing_docs)]
//! A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.

use anyhow::{anyhow, Result};


use tokio::task::JoinSet;
use typst_preprocess::args::ARGS;
use typst_preprocess::config;
use typst_preprocess::preprocessor;

/// Entry point; reads the command line arguments, determines the input files and jobs to run, and
/// then executes the jobs.
#[tokio::main]
async fn main() -> Result<()> {
    let typst_toml = ARGS.resolve_typst_toml().await?;
    let config = config::Config::read(typst_toml).await?;

    let preprocessors: Vec<_> = config.jobs.into_iter()
        .map(|job| preprocessor::get_preprocessor(job))
        .collect();

    let mut errors = preprocessors.iter()
        .filter_map(|result| result.as_ref().err())
        .peekable();

    if errors.peek().is_some() {
        eprintln!("at least one preprocessor has configuration errors:");
        for (name, error) in errors {
            eprintln!("[{name}] {error}");
        }
        return Err(anyhow!("at least one preprocessor has configuration errors"));
    }

    let mut set = JoinSet::new();

    for preprocessor in preprocessors {
        let mut preprocessor = preprocessor.expect("error already handled");
        set.spawn(async move {
            println!("[{}] beginning job...", preprocessor.name());
            let result = preprocessor.run().await;
            match &result {
                Ok(()) => {
                    println!("[{}] job finished", preprocessor.name());
                },
                Err(error) => {
                    eprintln!("[{}] job failed: {error}", preprocessor.name());
                },
            }
            result
        });
    }

    while let Some(_) = set.join_next().await {
        // we just want to join all the tasks
    }

    Ok(())
}
