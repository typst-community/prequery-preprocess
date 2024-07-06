//! The actual preprocessors and management of those

use std::collections::HashMap;

use anyhow::Result;
use once_cell::sync::Lazy;

use crate::args::CliArguments;
use crate::config;

pub mod web_resource;

/// A configured preprocessor that can be executed for its side effect
pub trait Preprocessor {
    /// Executes this preprocessor
    fn run(&mut self) -> Result<()>;
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
