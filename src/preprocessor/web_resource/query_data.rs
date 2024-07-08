use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A resource that should be downloaded
#[derive(Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Resource {
    /// The path to download to. Must be in the document's root.
    pub path: PathBuf,
    /// The URL to download from
    pub url: String,
}

pub type QueryData = Vec<Resource>;
