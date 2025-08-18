use std::sync::Arc;

use crate::preprocessor::PreprocessorMap;

/// The context for executing preprocessors.
pub trait World: Send + Sync {
    /// Map of preprocessors existing in this World
    fn preprocessors(&self) -> &PreprocessorMap;
}

pub type DynWorld = Arc<dyn World>;

/// The default context, accessing the real web, filesystem, etc.
pub struct DefaultWorld {
    preprocessors: PreprocessorMap,
}

impl Default for DefaultWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultWorld {
    /// Creates the default world.
    pub fn new() -> Self {
        let mut preprocessors = PreprocessorMap::new();
        preprocessors.register(crate::web_resource::WebResourceFactory);
        Self { preprocessors }
    }
}

impl World for DefaultWorld {
    fn preprocessors(&self) -> &PreprocessorMap {
        &self.preprocessors
    }
}
