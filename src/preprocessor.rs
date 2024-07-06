//! The actual preprocessors and management of those

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

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

/// A factory for creating [Preprocessor]s
pub trait PreprocessorFactory {
    /// The identifier of the preprocessor, referenced by the [config::Job::kind] field
    fn name(&self) -> &'static str;

    /// Creates the preprocessor. The configuration is checked for validity, but no processing is
    /// done yet.
    fn configure<'a>(
        &self,
        args: &'a CliArguments,
        config: toml::Table,
        query: config::Query,
    ) -> Result<BoxedPreprocessor<'a>>;
}

/// A dynamically dispatched, boxed preprocessor factory
pub type BoxedPreprocessorFactory = Box<dyn PreprocessorFactory + Send + Sync>;

type PreprocessorMap = HashMap<&'static str, BoxedPreprocessorFactory>;

/// Map of preprocessor factories defined in and known to this crate
pub static PREPROCESSORS: Lazy<PreprocessorMap> = Lazy::new(|| {
    fn register<T>(map: &mut PreprocessorMap, factory: T)
    where
        T: PreprocessorFactory + Send + Sync + 'static
    {
        map.insert(factory.name(), Box::new(factory));
    }

    let mut map = HashMap::new();
    register(&mut map, web_resource::WebResourceFactory);
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
