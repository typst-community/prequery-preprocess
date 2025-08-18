use std::io;
use std::path::{self, Component, Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs;

use crate::args::{CliArguments, ARGS};
use crate::manifest::{self, PrequeryManifest};
use crate::preprocessor::PreprocessorMap;

/// The context for executing preprocessors.
#[async_trait]
pub trait World: Send + Sync {
    /// Map of preprocessors existing in this World
    fn preprocessors(&self) -> &PreprocessorMap;

    /// The arguments given to the invocation
    fn arguments(&self) -> &CliArguments;

    /// Returns the path of the `typst.toml` file that is closest to the input file.
    async fn resolve_typst_toml(&self) -> io::Result<PathBuf>;

    /// Reads the `typst.toml` file that is closest to the input file.
    async fn read_typst_toml(&self) -> manifest::Result<PrequeryManifest>;

    /// returns the root path. This is either the explicitly given root or the directory in which
    /// the input file is located. If the input file path only consists of a file name, the current
    /// directory (`"."`) is the root. In general, this function does not return an absolute path.
    fn resolve_root(&self) -> &Path;

    /// Resolve the virtual path relative to an actual file system root
    /// (where the project or package resides).
    ///
    /// Returns `None` if the path lexically escapes the root. The path might
    /// still escape through symlinks.
    fn resolve(&self, path: &Path) -> Option<PathBuf>;
}

pub type DynWorld = Arc<dyn World>;

/// The default context, accessing the real web, filesystem, etc.
pub struct DefaultWorld {
    preprocessors: PreprocessorMap,
}

impl Default for DefaultWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultWorld {
    /// Creates the default world.
    pub fn new() -> Self {
        let mut preprocessors = PreprocessorMap::new();
        preprocessors.register(crate::web_resource::WebResourceFactory);
        Self { preprocessors }
    }
}

#[async_trait]
impl World for DefaultWorld {
    fn preprocessors(&self) -> &PreprocessorMap {
        &self.preprocessors
    }

    fn arguments(&self) -> &CliArguments {
        &ARGS
    }

    async fn resolve_typst_toml(&self) -> io::Result<PathBuf> {
        const TYPST_TOML: &str = "typst.toml";

        let input = path::absolute(&self.arguments().input)?;
        let mut p = input.clone();

        // the input path needs to refer to a file. refer to typst.toml instead
        p.set_file_name(TYPST_TOML);
        // repeat as long as the path does not point to an accessible regular file
        while !fs::metadata(&p).await.is_ok_and(|m| m.is_file()) {
            // remove the file name
            let result = p.pop();
            assert!(
                result,
                "the path should have had a final component of `{TYPST_TOML}`"
            );
            // go one level up
            let result = p.pop();
            if !result {
                // if there is no level up, not typst.toml was found
                let input_str = input.to_string_lossy();
                let msg = format!("no {TYPST_TOML} file found for input file {input_str}");
                return Err(io::Error::new(io::ErrorKind::NotFound, msg));
            }
            // re-add the file name
            p.push(TYPST_TOML);
        }
        Ok(p)
    }

    async fn read_typst_toml(&self) -> manifest::Result<PrequeryManifest> {
        let typst_toml = self
            .resolve_typst_toml()
            .await
            .map_err(manifest::Error::from)?;
        let config = PrequeryManifest::read(typst_toml).await?;
        Ok(config)
    }

    fn resolve_root(&self) -> &Path {
        if let Some(root) = &self.arguments().root {
            // a root was explicitly given
            root
        } else if let Some(root) = self.arguments().input.parent() {
            // the root is the directory of the input file
            root
        } else {
            // the root is the directory of the input file, which is the current directory
            Path::new(".")
        }
    }

    fn resolve(&self, path: &Path) -> Option<PathBuf> {
        let root = self.resolve_root();
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
}
