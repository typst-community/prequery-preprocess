use prequery_preprocess::query::Query;
use prequery_preprocess::shell::{MockWorld, MockWorld_NewContext, ShellFactory};
use serial_test::serial;

mod common;

struct ShellTest {
    pub _ctx: MockWorld_NewContext,
    pub test: common::PreprocessorTest,
}

impl ShellTest {
    pub fn new(
        args: &'static [&'static str],
        manifest: &'static str,
        query: Query,
        query_result: &'static [u8],
        cfg_world: impl Fn(&mut MockWorld) + Send + 'static,
    ) -> Self {
        let ctx = MockWorld::new_context();
        ctx.expect().returning(move |main| {
            let mut world = MockWorld::default();
            world.expect_main().return_const(main);
            cfg_world(&mut world);
            world
        });

        let test = common::PreprocessorTest::new(
            |preprocessors| {
                preprocessors.register(ShellFactory::<MockWorld>::new());
            },
            args,
            manifest,
            query,
            query_result,
        );

        Self { _ctx: ctx, test }
    }

    pub async fn run(self) -> common::RunResult {
        self.test.run().await
    }
}

/// Run the shell preprocessor without any resources and no index.
/// No downloads should happen, and no index should be saved.
#[tokio::test]
#[serial(shell)]
async fn run_shell_no_resources_no_index() {
    ShellTest::new(
        &["prequery-preprocess", "input.typ"],
        r#"
        [package]
        name = "test"
        version = "0.0.1"
        entrypoint = "main.typ"

        [[tool.prequery.jobs]]
        name = "shell"
        kind = "shell"

        query.selector = "<shell>"
        "#,
        Query {
            selector: "<shell>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();

            // no resources in the query result
            world.expect_resource_exists().never();
            world.expect_download().never();
        },
    )
    .run()
    .await
    .expect_ok("shell job should succeed")
    .expect_log(include_str!("shell/no-inputs.txt"));
}
