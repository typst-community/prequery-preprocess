use anyhow::Result;
use serde::Deserialize;

use crate::args::CliArguments;
use crate::config;
use crate::query;

pub mod web_resource;

pub trait Prequery {
    type Config: for<'a> Deserialize<'a>;
    type QueryData: for<'a> Deserialize<'a>;

    fn build_query(&self, config: config::Query) -> Result<query::Query>;

    fn query<'a>(&self, args: &CliArguments, config: config::Query) -> Result<Self::QueryData> {
        let config = self.build_query(config)?;
        let data = query::query(args, &config)?;
        Ok(data)
    }
}