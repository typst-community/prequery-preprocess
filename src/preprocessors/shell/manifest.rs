use std::fmt;
use std::path::PathBuf;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};

/// Auxiliary configuration for the preprocessor
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// command and arguments to run with this shell preprocessor job
    pub command: Command,

    /// Whether each input should be process by its own command invocation, or all inputs should be
    /// joined and processed by a single command invocation.
    #[serde(default)]
    pub joined: bool,

    /// Whether command invocation can be run concurrently to each other. This only has an effect if
    /// inputs are not joined together for a single command invocation.
    #[serde(default)]
    pub concurrent: bool,

    /// Change this to true or a file path given as a string to enable the index. If true, the
    /// default path is "shell-index.toml"; note that if multiple shell jobs are using the same
    /// index file, this will lead to problems!
    #[serde(default, deserialize_with = "deserialize_index")]
    pub index: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command(pub Vec<String>);

/// Deserializes the `index` config: if given, must be either a boolean or string.
fn deserialize_index<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    struct IndexVisitor;

    impl Visitor<'_> for IndexVisitor {
        type Value = Option<PathBuf>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a boolean or string")
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.then(|| "shell-index.toml".into()))
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
            Ok(Some(v.into()))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(IndexVisitor)
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();
        if let Some(s) = iter.next() {
            write!(f, "{s}")?;
        }
        for s in iter {
            write!(f, " {s}")?;
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CommandVisitor;

        impl<'de> Visitor<'de> for CommandVisitor {
            type Value = Command;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or array of strings")
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
                Ok(Command(vec![v]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut result = Vec::new();
                while let Some(value) = seq.next_element()? {
                    result.push(value);
                }
                Ok(Command(result))
            }
        }

        deserializer.deserialize_any(CommandVisitor)
    }
}
