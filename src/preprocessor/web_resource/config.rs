use std::path::PathBuf;

use serde::Deserialize;

/// Auxilliary configuration for the preprocessor
#[derive(Deserialize, Debug, Clone)]
pub struct Config {}

/// A resource that should be downloaded
#[derive(Deserialize, Debug, Clone)]
pub struct Resource {
    /// The URL to download from
    pub url: String,
    /// The path to download to. Must be in the document's root.
    pub path: PathBuf,
}

pub type QueryData = Vec<Resource>;
