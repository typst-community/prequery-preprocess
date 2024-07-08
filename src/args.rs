//! CLI argument parsing types

use std::path::{self, Component, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use once_cell::sync::Lazy;
use tokio::fs;

/// Map of preprocessors defined in this crate
pub static ARGS: Lazy<CliArguments> = Lazy::new(CliArguments::parse);

/// prequery-preprocess args
#[derive(Parser, Debug, Clone, PartialEq, Eq)]
pub struct CliArguments {
    /// Configures the types executable to use
    #[clap(long, value_name = "EXE", default_value = "typst")]
    pub typst: PathBuf,

    /// Configures the project root (for absolute paths)
    #[clap(long = "root", value_name = "DIR", env = "TYPST_ROOT")]
    pub root: Option<PathBuf>,

    /// Path to input Typst file. `prequery-preprocess` will look for a `typst.toml` file in
    /// directories upwards from that file to determine queries.
    pub input: PathBuf,
}

impl CliArguments {
    /// Returns the path of the `typst.toml` file that is closest to the input file.
    pub async fn resolve_typst_toml(&self) -> Result<PathBuf> {
        const TYPST_TOML: &str = "typst.toml";

        let input = path::absolute(&self.input)
            .with_context(|| {
                let input_str = self.input.to_string_lossy();
                format!("cannot resolve {TYPST_TOML} because input file {input_str} can't be resolved")
            })?;
        let mut p = input.clone();

        // the input path needs to refer to a file. refer to typst.toml instead
        p.set_file_name(TYPST_TOML);
        // repeat as long as the path does not point to an accessible regular file
        while !fs::metadata(&p).await.map_or(false, |m| m.is_file()) {
            // remove the file name
            let result = p.pop();
            assert!(result, "the path should have had a final component of `{TYPST_TOML}`");
            // go one level up
            let result = p.pop();
            if !result {
                // if there is no level up, not typst.toml was found
                let input_str = input.to_string_lossy();
                return Err(anyhow!("no {TYPST_TOML} file was found in any ancestor directory of {input_str}"));
            }
            // re-add the file name
            p.push(TYPST_TOML);
        }
        Ok(p)
    }

    /// returns the root path. This is either the explicitly given root or the directory in which
    /// the input file is located. If the input file path only consists of a file name, the current
    /// directory (`"."`) is the root. In general, this function does not return an absolute path.
    pub fn resolve_root(&self) -> &Path {
        if let Some(root) = &self.root {
            // a root was explicitly given
            root
        } else if let Some(root) = self.input.parent() {
            // the root is the directory of the input file
            root
        } else {
            // the root is the directory of the input file, which is the current directory
            Path::new(".")
        }
    }

    /// Resolve the virtual path relative to an actual file system root
    /// (where the project or package resides).
    ///
    /// Returns `None` if the path lexically escapes the root. The path might
    /// still escape through symlinks.
    pub fn resolve(&self, path: &Path) -> Option<PathBuf> {
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