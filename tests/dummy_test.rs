use clap::Parser;

use mockall::predicate::{always, eq};
use prequery_preprocess::VecLog;
use prequery_preprocess::args::CliArguments;
use prequery_preprocess::entry::run;
use prequery_preprocess::error::Result;
use prequery_preprocess::log;
use prequery_preprocess::manifest::PrequeryManifest;
use prequery_preprocess::preprocessor::{
    MockPreprocessor, MockPreprocessorDefinition, PreprocessorMap,
};
use prequery_preprocess::world::{MockWorld, World};

#[tokio::test]
async fn run_dummy() -> Result<()> {
    // dummy preprocessor that is used by the configuration
    let mut dummy = MockPreprocessorDefinition::<MockWorld>::new();
    dummy.expect_name().return_const("dummy");
    dummy
        .expect_configure()
        .once()
        .with(always(), eq("test".to_string()), always(), always())
        .returning(|world, name, _manifest, _query| {
            let world = world.clone();
            // when run, the preprocessor only logs something
            let mut preprocessor = MockPreprocessor::new();
            preprocessor.expect_world().return_const(world.clone());
            preprocessor.expect_name().return_const(name.clone());
            preprocessor.expect_run().once().returning(move || {
                let mut l = world.log();
                log!(l, "[{name}] this is a dummy preprocessor");
                Ok(())
            });
            Ok(Box::new(preprocessor))
        });

    // dummy preprocessor that is not used by the configuration
    let mut dummy2 = MockPreprocessorDefinition::new();
    dummy2.expect_name().return_const("dummy2");
    // must not be used to configure an instance
    dummy2.expect_configure().never();

    // mock world that contains the two preprocessors and basic setup
    let log = VecLog::new();
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
    world.expect_log().return_const(log.clone());
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
    run(world).await?;

    // assert correct logging
    assert_eq!(log.get_lossy(), include_str!("dummy/run.txt"));

    Ok(())
}
