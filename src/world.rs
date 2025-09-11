//! Abstracts an execution environment preprocessors can run in.
//! The world mediates access to the file system, the network, and more high-level resources
//! such as the project manifest

use std::fmt::Write;
use std::io;
use std::path::{self, Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use clap::Parser;
use itertools::{Either, Itertools};
use serde::Deserialize;
use tokio::fs;
use tokio::process::Command;

use crate::args::CliArguments;
use crate::error::MultiplePreprocessorConfigError;
use crate::manifest::{self, PrequeryManifest};
use crate::preprocessor::{BoxedPreprocessor, PreprocessorMap};
use crate::query::{self, Query};
use crate::reporting::Log;

/// The context for executing preprocessors.
#[cfg_attr(feature = "test", mockall::automock(type Logger = crate::test_utils::VecLog;))]
#[async_trait]
pub trait World: Send + Sync + 'static {
    /// The Logger type used by this world
    type Logger: Log;

    /// Map of preprocessors existing in this World
    fn preprocessors(&self) -> &PreprocessorMap<Self>;

    /// The arguments given to the invocation
    fn arguments(&self) -> &CliArguments;

    /// The log to which to write progress updates and errors.
    /// This method returns an owned value; usually it will actually be a _handle_ to the actual
    /// logger.
    fn log(&self) -> Self::Logger;

    /// Reads the `typst.toml` file that is closest to the input file.
    async fn read_typst_toml(&self) -> manifest::Result<PrequeryManifest>;

    /// Executes the query. This builds the necessary command line, runs the command, and returns
    /// the command's stdout.
    async fn query_impl(&self, query: &Query) -> query::Result<Vec<u8>>;
}

/// The context for executing preprocessors; provided methods that don't need to be customized
/// between environments.
#[async_trait]
pub trait WorldExt: World {
    /// returns the root path. This is either the explicitly given root or the directory in which
    /// the input file is located. If the input file path only consists of a file name, the current
    /// directory (`"."`) is the root. In general, this function does not return an absolute path.
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

    /// Resolve the virtual path relative to an actual file system root
    /// (where the project or package resides).
    ///
    /// Returns `None` if the path lexically escapes the root. The path might
    /// still escape through symlinks.
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
                    let result = out.pop();
                    if !result || out.as_os_str().len() < root_len {
                        return None;
                    }
                }
                Component::Normal(_) => out.push(component),
            }
        }
        Some(out)
    }

    /// Tries to configure all preprocessors in this manifest. Fails if any preprocessors can not be
    /// configured.
    fn get_preprocessors(
        self: &Arc<Self>,
        manifest: PrequeryManifest,
    ) -> Result<Vec<BoxedPreprocessor<Self>>, MultiplePreprocessorConfigError>
    where
        Self: Sized,
    {
        let (jobs, errors): (Vec<_>, Vec<_>) = manifest.jobs.into_iter().partition_map(|job| {
            match self.preprocessors().get(self, job) {
                Ok(value) => Either::Left(value),
                Err(err) => Either::Right(err),
            }
        });

        if !errors.is_empty() {
            return Err(MultiplePreprocessorConfigError::new(errors));
        }

        Ok(jobs)
    }

    /// Executes the query. This builds the necessary command line, runs the command, and returns
    /// the result parsed into the desired type from JSON.
    async fn query<T>(&self, query: &Query) -> query::Result<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        let output = self.query_impl(query).await?;
        let value = serde_json::from_slice(&output)?;
        Ok(value)
    }
}

#[async_trait]
impl<T: World> WorldExt for T {}

/// The default context, accessing the real web, filesystem, etc.
pub struct DefaultWorld {
    preprocessors: PreprocessorMap<Self>,
    arguments: CliArguments,
}

impl Default for DefaultWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultWorld {
    /// Creates the default world.
    pub fn new() -> Self {
        let mut preprocessors = PreprocessorMap::default();
        preprocessors.register(crate::web_resource::WebResourceFactory::default());
        let arguments = CliArguments::parse();
        Self {
            preprocessors,
            arguments,
        }
    }

    /// Returns the path of the `typst.toml` file that is closest to the input file.
    pub async fn resolve_typst_toml(&self) -> io::Result<PathBuf> {
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
}

#[async_trait]
impl World for DefaultWorld {
    type Logger = io::Stderr;

    fn preprocessors(&self) -> &PreprocessorMap<Self> {
        &self.preprocessors
    }

    fn arguments(&self) -> &CliArguments {
        &self.arguments
    }

    fn log(&self) -> Self::Logger {
        io::stderr()
    }

    async fn read_typst_toml(&self) -> manifest::Result<PrequeryManifest> {
        let typst_toml = self
            .resolve_typst_toml()
            .await
            .map_err(manifest::Error::from)?;
        let config = fs::read_to_string(typst_toml).await?;
        let config = PrequeryManifest::parse(&config)?;
        Ok(config)
    }

    async fn query_impl(&self, query: &Query) -> query::Result<Vec<u8>> {
        let mut cmd = Command::new(&self.arguments().typst);
        cmd.arg("query");
        if let Some(root) = &self.arguments().root {
            cmd.arg("--root").arg(root);
        }
        if let Some(field) = &query.field {
            cmd.arg("--field").arg(field);
        }
        if query.one {
            cmd.arg("--one");
        }
        let mut input = String::new();
        for (key, value) in &query.inputs {
            input.clear();
            write!(&mut input, "{key}={value}").expect("writing to a string failed");
            cmd.arg("--input").arg(&input);
        }
        cmd.arg("--input").arg("prequery-fallback=true");
        cmd.arg(&self.arguments().input).arg(&query.selector);

        cmd.stderr(Stdio::inherit());
        let output = cmd.output().await?;
        if !output.status.success() {
            let command = Box::new(cmd);
            let status = output.status;
            Err(query::Error::Failure { command, status })?;
        }

        Ok(output.stdout)
    }
}
