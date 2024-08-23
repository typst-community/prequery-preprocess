//! APIs for the implementation of preprocessors, and preprocessor management

use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use once_cell::sync::Lazy;

use crate::manifest;
pub use error::{ConfigError, ConfigResult, ExecutionError, ExecutionResult, ManifestError};

/// A configured preprocessor that can be executed for its side effect
#[async_trait]
pub trait Preprocessor {
    /// this preprocessor's name, which normally comes from [manifest::Job::name].
    fn name(&self) -> &str;

    /// Executes this preprocessor
    async fn run(&mut self) -> ExecutionResult<()>;
}

/// A dynamically dispatched, boxed preprocessor
pub type BoxedPreprocessor = Box<dyn Preprocessor + Send>;

/// A factory for creating [Preprocessor]s. This trait has a blanket implementation for functions
/// with the signature of [PreprocessorDefinition::configure] and does not usually need to be
/// implemented manually.
pub trait PreprocessorFactory {
    /// Creates the preprocessor. The manifest is checked for validity, but no processing is done
    /// yet.
    fn configure(
        &self,
        name: String,
        manifest: toml::Table,
        query: manifest::Query,
    ) -> ConfigResult<BoxedPreprocessor>;
}

impl<T> PreprocessorFactory for T
where
    T: Send + Sync,
    T: Fn(String, toml::Table, manifest::Query) -> ConfigResult<BoxedPreprocessor>,
{
    fn configure(
        &self,
        name: String,
        manifest: toml::Table,
        query: manifest::Query,
    ) -> ConfigResult<BoxedPreprocessor> {
        self(name, manifest, query)
    }
}

/// A preprocessor definition that can be put into the [PREPROCESSORS] map.
pub trait PreprocessorDefinition {
    /// The identifier of the preprocessor, referenced by the [manifest::Job::kind] field
    const NAME: &'static str;

    /// The specific error type for this preprocessor
    type Error: Error + Send + Sync + 'static;

    /// Creates the preprocessor. The manifest is checked for validity, but no processing is done
    /// yet.
    fn configure(
        name: String,
        manifest: toml::Table,
        query: manifest::Query,
    ) -> ConfigResult<BoxedPreprocessor> {
        let preprocessor = Self::configure_impl(name, manifest, query)
            .map_err(|error| ManifestError::new(Self::NAME, error))?;
        Ok(preprocessor)
    }

    /// Creates the preprocessor; implementation part.
    fn configure_impl(
        name: String,
        manifest: toml::Table,
        query: manifest::Query,
    ) -> Result<BoxedPreprocessor, Self::Error>;
}

type PreprocessorMap = HashMap<&'static str, &'static (dyn PreprocessorFactory + Sync)>;

/// Map of preprocessors defined in this crate
static PREPROCESSORS: Lazy<PreprocessorMap> = Lazy::new(|| {
    fn register<T: PreprocessorDefinition + 'static>(map: &mut PreprocessorMap) {
        map.insert(T::NAME, &T::configure);
    }

    let mut map = HashMap::new();
    register::<crate::web_resource::WebResourceFactory>(&mut map);
    map
});

/// looks up the preprocessor according to [manifest::Job::kind] and returns the name and result of
/// creating the preprocessor. The creation may fail if the kind is not recognized, or some part of
/// the manifest was not valid for that kind.
pub fn get_preprocessor(job: manifest::Job) -> Result<BoxedPreprocessor, (String, ConfigError)> {
    let manifest::Job {
        name,
        kind,
        query,
        manifest,
    } = job;
    let inner = || {
        let Some(preprocessor) = PREPROCESSORS.get(kind.as_str()) else {
            return Err(ConfigError::Unknown(kind));
        };
        let preprocessor = preprocessor.configure(name.clone(), manifest, query)?;
        Ok(preprocessor)
    };
    inner().map_err(|error| (name, error))
}

mod error {
    use std::error::Error;

    use thiserror::Error;
    use tokio::task::JoinError;

    /// A problem with the preprocessor's configuration.
    #[derive(Error, Debug)]
    pub enum ConfigError {
        /// The preprocessor kind is not known
        #[error("unknown job kind: {0}")]
        Unknown(String),
        /// The manifest is invalid for the specific preprocessor
        #[error("invalid job config")]
        Manifest(#[from] ManifestError),
    }

    /// A problem with the preprocessor's configuration
    #[derive(Error, Debug)]
    #[error("the job of kind `{kind}` could was configured incorrectly")]
    pub struct ManifestError {
        kind: &'static str,
        #[source]
        source: Box<dyn Error + Send + Sync + 'static>,
    }

    impl ManifestError {
        /// Creates a new manifest error for a preprocessor of the given kind
        pub fn new<E: Error + Send + Sync + 'static>(kind: &'static str, source: E) -> Self {
            let source = Box::new(source);
            Self { kind, source }
        }
    }

    /// A problem during the job's execution
    #[derive(Error, Debug)]
    pub enum ExecutionError {
        /// The job failed for preprocessor-specific reasons
        #[error("the job did not execute successfully")]
        Execution(#[source] Box<dyn Error + Send + Sync + 'static>),
        /// An error while waiting for the job to finish
        #[error("waiting for a job failed")]
        Join(#[from] JoinError),
    }

    impl ExecutionError {
        /// Creates a new execution error from a specific preprocessor's error
        pub fn new<E: Error + Send + Sync + 'static>(source: E) -> Self {
            Self::Execution(Box::new(source))
        }
    }

    /// A result with a config error in it
    pub type ConfigResult<T> = Result<T, ConfigError>;

    /// A result with an execution error in it
    pub type ExecutionResult<T> = Result<T, ExecutionError>;
}
