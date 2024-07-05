//! The actual preprocessors and management of those

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::args::CliArguments;
use crate::config;
use crate::query;

pub mod web_resource;

/// Implementation part of a preprocessor
pub trait PreprocessorImpl<'a> {
    /// The identifier of the preprocessor, referenced by the [config::Job::kind] field
    const NAME: &'static str;

    /// The factory that creates this preprocessor
    fn factory() -> impl PreprocessorFactory + Send + Sync + 'static;

    /// The type of configuration data stored in the [config::Job::config] field
    type Config: for<'b> Deserialize<'b>;
    /// The data returned when querying the document for this preprocessor
    type QueryData: for<'b> Deserialize<'b>;

    /// parse this preprocessor's config from an untyped table
    fn parse_config(config: toml::Table) -> Result<Self::Config> {
        let config = config.try_into()
            .context("invalid web-resource configuration")?;
        Ok(config)
    }

    /// Build the query, usually using a [query::QueryBuilder] and optionally doing validation
    /// afterwards
    fn build_query(config: config::Query) -> Result<query::Query>;
}

/// A configured preprocessor that can be executed for its side effect
pub trait Preprocessor<'a> {
    /// Executes this preprocessor
    fn run(&mut self) -> Result<()>;
}

/// A factory for creating [Preprocessor]s
pub trait PreprocessorFactory {
    /// Creates the preprocessor. The configuration is checked for validity, but no processing is
    /// done yet.
    fn configure<'a>(
        &self,
        args: &'a CliArguments,
        config: toml::Table,
        query: config::Query,
    ) -> Result<Box<dyn Preprocessor<'a> + 'a>>;
}

type PreprocessorMap = HashMap<&'static str, Box<dyn PreprocessorFactory + Send + Sync>>;
/// Map of preprocessors defined and known to this crate
pub static PREPROCESSORS: Lazy<PreprocessorMap> = Lazy::new(|| {
    fn register<'a, T>(map: &mut PreprocessorMap)
    where
        T: PreprocessorImpl<'a> + Send + Sync + 'static
    {
        map.insert(T::NAME, Box::new(T::factory()));
    }

    let mut map = HashMap::new();
    register::<web_resource::WebResource>(&mut map);
    map
});

/// Resolve the virtual path relative to an actual file system root
/// (where the project or package resides).
///
/// Returns `None` if the path lexically escapes the root. The path might
/// still escape through symlinks.
pub fn resolve(path: &Path, root: &Path) -> Option<PathBuf> {
    let root_len = root.as_os_str().len();
    let mut out = root.to_path_buf();
    for component in path.components() {
        match component {
            Component::Prefix(_) => {}
            Component::RootDir => {}
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
                if out.as_os_str().len() < root_len {
                    return None;
                }
            }
            Component::Normal(_) => out.push(component),
        }
    }
    Some(out)
}
