mod args;

use std::path::{self, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;

use args::CliArguments;

fn resolve_typst_toml<P: AsRef<Path>>(input: P) -> Result<PathBuf> {
    const TYPST_TOML: &str = "typst.toml";

    let input = path::absolute(&input)
        .with_context(|| {
            let input_str = input.as_ref().to_string_lossy();
            format!("cannot resolve {TYPST_TOML} because input file {input_str} can't be resolved")
        })?;
    let mut p = input.clone();

    // the input path needs to refer to a file. refer to typst.toml instead
    p.set_file_name(TYPST_TOML);
    while !p.is_file() {
        // remove the file name
        let result = p.pop();
        assert!(result, "the path should have had a final component of `{TYPST_TOML}`");
        // go one level up
        let result = p.pop();
        if !result {
            // if there is no level up, not typst.toml was found
            let input_str = input.to_string_lossy();
            return Err(anyhow!("no {TYPST_TOML} file was found in any ancestor directory of {input_str}"));
        }
        // re-add the file name
        p.push(TYPST_TOML);
    }
    Ok(p)
}

fn main() -> Result<()> {
    let args = CliArguments::parse();

    let typst_toml = resolve_typst_toml(args.input)?;
    let typst_toml = std::fs::read_to_string(typst_toml)?;
    println!("{typst_toml}");

    Ok(())
}
