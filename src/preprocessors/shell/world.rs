use std::ffi::OsStr;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process;

use super::index::Index;
use super::{CommandError, FileError, IndexError};

/// The context for executing a Shell job. Defines how downloading and saving files work, and thus
/// allows mocking.
#[cfg_attr(feature = "test", mockall::automock(type MainWorld = crate::world::MockWorld;))]
#[async_trait]
pub trait World: Send + Sync + 'static {
    type MainWorld: crate::world::World;

    /// Creates a new shell world based on the given main world.
    fn new(main: Arc<Self::MainWorld>) -> Self;

    /// Accesses the main world.
    fn main(&self) -> &Arc<Self::MainWorld>;

    /// Reads the shell index at the given path, interpreted relative to the typst.toml file.
    async fn read_index(&self, path: &Path) -> Result<Index, IndexError>;

    /// Writes the shell index to its location.
    async fn write_index(&self, index: &Index) -> Result<(), IndexError>;

    /// Runs a shell command.
    async fn run_command<S>(&self, command: &[S], input: &[u8]) -> Result<Vec<u8>, CommandError>
    where
        S: AsRef<OsStr> + std::fmt::Debug + Send + Sync + 'static;

    /// Writes a command's result to a file.
    async fn write_output(&self, location: &Path, output: &[u8]) -> Result<(), FileError>;
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

    async fn read_index(&self, path: &Path) -> Result<Index, IndexError> {
        let mut location = self.main().resolve_typst_toml().await?;
        let result = location.pop();
        assert!(
            result,
            "the path should have had a final filename component"
        );
        location.push(path);

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

    async fn run_command<S>(&self, command: &[S], input: &[u8]) -> Result<Vec<u8>, CommandError>
    where
        S: AsRef<OsStr> + Send + Sync,
    {
        let mut child = process::Command::new(&command[0])
            .args(&command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .expect("child did not have a handle to stdin");
        stdin.write_all(input).await?;
        stdin.shutdown().await?;
        drop(stdin);

        let stdout = child
            .stdout
            .take()
            .expect("child did not have a handle to stdout");
        let mut stdout = BufReader::new(stdout);
        let mut output = Vec::new();
        stdout.read_to_end(&mut output).await?;

        Ok(output)
    }

    async fn write_output(&self, location: &Path, output: &[u8]) -> Result<(), FileError> {
        if let Some(parent) = location.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = fs::File::create(&location).await?;
        file.write_all(output).await?;
        file.flush().await?;
        Ok(())
    }
}
