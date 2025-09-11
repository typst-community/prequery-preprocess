use std::path::PathBuf;

use clap::Parser;

use mockall::predicate::eq;
use prequery_preprocess::VecLog;
use prequery_preprocess::args::CliArguments;
use prequery_preprocess::entry::run;
use prequery_preprocess::error::Result;
use prequery_preprocess::manifest::PrequeryManifest;
use prequery_preprocess::preprocessor::PreprocessorMap;
use prequery_preprocess::query::Query;
use prequery_preprocess::web_resource::index::{Index, Resource};
use prequery_preprocess::web_resource::{MockWorld, MockWorld_NewContext, WebResourceFactory};
use prequery_preprocess::world::MockWorld as MainMockWorld;
use serial_test::serial;

struct WebResourceTest {
    pub _ctx: MockWorld_NewContext,
    pub world: MainMockWorld,
    pub log: VecLog,
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

        let log = VecLog::new();
        let mut world = MainMockWorld::new();
        world.expect_preprocessors().return_const({
            let mut preprocessors = PreprocessorMap::new();
            preprocessors.register(WebResourceFactory::<MockWorld>::new());
            preprocessors
        });
        world
            .expect_arguments()
            .return_const(CliArguments::parse_from(args));
        world.expect_log().return_const(log.clone());
        world
            .expect_read_typst_toml()
            .returning(|| PrequeryManifest::parse(manifest));

        world
            .expect_query_impl()
            .with(eq(query))
            .returning(|_| Ok(query_result.to_vec()));

        Self {
            _ctx: ctx,
            world,
            log,
        }
    }

    pub async fn run(self) -> RunResult {
        let result = run(self.world).await;
        let log = self.log;
        RunResult { result, log }
    }
}

#[derive(Debug)]
#[must_use]
struct RunResult {
    result: Result<()>,
    log: VecLog,
}

#[derive(Debug)]
#[must_use]
struct RunResultLog(VecLog);

impl RunResult {
    pub fn expect_ok(self, msg: &str) -> RunResultLog {
        self.result.as_ref().expect(msg);
        RunResultLog(self.log)
    }

    pub fn expect_err(self, msg: &str) -> RunResultLog {
        self.result.as_ref().expect_err(msg);
        RunResultLog(self.log)
    }
}

impl RunResultLog {
    pub fn expect_log(self, log: &str) {
        assert_eq!(self.0.get_lossy(), log);
    }
}

/// Run the web resource preprocessor without any resources and no index.
/// No downloads should happen, and no index should be saved.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_resources_no_index() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_ok("download job should succeed")
    .expect_log(
        "\
        [download] beginning job...\n\
        [download] job finished\n",
    );
}

/// Run the web resource preprocessor without any resources and an index.
/// No downloads should happen, but the index should be saved.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_resources_with_index() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_ok("download job should succeed")
    .expect_log(
        "\
        [download] beginning job...\n\
        [download] job finished\n",
    );
}

/// Run the web resource preprocessor with one resource and no index.
/// The resource is outside the root and should not be downloaded.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_download_outside_root() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_err("access to file outside root should be denied")
    .expect_log(
        "\
[download] beginning job...
[download] Can't download to ../example.png: ../example.png is outside the project root
[download] job failed: at least one download failed:
  ../example.png is outside the project root
at least one job's execution failed:
  [download] at least one download failed:
      ../example.png is outside the project root
",
    );
}

/// Run the web resource preprocessor with one resource and no index.
/// The resource does not exist locally and should be downloaded.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_index_missing() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_ok("download job should succeed")
    .expect_log(
        "\
        [download] beginning job...\n\
        [download] Downloading to assets/example.png: https://example.com/example.png...\n\
        [download] Downloading to assets/example.png finished\n\
        [download] job finished\n",
    );
}

/// Run the web resource preprocessor with one resource and no index.
/// The resource exists locally and should not be downloaded.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_index_existing() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_ok("download job should succeed")
    .expect_log(
        "\
        [download] beginning job...\n\
        [download] Downloading to assets/example.png skipped: https://example.com/example.png (file exists)\n\
        [download] job finished\n",
    );
}

/// Run the web resource preprocessor with one resource and no index.
/// The resource exists locally and should be re-downloaded according to the manifest.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_no_index_existing_forced() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_ok("download job should succeed")
    .expect_log(
        "\
        [download] beginning job...\n\
        [download] Downloading to assets/example.png: https://example.com/example.png (overwrite of existing files was forced)...\n\
        [download] Downloading to assets/example.png finished\n\
        [download] job finished\n",
    );
}

/// Run the web resource preprocessor with one resource and an index.
/// The resource does not exist locally and should be downloaded.
/// The index should be saved with the downloaded resource in it.
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_with_index_missing() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_ok("download job should succeed")
    .expect_log(
        "\
        [download] beginning job...\n\
        [download] Downloading to assets/example.png: https://example.com/example.png...\n\
        [download] Downloading to assets/example.png finished\n\
        [download] job finished\n",
    );
}

/// Run the web resource preprocessor with one resource and an index.
/// The resource exists locally and should not be downloaded.
/// The index should be saved with the downloaded resource in it (no change).
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_with_index_existing() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_ok("download job should succeed")
    .expect_log(
        "\
        [download] beginning job...\n\
        [download] Downloading to assets/example.png skipped: https://example.com/example.png (file exists)\n\
        [download] job finished\n",
    );
}

/// Run the web resource preprocessor with one resource and an index.
/// The resource exists locally and should be re-downloaded according to the manifest.
/// The index should be saved with the downloaded resource in it (no change).
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_with_index_existing_forced() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_ok("download job should succeed")
    .expect_log(
        "\
        [download] beginning job...\n\
        [download] Downloading to assets/example.png: https://example.com/example.png (overwrite of existing files was forced)...\n\
        [download] Downloading to assets/example.png finished\n\
        [download] job finished\n",
    );
}

/// Run the web resource preprocessor with one resource and an index.
/// The resource exists locally and should be re-downloaded because the URL has changed.
/// The index should be saved with the downloaded resource in it (changed URL).
#[tokio::test]
#[serial(web_resource)]
async fn run_web_resource_with_index_outdated() {
    WebResourceTest::new(
        &["prequery-preprocess", "input.typ"],
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
    .expect_ok("download job should succeed")
    .expect_log(
        "\
        [download] beginning job...\n\
        [download] Downloading to assets/example.png: https://example.com/example.png (URL has changed)...\n\
        [download] Downloading to assets/example.png finished\n\
        [download] job finished\n",
    );
}
