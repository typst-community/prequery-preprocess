//! APIs for the implementation of preprocessors, and preprocessor management

use std::sync::Arc;

use async_trait::async_trait;

mod factory;

pub use error::{
    ConfigError, ConfigResult, DynError, ExecutionError, ExecutionResult, ManifestError,
};
#[cfg(feature = "test")]
pub use factory::MockPreprocessorDefinition;
pub use factory::{PreprocessorDefinition, PreprocessorFactory, PreprocessorMap};

use crate::world::World;

/// A configured preprocessor that can be executed for its side effect
#[cfg_attr(feature = "test", mockall::automock)]
#[async_trait]
pub trait Preprocessor<W: World> {
    /// The world this preprocessor works in.
    fn world(&self) -> &Arc<W>;

    /// This preprocessor's name, which normally comes from [Job::name][crate::manifest::Job::name].
    fn name(&self) -> &str;

    /// Executes this preprocessor
    async fn run(&mut self) -> Result<(), DynError>;
}

/// A dynamically dispatched, boxed preprocessor
pub type BoxedPreprocessor<W> = Box<dyn Preprocessor<W> + Send>;

mod error {
    use std::borrow::Cow;
    use std::error::Error;

    use thiserror::Error;
    use tokio::task::JoinError;

    /// A boxed, dynamically typed error that is produced by a specific preprocessor
    pub type DynError = Box<dyn Error + Send + Sync + 'static>;

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
    #[error("the job of kind `{kind}` was configured incorrectly")]
    pub struct ManifestError {
        kind: Cow<'static, str>,
        #[source]
        source: DynError,
    }

    impl ManifestError {
        /// Creates a new manifest error for a preprocessor of the given kind
        pub fn new<E: Error + Send + Sync + 'static>(kind: Cow<'static, str>, source: E) -> Self {
            let source = Box::new(source);
            Self { kind, source }
        }
    }

    /// A problem during the job's execution
    #[derive(Error, Debug)]
    pub enum ExecutionError {
        /// The job failed for preprocessor-specific reasons
        #[error(transparent)]
        Execution(#[from] DynError),
        /// An error while waiting for the job to finish
        #[error("waiting for a job failed")]
        Join(#[from] JoinError),
    }

    /// A result with a config error in it
    pub type ConfigResult<T> = Result<T, ConfigError>;

    /// A result with an execution error in it
    pub type ExecutionResult<T> = Result<T, ExecutionError>;
}
