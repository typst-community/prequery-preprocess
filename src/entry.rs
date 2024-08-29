//! Contains the executable's entry point

use tokio::task::JoinSet;

use crate::args::ARGS;
use crate::error::{MultiplePreprocessorExecutionError, Result};

/// Entry point; reads the command line arguments, determines the input files and jobs to run, and
/// then executes the jobs.
#[tokio::main]
pub async fn main() -> Result<()> {
    let config = ARGS.read_typst_toml().await?;
    let jobs = config.get_preprocessors()?;

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
        return Err(MultiplePreprocessorExecutionError::new(errors).into());
    }

    Ok(())
}
