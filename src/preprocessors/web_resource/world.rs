use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs;

use super::index::Index;
use super::IndexError;

/// The context for executing a WebResource job. Defines how downloading and saving files work, and
/// thus allows mocking.
#[async_trait]
pub trait World: Send + Sync + 'static {
    type MainWorld: crate::world::World;

    /// Creates a new web resource world based on the given main world.
    fn new(main: Arc<Self::MainWorld>) -> Self;

    /// Accesses the main world.
    fn main(&self) -> &Arc<Self::MainWorld>;

    /// Reads the web resource index at the specified location.
    async fn read_index(&self, location: PathBuf) -> Result<Index, IndexError>;

    /// Writes the web resource index to its location.
    async fn write_index(&self, index: &Index) -> Result<(), IndexError>;
}

/// The default context, accessing the real web and filesystem.
#[derive(Clone)]
pub struct DefaultWorld {
    main: Arc<crate::world::DefaultWorld>,
}

#[async_trait]
impl World for DefaultWorld {
    type MainWorld = crate::world::DefaultWorld;

    fn new(main: Arc<Self::MainWorld>) -> Self {
        Self { main }
    }

    fn main(&self) -> &Arc<Self::MainWorld> {
        &self.main
    }

    async fn read_index(&self, location: PathBuf) -> Result<Index, IndexError> {
        let index = if fs::try_exists(&location).await.unwrap_or(false) {
            // read the existing index
            Index::read(location).await?
        } else {
            // generate an empty index
            Index::new(location)
        };
        Ok(index)
    }

    async fn write_index(&self, index: &Index) -> Result<(), IndexError> {
        index.write().await?;
        Ok(())
    }
}
