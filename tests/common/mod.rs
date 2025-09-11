use clap::Parser;

use mockall::predicate::eq;
use prequery_preprocess::VecLog;
use prequery_preprocess::args::CliArguments;
use prequery_preprocess::entry::run;
use prequery_preprocess::error::Result;
use prequery_preprocess::manifest::PrequeryManifest;
use prequery_preprocess::preprocessor::PreprocessorMap;
use prequery_preprocess::query::Query;
use prequery_preprocess::world::MockWorld;

pub struct PreprocessorTest {
    pub world: MockWorld,
    pub log: VecLog,
}

impl PreprocessorTest {
    pub fn new(
        register_preprocessors: impl FnOnce(&mut PreprocessorMap<MockWorld>),
        args: &'static [&'static str],
        manifest: &'static str,
        query: Query,
        query_result: &'static [u8],
    ) -> Self {
        let log = VecLog::new();
        let mut world = MockWorld::new();
        world.expect_preprocessors().return_const({
            let mut preprocessors = PreprocessorMap::new();
            register_preprocessors(&mut preprocessors);
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

        Self { world, log }
    }

    pub async fn run(self) -> RunResult {
        let result = run(self.world).await;
        let log = self.log;
        RunResult { result, log }
    }
}

#[derive(Debug)]
#[must_use]
pub struct RunResult {
    result: Result<()>,
    log: VecLog,
}

#[derive(Debug)]
#[must_use]
pub struct RunResultLog(VecLog);

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
    fn log_eq(output: &str, expected: &str) -> bool {
        let mut output = output.chars();
        let mut expected = expected.chars();

        let mut ch_out = output.next();
        let mut ch_exp = expected.next();
        loop {
            // `expected` is read from a file, and may be checked out by Git using platform-specific
            // line endings. The path separator in expected output is always `/`.
            // `output` is produced at runtime. Rust's `writeln!()` and related macros will always
            // use `\n`, but paths may contain `\\` instead of `/`.
            match (ch_out, ch_exp) {
                // both strings have been consumed
                (None, None) => return true,
                // one string has been consumed, the other is still going
                (None, Some(_)) | (Some(_), None) => return false,
                // both strings continue with the same character
                (Some(a), Some(b)) if a == b => {}
                // the difference is a path separator
                // (or a false positive, but we accept this possibility)
                (Some('\\'), Some('/')) => {}
                // the difference is a line separator
                (Some('\n'), Some('\r')) => {
                    let ch_exp = expected.next();
                    if ch_exp != Some('\n') {
                        // not actually a line separator difference
                        return false;
                    }
                }
                // it's a real difference
                (Some(_), Some(_)) => {
                    return false;
                }
            }

            ch_out = output.next();
            ch_exp = expected.next();
        }
    }

    pub fn expect_log(self, expected: &str) {
        let output = self.0.get_lossy();
        assert!(
            Self::log_eq(&output, expected),
            "{output}\nnot equal to\n\n{expected}"
        );
    }
}
