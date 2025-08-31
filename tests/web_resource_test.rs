use clap::Parser;

use mockall::predicate::eq;
use typst_preprocess::args::CliArguments;
use typst_preprocess::entry::run;
use typst_preprocess::error::Result;
use typst_preprocess::manifest::PrequeryManifest;
use typst_preprocess::preprocessor::PreprocessorMap;
use typst_preprocess::query::Query;
use typst_preprocess::web_resource::{MockWorld, MockWorld_NewContext, WebResourceFactory};
use typst_preprocess::world::MockWorld as MainMockWorld;

struct WebResourceTest {
    pub _ctx: MockWorld_NewContext,
    pub world: MainMockWorld,
}

impl WebResourceTest {
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

        let mut world = MainMockWorld::new();
        world.expect_preprocessors().return_const({
            let mut preprocessors = PreprocessorMap::new();
            preprocessors.register(WebResourceFactory::<MockWorld>::new());
            preprocessors
        });
        world
            .expect_arguments()
            .return_const(CliArguments::parse_from(args));
        world
            .expect_read_typst_toml()
            .returning(|| PrequeryManifest::parse(manifest));

        world
            .expect_query()
            .with(eq(query))
            .returning(|_| Ok(query_result.to_vec()));

        Self { _ctx: ctx, world }
    }

    pub async fn run(self) -> Result<()> {
        run(self.world).await
    }
}

#[tokio::test]
async fn run_web_resource() -> Result<()> {
    WebResourceTest::new(
        &["typst-preprocess", "input.typ"],
        r#"
        [package]
        name = "test"
        version = "0.0.1"
        entrypoint = "main.typ"

        [[tool.prequery.jobs]]
        name = "download"
        kind = "web-resource"
        "#,
        Query {
            selector: "<web-resource>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();
        },
    )
    .run()
    .await
}
