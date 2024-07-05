//! The `web-resource` preprocessor

use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::args::CliArguments;
use crate::config;
use crate::query::Query;

use super::{Preprocessor, PreprocessorFactory, PreprocessorImpl};

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

/// The `web-resource` preprocessor
pub struct WebResource<'a> {
    args: &'a CliArguments,
    _config: Config,
    query: Query,
}

impl<'a> PreprocessorImpl<'a> for WebResource<'a> {
    const NAME: &'static str = "web-resource";

    type Config = Config;
    type QueryData = Vec<Resource>;

    fn factory() -> impl PreprocessorFactory + Send + Sync + 'static {
        WebResourceFactory
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

impl<'a> WebResource<'a> {
    fn query(&self) -> Result<<Self as PreprocessorImpl<'a>>::QueryData> {
        self.query.query(self.args)
    }
}

impl<'a> Preprocessor<'a> for WebResource<'a> {
    fn run(&mut self) -> Result<()> {
        let query_data = self.query()?;
        println!("{query_data:?}");
        Ok(())
    }
}

/// The `web-resource` preprocessor factory
pub struct WebResourceFactory;

impl PreprocessorFactory for WebResourceFactory {
    fn configure<'a>(
        &self,
        args: &'a CliArguments,
        config: toml::Table,
        query: config::Query,
    ) -> Result<Box<dyn Preprocessor<'a> + 'a>> {
        let _config = WebResource::parse_config(config)?;
        let query = WebResource::build_query(query)?;
        let instance = WebResource { args, _config, query };
        Ok(Box::new(instance))
    }
}
