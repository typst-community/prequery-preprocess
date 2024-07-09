//! The `web-resource` preprocessor

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use tokio::{fs, sync::Mutex};
use tokio::task::JoinSet;
use tokio::io::AsyncWriteExt;

use crate::args::ARGS;
use crate::config:: Query as ConfigQuery;
use crate::query::Query;

use super::{BoxedPreprocessor, Preprocessor, PreprocessorDefinition};

mod config;
mod index;
mod query_data;

use config::*;
use index::*;
use query_data::*;

/// The `web-resource` preprocessor
#[derive(Debug)]
pub struct WebResource {
    name: String,
    config: Config,
    index: Option<Mutex<Index>>,
    query: Query,
}

impl WebResource {
    async fn populate_index(&mut self) -> Result<()> {
        if let Some(location) = self.config.resolve_index_path().await {
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

    async fn query(&self) -> Result<QueryData> {
        self.query.query().await
    }

    async fn download(self: Arc<Self>, resource: Resource) -> Result<()> {
        let name = self.name();
        let Resource { url, path } = &resource;

        let resolved_path = ARGS.resolve(path)
            .with_context(|| {
                let path_str = path.to_string_lossy();
                format!("[{name}] cannot download to {path_str} because it is outside the project root")
            })?;
        let path_str = resolved_path.to_string_lossy();

        let exists = fs::try_exists(&resolved_path).await.unwrap_or(false);
        let download = if !exists {
            println!("[{name}] Downloading {url} to {path_str}...");
            true
        } else if self.config.overwrite {
            println!("[{name}] Downloading {url} to {path_str} (overwrite of existing files was forced)...");
            true
        } else if let Some(index) = &self.index {
            let index = index.lock().await;
            if index.is_up_to_date(path, url) {
                println!("[{name}] Downloading of {url} to {path_str} skipped (file exists)");
                false
            } else {
                println!("[{name}] Downloading {url} to {path_str} (URL has changed)...");
                true
            }
        } else {
            println!("[{name}] Downloading of {url} to {path_str} skipped (file exists)");
            false
        };

        if download {
            if let Some(parent) = resolved_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            let mut response = reqwest::get(url).await?.error_for_status()?;
            let mut file = fs::File::create(&resolved_path).await?;
            while let Some(chunk) = response.chunk().await? {
                file.write_all(&chunk).await?;
            }
            file.flush().await?;

            if let Some(index) = &self.index {
                let mut index = index.lock().await;
                index.update(resource.clone());
            }
            println!("[{name}] Downloading {url} to {path_str} finished");
        }

        Ok(())
    }
}

#[async_trait]
impl Preprocessor for Arc<WebResource> {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&mut self) -> Result<()> {
        Arc::get_mut(self)
            .expect("web-resource ref count should be one before starting the processing")
            .populate_index().await?;

        let query_data = self.query().await?;

        let mut set = JoinSet::new();
        for (path, url) in query_data.resources {
            set.spawn(Arc::clone(self).download(Resource { path, url }));
        }

        let mut success = true;
        while let Some(result) = set.join_next().await {
            let result = result?;
            success &= result.is_ok();
        }

        if let Some(index) = &self.index {
            let index = index.lock().await;
            index.write().await?;
        }

        success.then_some(()).ok_or(anyhow!("at least one download failed"))
    }
}

/// The `web-resource` preprocessor factory
#[derive(Debug, Clone, Copy)]
pub struct WebResourceFactory;

impl WebResourceFactory {
    fn parse_config(config: toml::Table) -> Result<Config> {
        let config = config.try_into()
            .context("invalid web-resource configuration")?;
        Ok(config)
    }

    fn build_query(config: ConfigQuery) -> Result<Query> {
        let config = Query::builder()
            .default_field(Some("value".to_string()))
            .default_one(false)
            .default_selector("<web-resource>".to_string())
            .build(config)?;
        if config.one {
            return Err(anyhow!("web-resource prequery does not support --one"));
        }

        Ok(config)
    }
}

impl PreprocessorDefinition for WebResourceFactory {
    const NAME: &'static str = "web-resource";

    fn configure(
        name: String,
        config: toml::Table,
        query: ConfigQuery,
    ) -> Result<BoxedPreprocessor> {
        let config = Self::parse_config(config)?;
        // index begins as None and is asynchronously populated later
        let index = None;
        let query = Self::build_query(query)?;
        let instance = WebResource { name, index, config, query };
        Ok(Box::new(Arc::new(instance)))
    }
}
