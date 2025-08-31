use std::fmt;
use std::io;

use thiserror::Error;
use tokio::task::JoinError;

use crate::query;

/// An error in the configuration of the job's query
#[derive(Error, Debug)]
pub enum QueryConfigError {
    /// An option without a default value was not given
    #[error("invalid web-resource query configuration")]
    Builder(#[from] query::QueryBuilderError),
    /// The `--one` option was given, but is not supported
    #[error("web-resource does not support --one")]
    One,
}

/// A problem with the preprocessor's configuration
#[derive(Error, Debug)]
pub enum ManifestError {
    /// The provided configuration is not valid for a web-resource job
    #[error("invalid web-resource configuration")]
    Manifest(#[from] toml::de::Error),
    /// An error in the configuration of the job's query
    #[error(transparent)]
    Query(#[from] QueryConfigError),
}

/// A problem with using the index of downloaded resources
#[derive(Error, Debug)]
pub enum IndexError {
    /// I/O error while accessing the index file
    #[error("web-resource index file could not be read or written")]
    Io(#[from] io::Error),
    /// Unexpected version: must be 1
    #[error("expected web-resource index file version 1, was {0}")]
    Version(usize),
    /// Error parsing the index file's contents
    #[error("invalid web-resource index file content")]
    Parse(#[from] toml::de::Error),
    /// Error writing new index file contents
    #[error("web-resource index: TOML writing error")]
    Write(#[from] toml::ser::Error),
}

/// An error during downloading a resource from the web
#[derive(Error, Debug)]
pub enum DownloadError {
    /// A network error during the download
    #[error("network I/O error during download")]
    Network(#[from] reqwest::Error),
    /// An error accessing the local file for the resource
    #[error("file I/O error during download")]
    File(#[from] io::Error),
    /// An error while waiting for the download to finish
    #[error("waiting for a download task failed")]
    Join(#[from] JoinError),
}

/// One or more preprocessors were not configured correctly
#[derive(Error, Debug)]
pub struct MultipleDownloadError {
    errors: Vec<DownloadError>,
}

impl MultipleDownloadError {
    /// Creates a new error
    pub fn new(errors: Vec<DownloadError>) -> Self {
        Self { errors }
    }
}

impl fmt::Display for MultipleDownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at least one download failed:")?;
        for error in &self.errors {
            writeln!(f)?;
            write!(f, "  {error}")?;
        }
        Ok(())
    }
}

/// An error during the web-resource job's execution
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
    Download(#[from] MultipleDownloadError),
}

/// A result with a config error in it
pub type ManifestResult<T> = Result<T, ManifestError>;

/// A result with an execution error in it
pub type ExecutionResult<T> = Result<T, ExecutionError>;
