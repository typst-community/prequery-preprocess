//! APIs for the implementation of preprocessors, and preprocessor management

use async_trait::async_trait;

mod factory;

pub use error::{ConfigError, ConfigResult, ExecutionError, ExecutionResult, ManifestError};
pub use factory::{preprocessors, PreprocessorDefinition, PreprocessorFactory, PreprocessorMap};

/// A configured preprocessor that can be executed for its side effect
#[async_trait]
pub trait Preprocessor {
    /// this preprocessor's name, which normally comes from [Job::name][crate::manifest::Job::name].
    fn name(&self) -> &str;

    /// Executes this preprocessor
    async fn run(&mut self) -> ExecutionResult<()>;
}

/// A dynamically dispatched, boxed preprocessor
pub type BoxedPreprocessor = Box<dyn Preprocessor + Send>;

mod error {
    use std::borrow::Cow;
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
        kind: Cow<'static, str>,
        #[source]
        source: Box<dyn Error + Send + Sync + 'static>,
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
