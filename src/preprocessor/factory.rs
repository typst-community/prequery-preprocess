//! ...

use std::collections::HashMap;
use std::error::Error;

use once_cell::sync::Lazy;

use super::{BoxedPreprocessor, ConfigError, ConfigResult, ManifestError};
use crate::manifest;

/// A factory for creating [Preprocessor][super::Preprocessor]s. This trait has a blanket
/// implementation for functions with the signature of [PreprocessorDefinition::configure] and does
/// not usually need to be implemented manually.
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

pub type PreprocessorMap = HashMap<&'static str, &'static (dyn PreprocessorFactory + Sync)>;

#[allow(rustdoc::private_intra_doc_links)]
/// A preprocessor definition that can be put into the (private) [PREPROCESSORS] map.
pub trait PreprocessorDefinition {
    /// The identifier of the preprocessor, referenced by the [Job::kind][manifest::Job::kind] field
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

/// Map of preprocessors defined in this crate
static PREPROCESSORS: Lazy<PreprocessorMap> = Lazy::new(|| {
    fn register<T: PreprocessorDefinition + 'static>(map: &mut PreprocessorMap) {
        map.insert(T::NAME, &T::configure);
    }

    let mut map = HashMap::new();
    register::<crate::web_resource::WebResourceFactory>(&mut map);
    map
});

/// looks up the preprocessor according to [Job::kind][manifest::Job::kind] and returns the name and
/// result of creating the preprocessor. The creation may fail if the kind is not recognized, or
/// some part of the manifest was not valid for that kind.
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
