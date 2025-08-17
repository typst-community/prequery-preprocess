//! Configuration types

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use itertools::{Either, Itertools};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use tokio::fs;
use toml::Table;
use typst_syntax::package::PackageManifest;

use crate::error::MultiplePreprocessorConfigError;
use crate::preprocessor::{BoxedPreprocessor, PreprocessorMap};

pub use error::*;

/// The complete prequery manifest as found in the `[tool.prequery]` section in `typst.toml`.
/// Usually, that section will be defined as multiple `[[tool.prequery.jobs]]` entries.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct PrequeryManifest {
    /// The preprocessing jobs to execute
    pub jobs: Vec<Job>,
}

/// A single preprocessing job. A job normally consists of executing the configured query and then
/// processing the result in some way, usually writing to files in the project root.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Job {
    /// The job's name (for human consumption, e.g. in logs)
    pub name: String,
    /// Identifier of the preprocessor that should be run
    pub kind: String,
    /// The query the preprocessor needs to run
    #[serde(default)]
    pub query: Query,
    /// Arbitrary additional manifest for the job
    #[serde(flatten)]
    pub manifest: Table,
}

/// Query configuration. All fields here are optional, as preprocessors can define their own
/// defaults.
#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// The selector to be queried, e.g. `<label>`
    pub selector: Option<String>,
    /// The field (`--field`) to be queried from the selector (with metadata elements, this is
    /// usually `value`)
    #[serde(default, deserialize_with = "deserialize_field")]
    pub field: Option<Option<String>>,
    /// Whether only one (`--one`) query result is expected and should be returned
    pub one: Option<bool>,
    /// Any additional inputs (`--input`) to be given to the queried document. Regardless of these
    /// settings, `prequery-fallback` is always set to `true` during queries.
    #[serde(default)]
    pub inputs: HashMap<String, String>,
}

impl PrequeryManifest {
    /// Given the contents of a `typst.toml` file, parses the `[tool.prequery]` section.
    pub fn parse(content: &str) -> Result<Self> {
        let mut config: PackageManifest = toml::from_str(content)?;
        let config = config
            .tool
            .sections
            .remove("prequery")
            .ok_or(Error::Missing)?
            .try_into::<Self>()
            .map_err(Error::from)?;
        Ok(config)
    }

    /// Resolves and reads the given `typst.toml` file.
    pub async fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = fs::read_to_string(path).await?;
        let config = Self::parse(&config)?;
        Ok(config)
    }

    /// Tries to configure all preprocessors in this manifest. Fails if any preprocessors can not be
    /// configured.
    pub fn get_preprocessors(
        self,
        preprocessors: &PreprocessorMap,
    ) -> Result<Vec<BoxedPreprocessor>, MultiplePreprocessorConfigError> {
        let (jobs, errors): (Vec<_>, Vec<_>) =
            self.jobs
                .into_iter()
                .partition_map(|job| match preprocessors.get(job) {
                    Ok(value) => Either::Left(value),
                    Err(err) => Either::Right(err),
                });

        if !errors.is_empty() {
            return Err(MultiplePreprocessorConfigError::new(errors));
        }

        Ok(jobs)
    }
}

/// Deserializes the `field` config: if given, must be either a string or `false`.
fn deserialize_field<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct FieldVisitor;

    impl Visitor<'_> for FieldVisitor {
        type Value = Option<Option<String>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("`false` or a string`")
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v {
                return Err(E::invalid_value(de::Unexpected::Bool(v), &self));
            }
            Ok(Some(None))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_string(v.to_owned())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(Some(v)))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(FieldVisitor)
}

mod error {
    use std::io;

    use thiserror::Error;

    /// Errors that can occur when reading a prequery manifest
    #[derive(Error, Debug)]
    pub enum Error {
        /// An I/O error occurred reading the typst.toml file
        #[error("typst.toml file could not be read")]
        Io(#[from] io::Error),
        /// The prequery section is missing in typst.toml
        #[error("typst.toml does not contain `tool.prequery` section")]
        Missing,
        /// The prequery section contains invalid config data
        #[error("typst.toml contains `tool.prequery` key, but it's not a valid preprocessor configuration")]
        Invalid(#[from] toml::de::Error),
    }

    /// Result type alias that defaults error to [enum@Error].
    pub type Result<T, E = Error> = std::result::Result<T, E>;
}
