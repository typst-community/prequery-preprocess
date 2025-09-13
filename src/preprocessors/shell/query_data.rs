use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, Visitor};

#[derive(Debug, Clone, PartialEq)]
pub enum InputItem {
    Path(PathBuf),
    Data(serde_json::Value),
    PathWithData(PathBuf, serde_json::Value),
}

impl InputItem {
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            InputItem::Path(path_buf) => Some(path_buf),
            InputItem::Data(_) => None,
            InputItem::PathWithData(path_buf, _) => Some(path_buf),
        }
    }

    pub fn data(&self) -> Option<&serde_json::Value> {
        match self {
            InputItem::Path(_) => None,
            InputItem::Data(data) => Some(data),
            InputItem::PathWithData(_, data) => Some(data),
        }
    }

    pub fn into_path(self) -> Option<PathBuf> {
        match self {
            InputItem::Path(path_buf) => Some(path_buf),
            InputItem::Data(_) => None,
            InputItem::PathWithData(path_buf, _) => Some(path_buf),
        }
    }

    pub fn into_data(self) -> Option<serde_json::Value> {
        match self {
            InputItem::Path(_) => None,
            InputItem::Data(data) => Some(data),
            InputItem::PathWithData(_, data) => Some(data),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
pub struct QueryData {
    pub items: Vec<InputItem>,
}

impl<'de> Deserialize<'de> for InputItem {
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
            type Value = InputItem;

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
                    (Some(path), None) => InputItem::Path(path),
                    (None, Some(data)) => InputItem::Data(data),
                    (Some(path), Some(data)) => InputItem::PathWithData(path, data),
                    (None, None) => {
                        return Err(de::Error::missing_field("data"));
                    }
                };
                Ok(item)
            }
        }

        deserializer.deserialize_map(FieldVisitor)
    }
}
