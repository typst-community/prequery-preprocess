use std::path::PathBuf;

use clap::Parser;

use mockall::predicate::eq;
use serial_test::serial;
use typst_preprocess::args::CliArguments;
use typst_preprocess::entry::run;
use typst_preprocess::error::Result;
use typst_preprocess::manifest::PrequeryManifest;
use typst_preprocess::preprocessor::PreprocessorMap;
use typst_preprocess::query::Query;
use typst_preprocess::web_resource::index::{Index, Resource};
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
            .expect_query_impl()
            .with(eq(query))
            .returning(|_| Ok(query_result.to_vec()));

        Self { _ctx: ctx, world }
    }

    pub async fn run(self) -> Result<()> {
        run(self.world).await
    }
}

/// Run the web resource preprocessor without any resources and no index.
/// No downloads should happen, and no index should be saved.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_resources_no_index() -> Result<()> {
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

            // no resources in the query result
            world.expect_resource_exists().never();
            world.expect_download().never();
        },
    )
    .run()
    .await
}

/// Run the web resource preprocessor without any resources and an index.
/// No downloads should happen, but the index should be saved.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_resources_with_index() -> Result<()> {
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
        index = true
        "#,
        Query {
            selector: "<web-resource>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[]"#,
        |world| {
            world
                .expect_read_index()
                .once()
                .with(eq(PathBuf::from("web-resource-index.toml")))
                .returning(|location| Ok(Index::new(location.to_path_buf())));
            world
                .expect_write_index()
                .once()
                .with(eq(Index::new(PathBuf::from("web-resource-index.toml"))))
                .returning(|_| Ok(()));

            // no resources in the query result
            world.expect_resource_exists().never();
            world.expect_download().never();
        },
    )
    .run()
    .await
}

/// Run the web resource preprocessor with one resource and no index.
/// The resource is outside the root and should not be downloaded.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_download_outside_root() -> Result<()> {
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
        br#"[{"url": "https://example.com/example.png", "path": "../example.png"}]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();

            world.expect_resource_exists().never();
            world.expect_download().never();
        },
    )
    .run()
    .await
    .expect_err("access to file outside root should be denied");

    Ok(())
}

/// Run the web resource preprocessor with one resource and no index.
/// The resource does not exist locally and should be downloaded.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_index_missing() -> Result<()> {
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
        br#"[{"url": "https://example.com/example.png", "path": "assets/example.png"}]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();

            world
                .expect_resource_exists()
                .once()
                .with(eq(PathBuf::from("assets/example.png")))
                .return_const(false);
            world
                .expect_download()
                .once()
                .with(
                    eq(PathBuf::from("assets/example.png")),
                    eq("https://example.com/example.png"),
                )
                .returning(|_, _| Ok(()));
        },
    )
    .run()
    .await
}

/// Run the web resource preprocessor with one resource and no index.
/// The resource exists locally and should not be downloaded.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_index_existing() -> Result<()> {
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
        br#"[{"url": "https://example.com/example.png", "path": "assets/example.png"}]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();

            world
                .expect_resource_exists()
                .once()
                .with(eq(PathBuf::from("assets/example.png")))
                .return_const(true);
            world.expect_download().never();
        },
    )
    .run()
    .await
}

/// Run the web resource preprocessor with one resource and no index.
/// The resource exists locally and should be re-downloaded according to the manifest.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_index_existing_forced() -> Result<()> {
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
        overwrite = true
        "#,
        Query {
            selector: "<web-resource>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[{"url": "https://example.com/example.png", "path": "assets/example.png"}]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();

            world
                .expect_resource_exists()
                .once()
                .with(eq(PathBuf::from("assets/example.png")))
                .return_const(true);
            world
                .expect_download()
                .once()
                .with(
                    eq(PathBuf::from("assets/example.png")),
                    eq("https://example.com/example.png"),
                )
                .returning(|_, _| Ok(()));
        },
    )
    .run()
    .await
}

/// Run the web resource preprocessor with one resource and an index.
/// The resource does not exist locally and should be downloaded.
/// The index should be saved with the downloaded resource in it.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_with_index_missing() -> Result<()> {
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
        index = true
        "#,
        Query {
            selector: "<web-resource>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[{"url": "https://example.com/example.png", "path": "assets/example.png"}]"#,
        |world| {
            world
                .expect_read_index()
                .once()
                .with(eq(PathBuf::from("web-resource-index.toml")))
                .returning(|location| Ok(Index::new(location.to_path_buf())));
            world
                .expect_write_index()
                .once()
                .with(eq({
                    let mut index = Index::new(PathBuf::from("web-resource-index.toml"));
                    index.update(Resource {
                        path: PathBuf::from("assets/example.png"),
                        url: "https://example.com/example.png".to_string(),
                    });
                    index
                }))
                .returning(|_| Ok(()));

            world
                .expect_resource_exists()
                .once()
                .with(eq(PathBuf::from("assets/example.png")))
                .return_const(false);
            world
                .expect_download()
                .once()
                .with(
                    eq(PathBuf::from("assets/example.png")),
                    eq("https://example.com/example.png"),
                )
                .returning(|_, _| Ok(()));
        },
    )
    .run()
    .await
}

