use std::path::{Component, Path, PathBuf};

use anyhow::Result;
use serde::Deserialize;

use crate::args::CliArguments;
use crate::config;
use crate::query;

pub mod web_resource;

pub trait Prequery {
    type Config: for<'a> Deserialize<'a>;
    type QueryData: for<'a> Deserialize<'a>;

    fn build_query(&self, config: config::Query) -> Result<query::Query>;

    fn query<'a>(&self, args: &CliArguments, config: config::Query) -> Result<Self::QueryData> {
        let config = self.build_query(config)?;
        let data = query::query(args, &config)?;
        Ok(data)
    }
}

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
