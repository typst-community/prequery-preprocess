use std::collections::HashMap;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use crate::args::CliArguments;
use crate::config;

#[derive(Debug, Clone)]
pub struct Query {
    pub selector: String,
    pub field: Option<String>,
    pub one: bool,
    pub inputs: HashMap<String, String>,
}

impl Query {
    pub fn builder() -> QueryBuilder {
        QueryBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueryBuilder {
    pub selector: Option<String>,
    pub field: Option<Option<String>>,
    pub one: Option<bool>,
}

impl QueryBuilder {
    pub fn default_selector(mut self, selector: String) -> Self {
        self.selector = Some(selector);
        self
    }

    pub fn default_field(mut self, field: Option<String>) -> Self {
        self.field = Some(field);
        self
    }

    pub fn default_one(mut self, one: bool) -> Self {
        self.one = Some(one);
        self
    }

    pub fn build(self, config: config::Query) -> Result<Query> {
        let selector = config.selector
            .or(self.selector)
            .context("`selector` was not specified but is required")?;
        let field = config.field
            .or(self.field)
            .context("`field` was not specified but is required")?;
        let one = config.one
            .or(self.one)
            .context("`one` was not specified but is required")?;
        let inputs = config.inputs;
        Ok(Query { selector, field, one, inputs })
    }
}

pub fn query<T>(args: &CliArguments, config: &Query) -> Result<T>
where
    T: for<'a> Deserialize<'a>
{
    let mut cmd = Command::new(&args.typst);
    cmd.arg("query");
    if let Some(root) = &args.root {
        cmd.arg("--root").arg(root);
    }
    if let Some(field) = &config.field {
        cmd.arg("--field").arg(field);
    }
    if config.one {
        cmd.arg("--one");
    }
    let mut input = String::new();
    for (key, value) in &config.inputs {
        input.clear();
        input.push_str(key);
        input.push_str("=");
        input.push_str(value);
        cmd.arg("--input").arg(&input);
    }
    cmd.arg("--input").arg("prequery-fallback=true");
    cmd.arg(&args.input).arg(&config.selector);

    cmd.stderr(Stdio::inherit());
    let output = cmd.output()?;
    if !output.status.success() {
        let status = output.status;
        return Err(anyhow!("query command failed: {status}\n\n\t{cmd:?}"));
    }

    serde_json::from_slice(&output.stdout)
        .context("query resonse was not valid JSON or did not fit the expected schema")
}
