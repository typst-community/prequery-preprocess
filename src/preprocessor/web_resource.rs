//! The `web-resource` preprocessor

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use crate::args::CliArguments;
use crate::config;
use crate::query::Query;

use super::{BoxedPreprocessor, Preprocessor, PreprocessorDefinition};

/// Auxilliary configuration for the preprocessor
#[derive(Deserialize, Debug, Clone)]
pub struct Config {}

/// A resource that should be downloaded
#[derive(Deserialize, Debug, Clone)]
pub struct Resource {
    /// The URL to download from
    pub url: String,
    /// The path to download to. Must be in the document's root.
    pub path: String,
}

type QueryData = Vec<Resource>;

/// The `web-resource` preprocessor
pub struct WebResource<'a> {
    args: &'a CliArguments,
    _config: Config,
    query: Query,
}

impl WebResource<'_> {
    fn query(&self) -> Result<QueryData> {
        self.query.query(self.args)
    }
}

impl Preprocessor for WebResource<'_> {
    fn run(&mut self) -> Result<()> {
        let query_data = self.query()?;
        println!("{query_data:?}");
        Ok(())
    }
}

/// The `web-resource` preprocessor factory
pub struct WebResourceFactory;

impl WebResourceFactory {
    fn parse_config(config: toml::Table) -> Result<Config> {
        let config = config.try_into()
            .context("invalid web-resource configuration")?;
        Ok(config)
    }

    fn build_query(config: config::Query) -> Result<Query> {
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

    fn configure<'a>(
        args: &'a CliArguments,
        config: toml::Table,
        query: config::Query,
    ) -> Result<BoxedPreprocessor<'a>> {
        let _config = Self::parse_config(config)?;
        let query = Self::build_query(query)?;
        let instance = WebResource { args, _config, query };
        Ok(Box::new(instance))
    }
}
