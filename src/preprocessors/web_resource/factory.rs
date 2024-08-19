use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

use crate::manifest;
use crate::preprocessor::{BoxedPreprocessor, PreprocessorDefinition};
use crate::query::Query;

use super::{Manifest, WebResource};

/// The `web-resource` preprocessor factory
#[derive(Debug, Clone, Copy)]
pub struct WebResourceFactory;

impl WebResourceFactory {
    fn parse_config(config: toml::Table) -> Result<Manifest> {
        let config = config
            .try_into()
            .context("invalid web-resource configuration")?;
        Ok(config)
    }

    fn build_query(config: manifest::Query) -> Result<Query> {
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
        query: manifest::Query,
    ) -> Result<BoxedPreprocessor> {
        let config = Self::parse_config(config)?;
        // index begins as None and is asynchronously populated later
        let index = None;
        let query = Self::build_query(query)?;
        let instance = WebResource::new(name, config, index, query);
        Ok(Box::new(Arc::new(instance)))
    }
}
