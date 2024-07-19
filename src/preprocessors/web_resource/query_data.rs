use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use serde::de::{self, Deserializer, Error, Unexpected, Visitor};
use serde::Deserialize;

use super::Resource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryData {
    pub resources: BTreeMap<PathBuf, String>,
}

impl<'de> Deserialize<'de> for QueryData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = BTreeMap<PathBuf, String>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter
                    .write_str("a URL not conflicting with earlier resources for the same path")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut resources = Self::Value::new();
                while let Some(Resource { path, url }) = seq.next_element()? {
                    let entry = resources.entry(path);
                    match entry {
                        Entry::Occupied(entry) => {
                            // the entry is either ok, or we error here
                            if entry.get().as_str() != url {
                                return Err(Error::invalid_value(
                                    Unexpected::Str(entry.get()),
                                    &self,
                                ));
                            }
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(url);
                        }
                    }
                }
                Ok(resources)
            }
        }

        deserializer
            .deserialize_seq(FieldVisitor)
            .map(|resources| Self { resources })
    }
}
