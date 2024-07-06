//! Executing `typst query` commands

use std::collections::HashMap;
use std::fmt::Write;
use std::process::Stdio;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use tokio::process::Command;

use crate::args::CliArguments;
use crate::config;

/// A query that can be run against a Typst document. This is usually configured from a
/// [config::Query] using a [QueryBuilder].
#[derive(Debug, Clone)]
pub struct Query {
    /// The selector to be queried, e.g. `<label>`
    pub selector: String,
    /// The field (`--field`) to be queried from the selector (with metadata elements, this is
    /// usually `value`)
    pub field: Option<String>,
    /// Whether only one (`--one`) query result is expected and should be returned
    pub one: bool,
    /// Any additional inputs (`--input`) to be given to the queried document. Regardless of these
    /// settings, `prequery-fallback` is always set to `true` during queries.
    pub inputs: HashMap<String, String>,
}

impl Query {
    /// Creates a query builder
    pub fn builder() -> QueryBuilder {
        QueryBuilder::default()
    }

    /// Builds the `typst query` command line for executing this command.
    pub fn command(&self, args: &CliArguments) -> Command {
        let mut cmd = Command::new(&args.typst);
        cmd.arg("query");
        if let Some(root) = &args.root {
            cmd.arg("--root").arg(root);
        }
        if let Some(field) = &self.field {
            cmd.arg("--field").arg(field);
        }
        if self.one {
            cmd.arg("--one");
        }
        let mut input = String::new();
        for (key, value) in &self.inputs {
            input.clear();
            write!(&mut input, "{key}={value}").expect("writing to a string failed");
            cmd.arg("--input").arg(&input);
        }
        cmd.arg("--input").arg("prequery-fallback=true");
        cmd.arg(&args.input).arg(&self.selector);

        cmd
    }

    /// Executes the query. This builds the necessary command line, runs the command, and returns
    /// the result parsed into the desired type from JSON.
    pub async fn query<T>(&self, args: &CliArguments) -> Result<T>
    where
        T: for<'a> Deserialize<'a>
    {
        let mut cmd = self.command(args);
        cmd.stderr(Stdio::inherit());
        let output = cmd.output().await?;
        if !output.status.success() {
            let status = output.status;
            return Err(anyhow!("query command failed: {status}\n\n\t{cmd:?}"));
        }

        serde_json::from_slice(&output.stdout)
            .context("query resonse was not valid JSON or did not fit the expected schema")
    }
}

/// A query builder. Default values for the various configs can be set. If a setting is missing from
/// the [config::Query], that default will be used.
#[derive(Debug, Clone, Default)]
pub struct QueryBuilder {
    /// The selector to be queried, e.g. `<label>`
    pub selector: Option<String>,
    /// The field (`--field`) to be queried from the selector (with metadata elements, this is
    /// usually `value`)
    pub field: Option<Option<String>>,
    /// Whether only one (`--one`) query result is expected and should be returned
    pub one: Option<bool>,
}

impl QueryBuilder {
    /// Set the selector to be queried, e.g. `<label>`
    pub fn default_selector(mut self, selector: String) -> Self {
        self.selector = Some(selector);
        self
    }

    /// Set the field (`--field`) to be queried from the selector (with metadata elements, this is
    /// usually `value`)
    pub fn default_field(mut self, field: Option<String>) -> Self {
        self.field = Some(field);
        self
    }

    /// Set whether only one (`--one`) query result is expected and should be returned
    pub fn default_one(mut self, one: bool) -> Self {
        self.one = Some(one);
        self
    }

    /// build a [Query] using the given defaults. If the [config::Query] doesn't contain a field
    /// that also doesn't have a default value, this will fail.
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