/// Run the web resource preprocessor with one resource and an index.
/// The resource exists locally and should not be downloaded.
/// The index should be saved with the downloaded resource in it (no change).
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_with_index_existing() -> Result<()> {
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
        index = true
        "#,
        Query {
            selector: "<web-resource>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[{"url": "https://example.com/example.png", "path": "assets/example.png"}]"#,
        |world| {
            world
                .expect_read_index()
                .once()
                .with(eq(PathBuf::from("web-resource-index.toml")))
                .returning(|location| {
                    let mut index = Index::new(location.to_path_buf());
                    index.update(Resource {
                        path: PathBuf::from("assets/example.png"),
                        url: "https://example.com/example.png".to_string(),
                    });
                    Ok(index)
                });
            world
                .expect_write_index()
                .once()
                .with(eq({
                    let mut index = Index::new(PathBuf::from("web-resource-index.toml"));
                    index.update(Resource {
                        path: PathBuf::from("assets/example.png"),
                        url: "https://example.com/example.png".to_string(),
                    });
                    index
                }))
                .returning(|_| Ok(()));

            world
                .expect_resource_exists()
                .once()
                .with(eq(PathBuf::from("assets/example.png")))
                .return_const(true);
            world.expect_download().never();
        },
    )
    .run()
    .await
}

/// Run the web resource preprocessor with one resource and an index.
/// The resource exists locally and should be re-downloaded according to the manifest.
/// The index should be saved with the downloaded resource in it (no change).
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_with_index_existing_forced() -> Result<()> {
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
        index = true
        overwrite = true
        "#,
        Query {
            selector: "<web-resource>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[{"url": "https://example.com/example.png", "path": "assets/example.png"}]"#,
        |world| {
            world
                .expect_read_index()
                .once()
                .with(eq(PathBuf::from("web-resource-index.toml")))
                .returning(|location| {
                    let mut index = Index::new(location.to_path_buf());
                    index.update(Resource {
                        path: PathBuf::from("assets/example.png"),
                        url: "https://example.com/example.png".to_string(),
                    });
                    Ok(index)
                });
            world
                .expect_write_index()
                .once()
                .with(eq({
                    let mut index = Index::new(PathBuf::from("web-resource-index.toml"));
                    index.update(Resource {
                        path: PathBuf::from("assets/example.png"),
                        url: "https://example.com/example.png".to_string(),
                    });
                    index
                }))
                .returning(|_| Ok(()));

            world
                .expect_resource_exists()
                .once()
                .with(eq(PathBuf::from("assets/example.png")))
                .return_const(true);
            world
                .expect_download()
                .once()
                .with(
                    eq(PathBuf::from("assets/example.png")),
                    eq("https://example.com/example.png"),
                )
                .returning(|_, _| Ok(()));
        },
    )
    .run()
    .await
}

/// Run the web resource preprocessor with one resource and an index.
/// The resource exists locally and should be re-downloaded because the URL has changed.
/// The index should be saved with the downloaded resource in it (changed URL).
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_with_index_outdated() -> Result<()> {
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
        index = true
        "#,
        Query {
            selector: "<web-resource>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[{"url": "https://example.com/example.png", "path": "assets/example.png"}]"#,
        |world| {
            world
                .expect_read_index()
                .once()
                .with(eq(PathBuf::from("web-resource-index.toml")))
                .returning(|location| {
                    let mut index = Index::new(location.to_path_buf());
                    index.update(Resource {
                        path: PathBuf::from("assets/example.png"),
                        url: "https://example.com/example-old.png".to_string(),
                    });
                    Ok(index)
                });
            world
                .expect_write_index()
                .once()
                .with(eq({
                    let mut index = Index::new(PathBuf::from("web-resource-index.toml"));
                    index.update(Resource {
                        path: PathBuf::from("assets/example.png"),
                        url: "https://example.com/example.png".to_string(),
                    });
                    index
                }))
                .returning(|_| Ok(()));

            world
                .expect_resource_exists()
                .once()
                .with(eq(PathBuf::from("assets/example.png")))
                .return_const(true);
            world
                .expect_download()
                .once()
                .with(
                    eq(PathBuf::from("assets/example.png")),
                    eq("https://example.com/example.png"),
                )
                .returning(|_, _| Ok(()));
        },
    )
    .run()
    .await
}
