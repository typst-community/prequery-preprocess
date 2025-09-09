//! Contains the executable's entry point

use std::process::exit;
use std::sync::Arc;

use crate::error::{MultiplePreprocessorExecutionError, Result};
use crate::utils;
use crate::world::{DefaultWorld, World, WorldExt};

/// Entry point; reads the command line arguments, determines the input files and jobs to run, and
/// then executes the jobs.
#[tokio::main]
pub async fn main() -> Result<()> {
    let result = run(DefaultWorld::new()).await;
    let Err(error) = result else {
        return Ok(());
    };

    eprintln!("{}", error);

    exit(1);
}

/// Entry point; takes a World and executes preprocessors according to the contained data.
pub async fn run(world: impl World) -> Result<()> {
    let world = Arc::new(world);
    let config = world.read_typst_toml().await?;
    let jobs = world.get_preprocessors(config)?;

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
