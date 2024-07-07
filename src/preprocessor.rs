//! The actual preprocessors and management of those

use std::collections::HashMap;

use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use once_cell::sync::Lazy;

use crate::config;

pub mod web_resource;

/// A configured preprocessor that can be executed for its side effect
#[async_trait]
pub trait Preprocessor {
    /// this preprocessor's name, which normally comes from [config::Job::name].
    fn name(&self) -> &str;

    /// Executes this preprocessor
    async fn run(&mut self) -> Result<()>;
}

/// A dynamically dispatched, boxed preprocessor
pub type BoxedPreprocessor = Box<dyn Preprocessor>;

/// A factory for creating [Preprocessor]s. This trait has a blanket implementation for functions
/// with the signature of [PreprocessorDefinition::configure] and does not usually need to be
/// implemented manually.
pub trait PreprocessorFactory {
    /// Creates the preprocessor. The configuration is checked for validity, but no processing is
    /// done yet.
    fn configure(
        &self,
        name: String,
        config: toml::Table,
        query: config::Query,
    ) -> Result<BoxedPreprocessor>;
}

impl<T> PreprocessorFactory for T
where
    T: Send + Sync,
    T: Fn(String, toml::Table, config::Query) -> Result<BoxedPreprocessor>
{
    fn configure(
        &self,
        name: String,
        config: toml::Table,
        query: config::Query,
    ) -> Result<BoxedPreprocessor> {
        self(name, config, query)
    }
}

/// A preprocessor definition that can be put into the [PREPROCESSORS] map.
pub trait PreprocessorDefinition {
    /// The identifier of the preprocessor, referenced by the [config::Job::kind] field
    const NAME: &'static str;

    /// Creates the preprocessor. The configuration is checked for validity, but no processing is
    /// done yet.
    fn configure(
        name: String,
        config: toml::Table,
        query: config::Query,
    ) -> Result<BoxedPreprocessor>;
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
pub fn get_preprocessor(job: config::Job) -> Result<BoxedPreprocessor, (String, Error)> {
    let config::Job { name, kind, query, config } = job;
    let inner = || {
        let preprocessor = PREPROCESSORS.get(kind.as_str())
            .with_context(|| format!("unknown job kind: {kind}"))?
            .configure(name.clone(), config, query)?;
        Ok(preprocessor)
    };
    inner().map_err(|error| (name, error))
}
