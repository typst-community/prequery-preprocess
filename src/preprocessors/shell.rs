//! The `shell` preprocessor

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use itertools::{Either, Itertools};
use tokio::sync::Mutex;

use crate::preprocessor::{DynError, Preprocessor};
use crate::query::{self, Query};
use crate::world::{World as _, WorldExt as _};

mod error;
mod factory;
#[cfg(not(feature = "test"))]
mod index;
#[cfg(feature = "test")]
pub mod index;
mod manifest;
mod query_data;
mod world;

use index::*;
use manifest::*;
use query_data::*;
use world::World;

pub use error::*;
pub use factory::ShellFactory;
#[cfg(feature = "test")]
pub use world::{__mock_MockWorld_World::__new::Context as MockWorld_NewContext, MockWorld};

/// The `shell` preprocessor
#[derive(Debug)]
pub struct Shell<W: World> {
    #[debug(skip)]
    world: Arc<W>,
    name: String,
    manifest: Manifest,
    index: Option<Mutex<Index>>,
    query: Query,
}

impl<W: World> Shell<W> {
    pub(crate) fn new(
        world: Arc<W>,
        name: String,
        manifest: Manifest,
        index: Option<Mutex<Index>>,
        query: Query,
    ) -> Self {
        Self {
            world,
            name,
            index,
            manifest,
            query,
        }
    }

    async fn populate_index(&mut self) -> Result<(), IndexError> {
        if let Some(path) = self.manifest.index.as_ref() {
            // an index is in use
            let index = self.world.read_index(path).await?;
            self.index = Some(Mutex::new(index));
        } else {
            // no index is in use
            self.index = None;
        }

        Ok(())
    }

    async fn query(&self) -> query::Result<QueryData> {
        let data = self.world.main().query(&self.query).await?;
        Ok(data)
    }

    async fn run_command(
        self: Arc<Self>,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, CommandError> {
        let command = &self.manifest.command;
        let input = serde_json::to_vec(&input)?;

        let output = self.world.run_command(&command.0, &input).await?;
        let output = serde_json::from_slice(&output)?;

        Ok(output)
    }

    async fn write_output(
        self: Arc<Self>,
        location: PathBuf,
        output: serde_json::Value,
    ) -> Result<(), FileError> {
        let output = serde_json::to_vec(&output)?;
        self.world.write_output(&location, &output).await?;
        Ok(())
    }

    async fn run_impl(self: &mut Arc<Self>) -> ExecutionResult<()> {
        Arc::get_mut(self)
            .expect("shell ref count should be one before starting the processing")
            .populate_index()
            .await?;

        let mut l = self.world.main().log();
        let name = self.name();

        let query_data = self.query().await?;
        let (outputs, inputs) = query_data.split();
        let output = if self.manifest.joined {
            // run one command
            log!(
                l,
                "[{name}] executing command \"{}\" with {} joined inputs...",
                self.manifest.command,
                inputs.len(),
            );

            let length = inputs.len();

            let input = inputs.into();
            let output = Arc::clone(self).run_command(input).await?;

            // output must be an array as long as the input
            if !matches!(&output, serde_json::Value::Array(outputs) if outputs.len() == length) {
                return Err(CommandError::Array.into());
            }

            output
        } else {
            // run many commands
            log!(
                l,
                "[{name}] executing command \"{}\" for {} inputs...",
                self.manifest.command,
                inputs.len(),
            );

            let commands = inputs
                .into_iter()
                .map(|input| Arc::clone(self).run_command(input));
            let results = futures::future::join_all(commands).await;

            // collect
            let (outputs, errors): (Vec<_>, Vec<_>) =
                results.into_iter().partition_map(|result| match result {
                    Ok(output) => Either::Left(output),
                    Err(error) => Either::Right(error),
                });
            if !errors.is_empty() {
                return Err(error::MultipleCommandError::new(errors).into());
            }

            let output = outputs.into();
            output
        };

        match outputs {
            Output::SharedOutput(path) => {
                // save to one file
                log!(
                    l,
                    "[{name}] execution finished, saving to {}...",
                    path.display(),
                );

                let output = serde_json::to_vec(&output).map_err(CommandError::from)?;
                self.world.write_output(&path, &output).await?;
            }
            Output::IndividualOutput(paths) => {
                // save to many files
                log!(l, "[{name}] execution finished, saving...",);

                let serde_json::Value::Array(outputs) = output else {
                    unreachable!("output is an array, previously checked");
                };
                let writes = paths
                    .into_iter()
                    .zip(outputs)
                    .map(|(path, output)| Arc::clone(self).write_output(path, output));
                let results = futures::future::join_all(writes).await;
                let errors: Vec<_> = results.into_iter().filter_map(Result::err).collect();
                if !errors.is_empty() {
                    return Err(error::MultipleFileError::new(errors).into());
                }
            }
        }

        log!(l, "[{name}] command results saved",);

        if let Some(index) = &self.index {
            let index = index.lock().await;
            self.world.write_index(&index).await?;
        }

        Ok::<_, ExecutionError>(())
    }
}

#[async_trait]
impl<W: World> Preprocessor<W::MainWorld> for Arc<Shell<W>> {
    fn world(&self) -> &Arc<W::MainWorld> {
        self.world.main()
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&mut self) -> Result<(), DynError> {
        self.run_impl().await.map_err(Box::new)?;
        Ok(())
    }
}
