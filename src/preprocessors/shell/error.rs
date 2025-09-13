use std::fmt;
use std::io;

use thiserror::Error;
use tokio::task::JoinError;

use crate::query;
use crate::reporting::{ErrorExt, WriteExt};

/// An error in the configuration of the job's query
#[derive(Error, Debug)]
pub enum QueryConfigError {
    /// An option without a default value was not given
    #[error("invalid shell query configuration")]
    Builder(#[from] query::QueryBuilderError),
    /// The `--one` option was given, but is not supported
    #[error("shell does not support --one")]
    One,
}

/// A problem with the preprocessor's configuration
#[derive(Error, Debug)]
pub enum ManifestError {
    /// The provided configuration is not valid for a shell job
    #[error("invalid shell configuration")]
    Manifest(#[from] toml::de::Error),
    /// An error in the configuration of the job's query
    #[error(transparent)]
    Query(#[from] QueryConfigError),
}

/// A problem with using the index of downloaded resources
#[derive(Error, Debug)]
pub enum IndexError {
    /// I/O error while accessing the index file
    #[error("shell index file could not be read or written")]
    Io(#[from] io::Error),
    /// Unexpected version: must be 1
    #[error("expected shell index file version 1, was {0}")]
    Version(usize),
    /// Error parsing the index file's contents
    #[error("invalid shell index file content")]
    Parse(#[from] toml::de::Error),
    /// Error writing new index file contents
    #[error("shell index: TOML writing error")]
    Write(#[from] toml::ser::Error),
}

/// An error while executing a shell command
#[derive(Error, Debug)]
pub enum CommandError {
    /// An error accessing the local file for the command result
    #[error(transparent)]
    File(#[from] io::Error),
    /// The command input or output was not valid
    #[error("command input or output was not valid JSON or did not fit the expected format")]
    Json(#[from] serde_json::Error),
    /// An error while waiting for the command to finish
    #[error("waiting for a command task failed")]
    Join(#[from] JoinError),
}

/// One or more preprocessors were not configured correctly
#[derive(Error, Debug)]
pub struct MultipleCommandError {
    errors: Vec<CommandError>,
}

impl MultipleCommandError {
    /// Creates a new error
    pub fn new(errors: Vec<CommandError>) -> Self {
        Self { errors }
    }
}

impl fmt::Display for MultipleCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        let mut w = f.hanging_indent("  ");
        write!(w, "at least one command failed:")?;
        for error in &self.errors {
            writeln!(w)?;
            write!(w, "{}", error.error_chain())?;
        }
        Ok(())
    }
}

/// An error during the shell job's execution
#[derive(Error, Debug)]
pub enum ExecutionError {
    /// A problem with using the index of downloaded resources
    #[error(transparent)]
    Index(#[from] IndexError),
    /// An error while executing the job's query
    #[error(transparent)]
    Query(#[from] query::Error),
    /// An error during downloading a resource from the web
    #[error(transparent)]
    Command(#[from] MultipleCommandError),
}

/// A result with a config error in it
pub type ManifestResult<T> = Result<T, ManifestError>;

/// A result with an execution error in it
pub type ExecutionResult<T> = Result<T, ExecutionError>;
