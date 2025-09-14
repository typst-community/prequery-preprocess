use std::path::PathBuf;

use mockall::predicate::eq;
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

/// Run the shell preprocessor with two separate commands, saved to one file.
#[tokio::test]
#[serial(shell)]
async fn run_shell_python_snippets() {
    ShellTest::new(
        &["prequery-preprocess", "input.typ"],
        r#"
        [package]
        name = "test"
        version = "0.0.1"
        entrypoint = "main.typ"

        [[tool.prequery.jobs]]
        name = "python"
        kind = "shell"

        query.selector = "<python>"

        command = "python"
        "#,
        Query {
            selector: "<python>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[{"path": "out.json"}, {"data": "print(\"Hello World\")"}, {"data": "print(\"Hello Prequery\")"}]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();

            // two code snippets
            world.expect_run_command()
                .once()
                .with(
                    eq(["python".to_string()]),
                    eq(*br#""print(\"Hello World\")""#),
                )
                .returning(|_, _| Ok(br#""Hello World\n""#.to_vec()));
            world.expect_run_command()
                .once()
                .with(
                    eq(["python".to_string()]),
                    eq(*br#""print(\"Hello Prequery\")""#),
                )
                .returning(|_, _| Ok(br#""Hello Prequery\n""#.to_vec()));

            // one combined output file
            world
                .expect_write_output()
                .once()
                .with(
                    eq(PathBuf::from("out.json")),
                    eq(*br#"["Hello World\n","Hello Prequery\n"]"#),
                )
                .returning(|_, _| Ok(()));
        },
    )
    .run()
    .await
    .expect_ok("shell job should succeed")
    .expect_log(include_str!("shell/python.txt"));
}

/// Run the shell preprocessor with two separate commands, saved to separate files.
#[tokio::test]
#[serial(shell)]
async fn run_shell_python_snippets_separate_files() {
    ShellTest::new(
        &["prequery-preprocess", "input.typ"],
        r#"
        [package]
        name = "test"
        version = "0.0.1"
        entrypoint = "main.typ"

        [[tool.prequery.jobs]]
        name = "python"
        kind = "shell"

        query.selector = "<python>"

        command = "python"
        "#,
        Query {
            selector: "<python>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[{"path": "out1.json", "data": "print(\"Hello World\")"}, {"path": "out2.json", "data": "print(\"Hello Prequery\")"}]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();

            // two code snippets
            world.expect_run_command()
                .once()
                .with(
                    eq(["python".to_string()]),
                    eq(*br#""print(\"Hello World\")""#),
                )
                .returning(|_, _| Ok(br#""Hello World\n""#.to_vec()));
            world.expect_run_command()
                .once()
                .with(
                    eq(["python".to_string()]),
                    eq(*br#""print(\"Hello Prequery\")""#),
                )
                .returning(|_, _| Ok(br#""Hello Prequery\n""#.to_vec()));

            // separate output files
            world
                .expect_write_output()
                .with(
                    eq(PathBuf::from("out1.json")),
                    eq(*br#""Hello World\n""#),
                )
                .returning(|_, _| Ok(()));
            world
                .expect_write_output()
                .with(
                    eq(PathBuf::from("out2.json")),
                    eq(*br#""Hello Prequery\n""#),
                )
                .returning(|_, _| Ok(()));
        },
    )
    .run()
    .await
    .expect_ok("shell job should succeed")
    .expect_log(include_str!("shell/python.txt"));
}

/// Run the shell preprocessor with two joined commands, saved to one file.
#[tokio::test]
#[serial(shell)]
async fn run_shell_python_joined_snippets() {
    ShellTest::new(
        &["prequery-preprocess", "input.typ"],
        r#"
        [package]
        name = "test"
        version = "0.0.1"
        entrypoint = "main.typ"

        [[tool.prequery.jobs]]
        name = "python"
        kind = "shell"

        query.selector = "<python>"

        command = ["python", "exec.py"]
        joined = true
        "#,
        Query {
            selector: "<python>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[{"path": "out.json"}, {"data": "x = 1\nprint(x)"}, {"data": "y = x + 1\nprint(y)"}]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();

            // two code snippets
            world
                .expect_run_command()
                .once()
                .with(
                    eq(["python".to_string(), "exec.py".to_string()]),
                    eq(*br#"["x = 1\nprint(x)","y = x + 1\nprint(y)"]"#),
                )
                .returning(|_, _| Ok(br#"["1\n","2\n"]"#.to_vec()));

            // one combined output file
            world
                .expect_write_output()
                .once()
                .with(eq(PathBuf::from("out.json")), eq(*br#"["1\n","2\n"]"#))
                .returning(|_, _| Ok(()));
        },
    )
    .run()
    .await
    .expect_ok("shell job should succeed")
    .expect_log(include_str!("shell/joined-python.txt"));
}

/// Run the shell preprocessor with two joined commands, saved to separate files.
#[tokio::test]
#[serial(shell)]
async fn run_shell_python_joined_snippets_separate_files() {
    ShellTest::new(
        &["prequery-preprocess", "input.typ"],
        r#"
        [package]
        name = "test"
        version = "0.0.1"
        entrypoint = "main.typ"

        [[tool.prequery.jobs]]
        name = "python"
        kind = "shell"

        query.selector = "<python>"

        command = ["python", "exec.py"]
        joined = true
        "#,
        Query {
            selector: "<python>".to_string(),
            field: Some("value".to_string()),
            one: false,
            inputs: Default::default(),
        },
        br#"[{"path": "out1.json", "data": "x = 1\nprint(x)"}, {"path": "out2.json", "data": "y = x + 1\nprint(y)"}]"#,
        |world| {
            // no index specified in the manifest
            world.expect_read_index().never();
            world.expect_write_index().never();

            // two code snippets
            world
                .expect_run_command()
                .once()
                .with(
                    eq(["python".to_string(), "exec.py".to_string()]),
                    eq(*br#"["x = 1\nprint(x)","y = x + 1\nprint(y)"]"#),
                )
                .returning(|_, _| Ok(br#"["1\n","2\n"]"#.to_vec()));

            // separate output files
            world
                .expect_write_output()
                .with(
                    eq(PathBuf::from("out1.json")),
                    eq(*br#""1\n""#),
                )
                .returning(|_, _| Ok(()));
            world
                .expect_write_output()
                .with(
                    eq(PathBuf::from("out2.json")),
                    eq(*br#""2\n""#),
                )
                .returning(|_, _| Ok(()));
        },
    )
    .run()
    .await
    .expect_ok("shell job should succeed")
    .expect_log(include_str!("shell/joined-python.txt"));
}
