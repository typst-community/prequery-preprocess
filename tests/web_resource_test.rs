use clap::Parser;

use mockall::predicate::eq;
use typst_preprocess::args::CliArguments;
use typst_preprocess::entry::run;
use typst_preprocess::error::Result;
use typst_preprocess::manifest::PrequeryManifest;
use typst_preprocess::preprocessor::PreprocessorMap;
use typst_preprocess::query::Query;
use typst_preprocess::web_resource::{MockWorld, WebResourceFactory};
use typst_preprocess::world::MockWorld as MainMockWorld;

#[tokio::test]
async fn run_web_resource() -> Result<()> {
    // prepare the web resource mock world
    let ctx = MockWorld::new_context();
    ctx.expect().returning(|main| {
        let mut world = MockWorld::default();
        world.expect_main().return_const(main);
        // no index specified in the manifest
        world.expect_read_index().never();
        world.expect_write_index().never();

        world
    });

    // mock world that contains the preprocessor and basic setup
    let mut world = MainMockWorld::new();
    world.expect_preprocessors().return_const({
        let mut preprocessors = PreprocessorMap::new();
        preprocessors.register(WebResourceFactory::<MockWorld>::new());
        preprocessors
    });
    world
        .expect_arguments()
        .return_const(CliArguments::parse_from(&["typst-preprocess", "input.typ"]));
    world.expect_read_typst_toml().returning(|| {
        PrequeryManifest::parse(
            r#"
            [package]
            name = "test"
            version = "0.0.1"
            entrypoint = "main.typ"

            [[tool.prequery.jobs]]
            name = "download"
            kind = "web-resource"
            "#,
        )
    });

    // mock the query result
    world
        .expect_query()
        .with(eq(Query {
            selector: "<web-resource>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        }))
        .returning(|_| Ok(br#"[]"#.to_vec()));

    // run the world
    run(world).await
}
