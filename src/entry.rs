//! Contains the executable's entry point

use std::sync::Arc;

use crate::error::{MultiplePreprocessorExecutionError, Result};
use crate::utils;
use crate::world::{DefaultWorld, DynWorld};

/// Entry point; reads the command line arguments, determines the input files and jobs to run, and
/// then executes the jobs.
#[tokio::main]
pub async fn main() -> Result<()> {
    let world: DynWorld = Arc::new(DefaultWorld::new());

    let config = world.read_typst_toml().await?;
    let jobs = config.get_preprocessors(&world)?;

    let jobs = jobs.into_iter().map(|mut job| async move {
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
    let errors = utils::spawn_set(jobs).await;

    if !errors.is_empty() {
        return Err(MultiplePreprocessorExecutionError::new(errors).into());
    }

    Ok(())
}
