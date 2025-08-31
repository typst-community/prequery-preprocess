use std::borrow::Cow;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::manifest;
use crate::preprocessor::{BoxedPreprocessor, PreprocessorDefinition};
use crate::query::Query;

use super::world::{DefaultWorld, World};
use super::{Manifest, ManifestError, ManifestResult, QueryConfigError, WebResource};

/// The `web-resource` preprocessor factory
#[derive(Debug, Clone, Copy)]
pub struct WebResourceFactory<W> {
    _w: PhantomData<W>,
}

impl Default for WebResourceFactory<DefaultWorld> {
    fn default() -> Self {
        Self::new()
    }
}

impl<W: World> WebResourceFactory<W> {
    /// Creates a factory with the given world.
    pub fn new() -> Self {
        Self { _w: PhantomData }
    }

    fn parse_config(config: toml::Table) -> ManifestResult<Manifest> {
        let config = config.try_into()?;
        Ok(config)
    }

    fn build_query(config: manifest::Query) -> ManifestResult<Query> {
        let config = Query::builder()
            .default_field(Some("value".to_string()))
            .default_one(false)
            .default_selector("<web-resource>".to_string())
            .build(config)
            .map_err(QueryConfigError::Builder)?;
        if config.one {
            return Err(QueryConfigError::One.into());
        }

        Ok(config)
    }
}

impl<W: World> PreprocessorDefinition<W::MainWorld> for WebResourceFactory<W> {
    type Error = ManifestError;

    fn name(&self) -> Cow<'static, str> {
        "web-resource".into()
    }

    fn configure(
        &self,
        world: &Arc<W::MainWorld>,
        name: String,
        config: toml::Table,
        query: manifest::Query,
    ) -> ManifestResult<BoxedPreprocessor<W::MainWorld>> {
        let world = Arc::new(W::new(world.clone()));
        let config = Self::parse_config(config)?;
        // index begins as None and is asynchronously populated later
        let index = None;
        let query = Self::build_query(query)?;
        let instance = WebResource::new(world, name, config, index, query);
        Ok(Box::new(Arc::new(instance)))
    }
}
