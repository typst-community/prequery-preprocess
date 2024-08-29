//! The `web-resource` preprocessor

use std::io;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::args::ARGS;
use crate::preprocessor::{self, Preprocessor};
use crate::query::{self, Query};
use crate::utils;

mod error;
mod factory;
mod index;
mod manifest;
mod query_data;

use index::*;
use manifest::*;
use query_data::*;

pub use error::*;
pub use factory::WebResourceFactory;

/// The `web-resource` preprocessor
#[derive(Debug)]
pub struct WebResource {
    name: String,
    manifest: Manifest,
    index: Option<Mutex<Index>>,
    query: Query,
}

/// The state of the file: if and how the existing file corresponds to the desired web resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceState {
    /// No local file exists.
    Missing,
    /// A re-download is forced despite the file existing.
    Forced,
    /// The file seems to be up-to-date: the URL hasn't changed, or no index is kept.
    Existing,
    /// The file seems is not up-to-date: the URL has changed according to the index.
    ChangedResource,
}

impl ResourceState {
    pub fn download(self) -> bool {
        match self {
            Self::Missing | Self::Forced | Self::ChangedResource => true,
            Self::Existing => false,
        }
    }

    pub fn reason(self) -> Option<&'static str> {
        match self {
            Self::Missing => None,
            Self::Forced => Some("overwrite of existing files was forced"),
            Self::ChangedResource => Some("URL has changed"),
            Self::Existing => Some("file exists"),
        }
    }

    fn print_reason(self) {
        if let Some(msg) = self.reason() {
            print!(" ({msg})");
        }
    }

    pub fn print(self, name: &str, url: &str, path: &str) {
        if self.download() {
            print!("[{name}] Downloading {url} to {path}");
            self.print_reason();
            println!("...");
        } else {
            print!("[{name}] Downloading of {url} to {path} skipped");
            self.print_reason();
            println!();
        }
    }
}

impl WebResource {
    pub(crate) fn new(
        name: String,
        manifest: Manifest,
        index: Option<Mutex<Index>>,
        query: Query,
    ) -> Self {
        Self {
            name,
            index,
            manifest,
            query,
        }
    }

    async fn populate_index(&mut self) -> Result<(), IndexError> {
        if let Some(location) = self.manifest.resolve_index_path().await {
            // an index is in use
            let location = location?;
            let index = if fs::try_exists(&location).await.unwrap_or(false) {
                // read the existing index
                Index::read(location).await?
            } else {
                // generate an empty index
                Index::new(location)
            };

            self.index = Some(Mutex::new(index));
        } else {
            // no index is in use
            self.index = None;
        }

        Ok(())
    }

    async fn query(&self) -> query::Result<QueryData> {
        let data = self.query.query().await?;
        Ok(data)
    }

    async fn download(self: Arc<Self>, resource: Resource) -> Result<(), DownloadError> {
        let name = self.name();
        let Resource { url, path } = &resource;

        let resolved_path = ARGS.resolve(path).ok_or_else(|| {
            let path_str = path.to_string_lossy();
            let msg = format!("{path_str} is outside the project root");
            io::Error::new(io::ErrorKind::PermissionDenied, msg)
        })?;
        let path_str = resolved_path.to_string_lossy();

        let exists = fs::try_exists(&resolved_path).await.unwrap_or(false);
        let state = if !exists {
            ResourceState::Missing
        } else if self.manifest.overwrite {
            ResourceState::Forced
        } else if let Some(index) = &self.index {
            let index = index.lock().await;
            if index.is_up_to_date(path, url) {
                ResourceState::Existing
            } else {
                ResourceState::ChangedResource
            }
        } else {
            ResourceState::Existing
        };

        state.print(name, url, &path_str);

        if state.download() {
            let result = self.do_download(&resolved_path, url).await;
            match &result {
                Ok(()) => {
                    if let Some(index) = &self.index {
                        let mut index = index.lock().await;
                        index.update(resource.clone());
                    }
                    println!("[{name}] Downloading {url} to {path_str} finished");
                }
                Err(error) => {
                    println!("[{name}] Downloading {url} to {path_str} failed: {error:?}");
                }
            }
            result?;
        }

        Ok(())
    }

    async fn do_download(&self, resolved_path: &Path, url: &String) -> Result<(), DownloadError> {
        if let Some(parent) = resolved_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut response = reqwest::get(url).await?.error_for_status()?;
        let mut file = fs::File::create(&resolved_path).await?;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(())
    }

    async fn run_impl(self: &mut Arc<WebResource>) -> ExecutionResult<()> {
        Arc::get_mut(self)
            .expect("web-resource ref count should be one before starting the processing")
            .populate_index()
            .await?;

        let downloads = self
            .query()
            .await?
            .resources
            .into_iter()
            .map(|(path, url)| Arc::clone(self).download(Resource { path, url }));
        let errors = utils::spawn_set(downloads).await;

        if let Some(index) = &self.index {
            let index = index.lock().await;
            index.write().await?;
        }

        if !errors.is_empty() {
            return Err(error::MultipleDownloadError::new(errors).into());
        }

        Ok::<_, ExecutionError>(())
    }
}

#[async_trait]
impl Preprocessor for Arc<WebResource> {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&mut self) -> preprocessor::ExecutionResult<()> {
        self.run_impl()
            .await
            .map_err(preprocessor::ExecutionError::new)?;
        Ok(())
    }
}
