//! CLI argument parsing types

use std::path::{Component, Path, PathBuf};

use clap::Parser;

/// prequery-preprocess args
#[derive(Debug, Clone, Parser)]
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