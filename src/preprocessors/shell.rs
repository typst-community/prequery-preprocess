//! The `shell` preprocessor

use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
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
        let mut l = self.world.main().log();

        let name = self.name();
        let command = &self.manifest.command;
        let input = serde_json::to_vec(&input)?;

        log!(
            l,
            "[{name}] run command {:?} on {}",
            command,
            String::from_utf8_lossy(&input),
        );

        let output = self.world.run_command(command, &input).await?;
        let output = serde_json::from_slice(&output)?;

        Ok(output)
    }

    async fn run_impl(self: &mut Arc<Self>) -> ExecutionResult<()> {
        Arc::get_mut(self)
            .expect("shell ref count should be one before starting the processing")
            .populate_index()
            .await?;

        let query_data = self.query().await?;
        let inputs = query_data
            .items
            .into_iter()
            .filter_map(InputItem::into_data);

        let errors = if self.manifest.joined {
            // combine all inputs and process in one swoop
            let input = inputs.collect::<Vec<_>>().into();
            let result = Arc::clone(self).run_command(input).await;
            result.err().into_iter().collect()
        } else {
            // execute individual commands per input
            let commands = inputs.map(|input| Arc::clone(self).run_command(input));
            // utils::spawn_set(commands).await
            todo!() as Vec<_>
        };

        if let Some(index) = &self.index {
            let index = index.lock().await;
            self.world.write_index(&index).await?;
        }

        if !errors.is_empty() {
            return Err(error::MultipleCommandError::new(errors).into());
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
