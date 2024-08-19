//! APIs for the implementation of preprocessors, and preprocessor management

use std::collections::HashMap;

use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use once_cell::sync::Lazy;

use crate::manifest;

/// A configured preprocessor that can be executed for its side effect
#[async_trait]
pub trait Preprocessor {
    /// this preprocessor's name, which normally comes from [manifest::Job::name].
    fn name(&self) -> &str;

    /// Executes this preprocessor
    async fn run(&mut self) -> Result<()>;
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
    ) -> Result<BoxedPreprocessor>;
}

impl<T> PreprocessorFactory for T
where
    T: Send + Sync,
    T: Fn(String, toml::Table, manifest::Query) -> Result<BoxedPreprocessor>,
{
    fn configure(
        &self,
        name: String,
        manifest: toml::Table,
        query: manifest::Query,
    ) -> Result<BoxedPreprocessor> {
        self(name, manifest, query)
    }
}

/// A preprocessor definition that can be put into the [PREPROCESSORS] map.
pub trait PreprocessorDefinition {
    /// The identifier of the preprocessor, referenced by the [manifest::Job::kind] field
    const NAME: &'static str;

    /// Creates the preprocessor. The manifest is checked for validity, but no processing is done
    /// yet.
    fn configure(
        name: String,
        manifest: toml::Table,
        query: manifest::Query,
    ) -> Result<BoxedPreprocessor>;
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
pub fn get_preprocessor(job: manifest::Job) -> Result<BoxedPreprocessor, (String, Error)> {
    let manifest::Job {
        name,
        kind,
        query,
        manifest,
    } = job;
    let inner = || {
        let preprocessor = PREPROCESSORS
            .get(kind.as_str())
            .with_context(|| format!("unknown job kind: {kind}"))?
            .configure(name.clone(), manifest, query)?;
        Ok(preprocessor)
    };
    inner().map_err(|error| (name, error))
}
