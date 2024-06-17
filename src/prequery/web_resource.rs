use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::config;
use crate::query::Query;

use super::Prequery;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {}

#[derive(Deserialize, Debug, Clone)]
pub struct Resource {
    pub url: String,
    pub path: String,
}

pub struct WebResource;

impl Prequery for WebResource {
    type Config = Config;
    type QueryData = Vec<Resource>;

    fn build_query(&self, config: config::Query) -> Result<Query> {
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