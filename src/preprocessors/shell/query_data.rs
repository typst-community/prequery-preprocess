use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, Visitor};

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct InputItem {
    path: PathBuf,
    data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryData {
    SharedOutput {
        path: PathBuf,
        inputs: Vec<serde_json::Value>,
    },
    IndividualOutput(Vec<InputItem>),
}

impl<'de> Deserialize<'de> for QueryData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, PartialEq)]
        pub enum FirstItem {
            Path(PathBuf),
            PathWithData(PathBuf, serde_json::Value),
        }

        #[derive(Deserialize, Debug, Clone, PartialEq)]
        pub struct DataItem {
            data: serde_json::Value,
        }

        impl<'de> Deserialize<'de> for FirstItem {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                #[derive(Deserialize)]
                #[serde(field_identifier, rename_all = "lowercase")]
                enum Field {
                    Path,
                    Data,
                }

                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = FirstItem;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a map containing path, data, or both")
                    }

                    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
                    where
                        V: MapAccess<'de>,
                    {
                        let mut path = None;
                        let mut data = None;
                        while let Some(key) = map.next_key()? {
                            match key {
                                Field::Path => {
                                    if path.is_some() {
                                        return Err(de::Error::duplicate_field("path"));
                                    }
                                    path = Some(map.next_value()?);
                                }
                                Field::Data => {
                                    if data.is_some() {
                                        return Err(de::Error::duplicate_field("data"));
                                    }
                                    data = Some(map.next_value()?);
                                }
                            }
                        }
                        let item = match (path, data) {
                            (Some(path), None) => FirstItem::Path(path),
                            (Some(path), Some(data)) => FirstItem::PathWithData(path, data),
                            (None, _) => {
                                return Err(de::Error::missing_field("path"));
                            }
                        };
                        Ok(item)
                    }
                }

                deserializer.deserialize_map(FieldVisitor)
            }
        }

        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = QueryData;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence containing a path followed by data, entries of both path and data each")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(first) = seq.next_element::<FirstItem>()? else {
                    return Ok(QueryData::IndividualOutput(Vec::new()));
                };

                match first {
                    FirstItem::Path(path) => {
                        let mut inputs = match seq.size_hint() {
                            Some(size) => Vec::with_capacity(size),
                            None => Vec::new(),
                        };
                        while let Some(item) = seq.next_element::<DataItem>()? {
                            inputs.push(item.data);
                        }
                        Ok(QueryData::SharedOutput { path, inputs })
                    }
                    FirstItem::PathWithData(path, data) => {
                        let mut items = match seq.size_hint() {
                            Some(size) => Vec::with_capacity(size + 1),
                            None => Vec::new(),
                        };
                        items.push(InputItem { path, data });
                        while let Some(item) = seq.next_element()? {
                            items.push(item);
                        }
                        Ok(QueryData::IndividualOutput(items))
                    }
                }
            }
        }

        deserializer.deserialize_seq(FieldVisitor)
    }
}
