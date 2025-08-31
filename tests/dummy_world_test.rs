use clap::Parser;

use mockall::predicate::{always, eq};
use prequery_preprocess::args::CliArguments;
use prequery_preprocess::entry::run;
use prequery_preprocess::error::Result;
use prequery_preprocess::manifest::PrequeryManifest;
use prequery_preprocess::preprocessor::{
    MockPreprocessor, MockPreprocessorDefinition, PreprocessorMap,
};
use prequery_preprocess::world::MockWorld;

#[tokio::test]
async fn run_dummy_preprocessor() -> Result<()> {
    // dummy preprocessor that is used by the configuration
    let mut dummy = MockPreprocessorDefinition::new();
    dummy.expect_name().return_const("dummy");
    dummy
        .expect_configure()
        .once()
        .with(always(), eq("test".to_string()), always(), always())
        .returning(|_world, name, _manifest, _query| {
            // when run, the preprocessor does nothing
            let mut preprocessor = MockPreprocessor::new();
            preprocessor.expect_name().return_const(name);
            preprocessor.expect_run().once().returning(|| Ok(()));
            Ok(Box::new(preprocessor))
        });

    // dummy preprocessor that is not used by the configuration
    let mut dummy2 = MockPreprocessorDefinition::new();
    dummy2.expect_name().return_const("dummy2");
    // must not be used to configure an instance
    dummy2.expect_configure().never();

    // mock world that contains the two preprocessors and basic setup
    let mut world = MockWorld::new();
    world.expect_preprocessors().return_const({
        let mut preprocessors = PreprocessorMap::new();
        preprocessors.register(dummy);
        preprocessors.register(dummy2);
        preprocessors
    });
    world
        .expect_arguments()
        .return_const(CliArguments::parse_from([
            "prequery-preprocess",
            "input.typ",
        ]));
    world.expect_read_typst_toml().returning(|| {
        PrequeryManifest::parse(
            r#"
            [package]
            name = "test"
            version = "0.0.1"
            entrypoint = "main.typ"

            [[tool.prequery.jobs]]
            name = "test"
            kind = "dummy"
            "#,
        )
    });

    // run the world
    run(world).await
}
