//! ...

use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;

use super::{BoxedPreprocessor, ConfigError, ConfigResult, ManifestError};
use crate::manifest;
use crate::world::DynWorld;

#[allow(rustdoc::private_intra_doc_links)]
/// A preprocessor definition that [Preprocessor][super::Preprocessor]s can be created from.
pub trait PreprocessorDefinition {
    /// The specific error type for this preprocessor
    type Error: Error + Send + Sync + 'static;

    /// The identifier of the preprocessor, referenced by the [Job::kind][manifest::Job::kind] field
    fn name(&self) -> Cow<'static, str>;

    /// Creates the preprocessor; implementation part.
    fn configure(
        &self,
        world: &DynWorld,
        name: String,
        manifest: toml::Table,
        query: manifest::Query,
    ) -> Result<BoxedPreprocessor, Self::Error>;
}

/// A dyn-safe version of [PreprocessorDefinition]. This trait has a blanket implementation and does
/// not usually need to be implemented manually.
pub trait PreprocessorFactory {
    /// The identifier of the preprocessor, referenced by the [Job::kind][manifest::Job::kind] field
    fn name(&self) -> Cow<'static, str>;

    /// Creates the preprocessor. The manifest is checked for validity, but no processing is done
    /// yet.
    fn configure(
        &self,
        world: &DynWorld,
        name: String,
        manifest: toml::Table,
        query: manifest::Query,
    ) -> ConfigResult<BoxedPreprocessor>;
}

impl<T> PreprocessorFactory for T
where
    T: PreprocessorDefinition,
{
    fn name(&self) -> Cow<'static, str> {
        self.name()
    }

    fn configure(
        &self,
        world: &DynWorld,
        name: String,
        manifest: toml::Table,
        query: manifest::Query,
    ) -> ConfigResult<BoxedPreprocessor> {
        let preprocessor = self
            .configure(world, name, manifest, query)
            .map_err(|error| ManifestError::new(self.name(), error))?;
        Ok(preprocessor)
    }
}

/// A map of preprocessor definitions that can be used to run a set of [Jobs][manifest::Job].
pub struct PreprocessorMap {
    map: HashMap<Cow<'static, str>, Box<dyn PreprocessorFactory + Send + Sync>>,
}

impl Default for PreprocessorMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PreprocessorMap {
    /// Creates an empty preprocessor maps
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Registers a preprocessor definition with its name in the map
    pub fn register<T>(&mut self, preprocessor: T)
    where
        T: PreprocessorDefinition + Send + Sync + 'static,
    {
        self.map.insert(preprocessor.name(), Box::new(preprocessor));
    }

    /// Looks up the preprocessor according to [Job::kind][manifest::Job::kind] and returns the name
    /// and result of creating the preprocessor. The creation may fail if the kind is not
    /// recognized, or some part of the manifest was not valid for that kind.
    pub fn get(
        &self,
        world: &DynWorld,
        job: manifest::Job,
    ) -> Result<BoxedPreprocessor, (String, ConfigError)> {
        let manifest::Job {
            name,
            kind,
            query,
            manifest,
        } = job;
        let inner = || {
            let Some(preprocessor) = self.map.get(kind.as_str()) else {
                return Err(ConfigError::Unknown(kind));
            };
            let preprocessor = preprocessor.configure(world, name.clone(), manifest, query)?;
            Ok(preprocessor)
        };
        inner().map_err(|error| (name, error))
    }
}
