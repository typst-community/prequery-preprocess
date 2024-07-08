use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Represents an index of resources.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Index {
    /// a file format version number. Should be 1.
    pub version: usize,
    /// The entries in the index.
    #[serde(
        default, rename = "resource",
        serialize_with = "serialize_entries",
        deserialize_with = "deserialize_entries",
        skip_serializing_if = "BTreeMap::is_empty",
    )]
    pub entries: BTreeMap<PathBuf, Resource>,
}

/// A resource that should be downloaded
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Resource {
    /// The path to download to. Must be in the document's root.
    pub path: PathBuf,
    /// The URL to download from.
    pub url: String,
}

impl Index {
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: BTreeMap::new(),
        }
    }

    /// Reads an index from a file.
    pub async fn read<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let index = fs::read_to_string(file_path).await?;
        let index: Self = toml::from_str(&index)?;
        if index.version != 1 {
            return Err(anyhow!("index file version number was not 1"));
        }
        Ok(index)
    }

    /// Writes the index to a file.
    pub async fn write<P: AsRef<Path>>(&self, file_path: P) -> Result<()> {
        let mut file = fs::File::create(file_path).await?;
        let index = toml::to_string(self)?;
        file.write_all(index.as_bytes()).await?;
        Ok(())
    }
}

fn serialize_entries<S>(map: &BTreeMap<PathBuf, Resource>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer
{
    serializer.collect_seq(map.values())
}

/// Deserializes the `field` config: if given, must be either a string or `false`.
fn deserialize_entries<'de, D>(deserializer: D) -> Result<BTreeMap<PathBuf, Resource>, D::Error>
where
    D: Deserializer<'de>
{
    struct FieldVisitor;

    impl<'de> Visitor<'de> for FieldVisitor {
        type Value = BTreeMap<PathBuf, Resource>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("`false` or a string`")
        }

        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>, {
            let mut entries = BTreeMap::new();
            while let Some(elem) = seq.next_element::<Resource>()? {
                entries.insert(elem.path.to_owned(), elem);
            }
            Ok(entries)
        }
    }

    deserializer.deserialize_any(FieldVisitor)
}
