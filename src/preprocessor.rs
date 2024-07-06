//! The actual preprocessors and management of those

use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use once_cell::sync::Lazy;

use crate::args::CliArguments;
use crate::config;

pub mod web_resource;

/// A configured preprocessor that can be executed for its side effect
#[async_trait]
pub trait Preprocessor {
    /// Executes this preprocessor
    async fn run(&mut self) -> Result<()>;
}

/// A dynamically dispatched, boxed preprocessor
pub type BoxedPreprocessor<'a> = Box<dyn Preprocessor + 'a>;

/// A factory for creating [Preprocessor]s. This trait has a blanket implementation for functions
/// with the signature of [PreprocessorDefinition::configure] and does not usually need to be
/// implemented manually.
pub trait PreprocessorFactory {
    /// Creates the preprocessor. The configuration is checked for validity, but no processing is
    /// done yet.
    fn configure<'a>(
        &self,
        args: &'a CliArguments,
        config: toml::Table,
        query: config::Query,
    ) -> Result<BoxedPreprocessor<'a>>;
}

impl<T> PreprocessorFactory for T
where
    T: Send + Sync,
    T: for<'a> Fn(&'a CliArguments, toml::Table, config::Query) -> Result<BoxedPreprocessor<'a>>
{
    fn configure<'a>(
        &self,
        args: &'a CliArguments,
        config: toml::Table,
        query: config::Query,
    ) -> Result<BoxedPreprocessor<'a>> {
        self(args, config, query)
    }
}

/// A preprocessor definition that can be put into the [PREPROCESSORS] map.
pub trait PreprocessorDefinition {
    /// The identifier of the preprocessor, referenced by the [config::Job::kind] field
    const NAME: &'static str;

    /// Creates the preprocessor. The configuration is checked for validity, but no processing is
    /// done yet.
    fn configure<'a>(
        args: &'a CliArguments,
        config: toml::Table,
        query: config::Query,
    ) -> Result<BoxedPreprocessor<'a>>;
}

type PreprocessorMap = HashMap<&'static str, &'static (dyn PreprocessorFactory + Send + Sync)>;

/// Map of preprocessors defined in this crate
pub static PREPROCESSORS: Lazy<PreprocessorMap> = Lazy::new(|| {
    fn register<T: PreprocessorDefinition + 'static>(map: &mut PreprocessorMap) {
        map.insert(T::NAME, &T::configure);
    }

    let mut map = HashMap::new();
    register::<web_resource::WebResourceFactory>(&mut map);
    map
});

/// looks up the preprocessor according to [config::Job::kind] and returns the name and result of
/// creating the preprocessor. The creation may fail if the kind is not recognized, or some part of
/// the configuration was not valid for that kind.
pub fn get_preprocessor<'a>(args: &'a CliArguments, job: config::Job) -> (String, Result<BoxedPreprocessor<'a>>) {
    let config::Job { name, kind, query, config } = job;
    let inner = || {
        let preprocessor = PREPROCESSORS.get(kind.as_str())
            .with_context(|| format!("unknown job kind: {kind}"))?
            .configure(&args, config, query)?;
        Ok(preprocessor)
    };
    (name, inner())
}
