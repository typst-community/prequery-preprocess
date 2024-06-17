use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};
use serde::de::{self, Visitor};
use toml::Table;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub jobs: Vec<Job>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Job {
    pub name: String,
    pub kind: String,
    pub query: Query,
    #[serde(flatten)]
    pub config: Table,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Query {
    pub selector: Option<String>,
    #[serde(default, deserialize_with = "deserialize_field")]
    pub field: Option<Option<String>>,
    pub one: Option<bool>,
    #[serde(default)]
    pub inputs: HashMap<String, String>,
}

pub fn read_typst_toml(content: &str) -> Result<Config> {
    let mut config = toml::Table::from_str(content)?;
    let config = config
        .remove("tool")
        .context("typst.toml does not contain `tool` section")?
        .try_into::<Table>()
        .context("typst.toml contains `tool` key, but it's not a table")?
        .remove("prequery")
        .context("typst.toml does not contain `tool.prequery` section")?
        .try_into::<Config>()
        .context("typst.toml contains `tool.prequery` key, but it's not a valid prequery configuration")?;
    Ok(config)
}

fn deserialize_field<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>
{
    struct FieldVisitor;

    impl<'de> Visitor<'de> for FieldVisitor {
        type Value = Option<Option<String>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("`false` or a string`")
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v {
                return Err(E::invalid_value(de::Unexpected::Bool(v), &self));
            }
            Ok(Some(None))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_string(v.to_owned())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(Some(v)))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(FieldVisitor)
}
