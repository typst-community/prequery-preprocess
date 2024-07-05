//! The `web-resource` prequery

use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::args::CliArguments;
use crate::config;
use crate::query::Query;

use super::{Prequery, PrequeryFactory, PrequeryImpl};

/// Auxilliary configuration for the prequery
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

/// The `web-resource` prequery
pub struct WebResource<'a> {
    args: &'a CliArguments,
    _config: Config,
    query: Query,
}

impl<'a> PrequeryImpl<'a> for WebResource<'a> {
    const NAME: &'static str = "web-resource";

    type Config = Config;
    type QueryData = Vec<Resource>;

    fn factory() -> impl PrequeryFactory + Send + Sync + 'static {
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
    fn query(&self) -> Result<<Self as PrequeryImpl<'a>>::QueryData> {
        self.query.query(self.args)
    }
}

impl<'a> Prequery<'a> for WebResource<'a> {
    fn run(&mut self) -> Result<()> {
        let query_data = self.query()?;
        println!("{query_data:?}");
        Ok(())
    }
}

/// The `web-resource` prequery factory
pub struct WebResourceFactory;

impl PrequeryFactory for WebResourceFactory {
    fn configure<'a>(
        &self,
        args: &'a CliArguments,
        config: toml::Table,
        query: config::Query,
    ) -> Result<Box<dyn Prequery<'a> + 'a>> {
        let _config = WebResource::parse_config(config)?;
        let query = WebResource::build_query(query)?;
        let instance = WebResource { args, _config, query };
        Ok(Box::new(instance))
    }
}
