use thiserror::Error;

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

/// A result with a config error in it
pub type ManifestResult<T> = Result<T, ManifestError>;
