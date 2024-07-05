//! The actual prequeries and management of those

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use anyhow::Result;
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::args::CliArguments;
use crate::config;
use crate::query;

pub mod web_resource;

/// Implementation part of a prequery
pub trait PrequeryImpl {
    /// The identifier of the prequery, referenced by the [config::Job::kind] field
    const NAME: &'static str;

    /// The type of configuration data stored in the [config::Job::config] field
    type Config: for<'a> Deserialize<'a>;
    /// The data returned when querying the document for this prequery
    type QueryData: for<'a> Deserialize<'a>;

    /// Build the query, usually using a [query::QueryBuilder] and optionally doing validation
    /// afterwards
    fn build_query(&self, config: config::Query) -> Result<query::Query>;

    /// Executes the query defined by [PrequeryImpl::build_query()]
    fn query(&self, args: &CliArguments, config: config::Query) -> Result<Self::QueryData> {
        let config = self.build_query(config)?;
        let data = query::query(args, &config)?;
        Ok(data)
    }
}

/// Outward-facing part of a prequery: this trait is object safe and simply allows executing the
/// prequery.
pub trait Prequery {
    /// Runs the prequery
    fn execute(&self, args: &CliArguments, config: config::Query) -> Result<()>;
}

type PrequeryMap = HashMap<&'static str, Box<dyn Prequery + Send + Sync>>;
/// Map of prequeries defined and known to this crate
pub static PREQUERIES: Lazy<PrequeryMap> = Lazy::new(|| {
    fn register<T: Prequery + PrequeryImpl + Send + Sync + 'static>(
        map: &mut PrequeryMap,
        prequery: T,
    ) {
        map.insert(T::NAME, Box::new(prequery));
    }

    let mut map = HashMap::new();
    register(&mut map, web_resource::WebResource);
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
