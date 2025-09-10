//! Contains the executable's entry point

use std::process::exit;
use std::sync::Arc;

use crate::error::{MultiplePreprocessorExecutionError, Result};
use crate::preprocessor::{ExecutionError, Preprocessor};
use crate::reporting::ErrorExt;
use crate::utils;
use crate::world::{DefaultWorld, World, WorldExt};

/// Entry point; reads the command line arguments, determines the input files and jobs to run, and
/// then executes the jobs.
#[tokio::main]
pub async fn main() {
    let result = run(DefaultWorld::new()).await;
    if result.is_err() {
        exit(1);
    }
}

/// Entry point; takes a World and executes preprocessors according to the contained data.
pub async fn run(world: impl World) -> Result<()> {
    let world = Arc::new(world);
    let mut l = world.log();
    let config = world.read_typst_toml().await?;
    let jobs = world.get_preprocessors(config)?;

    async fn run_job(
        mut job: Box<dyn Preprocessor<impl World> + Send>,
    ) -> Result<(), (String, ExecutionError)> {
        let mut l = job.world().log();
        log!(l, "[{}] beginning job...", job.name());
        let result = job.run().await;
        match &result {
            Ok(()) => {
                log!(l, "[{}] job finished", job.name());
            }
            Err(error) => {
                log!(l, "[{}] job failed: {error}", job.name());
            }
        }
        result.map_err(|error| (job.name().to_string(), error.into()))
    }

    let jobs = jobs
        .into_iter()
        .map(|job| (job.name().to_string(), run_job(job)));
    let errors = utils::spawn_set_with_id(jobs, |name, error| (name, error.into())).await;

    if !errors.is_empty() {
        let error: crate::error::Error = MultiplePreprocessorExecutionError::new(errors).into();
        log!(l, "{}", error.error_chain());
        return Err(error);
    }

    Ok(())
}
