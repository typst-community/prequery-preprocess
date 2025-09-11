use mockall::predicate::{always, eq};
use prequery_preprocess::log;
use prequery_preprocess::preprocessor::{MockPreprocessor, MockPreprocessorDefinition};
use prequery_preprocess::query::Query;
use prequery_preprocess::world::{MockWorld, World};

mod common;

#[tokio::test]
async fn run_dummy() {
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

    common::PreprocessorTest::new(
        |preprocessors| {
            preprocessors.register(dummy);
            preprocessors.register(dummy2);
        },
        &["prequery-preprocess", "input.typ"],
        r#"
        [package]
        name = "test"
        version = "0.0.1"
        entrypoint = "main.typ"

        [[tool.prequery.jobs]]
        name = "test"
        kind = "dummy"
        "#,
        // unused
        Query {
            selector: Default::default(),
            field: Default::default(),
            one: Default::default(),
            inputs: Default::default(),
        },
        b"",
    )
    .run()
    .await
    .expect_ok("dummy job should succeed")
    .expect_log(include_str!("dummy/run.txt"));
}
