#![warn(missing_docs)]
//! A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.

use itertools::{Either, Itertools};
use tokio::task::JoinSet;
use typst_preprocess::args::ARGS;
use typst_preprocess::error::{self, Result};
use typst_preprocess::manifest::{self, PrequeryManifest};
use typst_preprocess::preprocessor;

/// Entry point; reads the command line arguments, determines the input files and jobs to run, and
/// then executes the jobs.
#[tokio::main]
async fn main() -> Result<()> {
    let typst_toml = ARGS
        .resolve_typst_toml()
        .await
        .map_err(manifest::Error::from)?;
    let config = PrequeryManifest::read(typst_toml).await?;

    let jobs: Vec<_> = config
        .jobs
        .into_iter()
        .map(preprocessor::get_preprocessor)
        .collect();

    let (jobs, errors): (Vec<_>, Vec<_>) = jobs.into_iter().partition_map(|result| match result {
        Ok(value) => Either::Left(value),
        Err(err) => Either::Right(err),
    });

    if !errors.is_empty() {
        return Err(error::MultiplePreprocessorConfigError::new(errors).into());
    }

    let mut set = JoinSet::new();

    for mut job in jobs {
        set.spawn(async move {
            println!("[{}] beginning job...", job.name());
            let result = job.run().await;
            match &result {
                Ok(()) => {
                    println!("[{}] job finished", job.name());
                }
                Err(error) => {
                    eprintln!("[{}] job failed: {error:?}", job.name());
                }
            }
            result
        });
    }

    let mut errors = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Err(error) => errors.push(error.into()),
            Ok(Err(error)) => errors.push(error),
            Ok(Ok(())) => {}
        }
    }

    if !errors.is_empty() {
        return Err(error::MultiplePreprocessorExecutionError::new(errors).into());
    }

    Ok(())
}
