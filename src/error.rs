//! Error types for the overall typst-preprocessor API

use std::fmt;

use thiserror::Error;

use crate::{manifest, preprocessor};

/// Indicates that the query config is not valid for web-resource
#[derive(Error, Debug)]
pub enum Error {
    /// The typst.toml file could not be read
    #[error("prequery configuration could not be read from typst.toml")]
    Manifest(#[from] manifest::Error),
    /// A preprocessor is not configured correctly
    #[error(transparent)]
    PreprocessorConfig(#[from] MultiplePreprocessorConfigError),
    /// A preprocessor's execution failed
    #[error(transparent)]
    PreprocessorExecution(#[from] MultiplePreprocessorExecutionError),
}

/// One or more preprocessors were not configured correctly
#[derive(Error, Debug)]
pub struct MultiplePreprocessorConfigError {
    errors: Vec<(String, preprocessor::ConfigError)>,
}

impl MultiplePreprocessorConfigError {
    /// Creates a new error
    pub fn new(errors: Vec<(String, preprocessor::ConfigError)>) -> Self {
        Self { errors }
    }
}

impl fmt::Display for MultiplePreprocessorConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at least one job's configuration failed:")?;
        for (name, error) in &self.errors {
            writeln!(f)?;
            write!(f, "  [{name}] {error}")?;
        }
        Ok(())
    }
}

/// One or more preprocessors failed during execution
#[derive(Error, Debug)]
pub struct MultiplePreprocessorExecutionError {
    errors: Vec<preprocessor::ExecutionError>,
}

impl MultiplePreprocessorExecutionError {
    /// Creates a new error
    pub fn new(errors: Vec<preprocessor::ExecutionError>) -> Self {
        Self { errors }
    }
}

impl fmt::Display for MultiplePreprocessorExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at least one job's execution failed:")?;
        for error in &self.errors {
            writeln!(f)?;
            write!(f, "  {error}")?;
        }
        Ok(())
    }
}

/// Result type alias that defaults error to [enum@Error].
pub type Result<T, E = Error> = std::result::Result<T, E>;
