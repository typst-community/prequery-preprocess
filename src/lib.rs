#![cfg_attr(not(feature = "test"), warn(missing_docs))]
//! A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.

#[macro_use]
mod reporting;

pub mod args;
pub mod entry;
pub mod error;
pub mod manifest;
pub mod preprocessor;
mod preprocessors;
pub mod query;
mod utils;
pub mod world;

// re-export the actual preprocessors from the top level
pub use preprocessors::*;

#[cfg(feature = "test")]
pub use test_utils::*;

#[cfg(feature = "test")]
mod test_utils {
    use std::error::Error;
    use std::fmt::Display;
    use std::io;
    use std::sync::{Arc, Mutex};

    /// Never type, see https://github.com/rust-lang/rust/issues/35121
    #[derive(Debug)]
    pub enum Never {}

    impl Display for Never {
        fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match *self {}
        }
    }

    impl Error for Never {}

    #[derive(Default, Debug, Clone)]
    pub struct VecLog(Arc<Mutex<Vec<u8>>>);

    impl VecLog {
        pub fn new() -> Self {
            Self::default()
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, Vec<u8>> {
            self.0.lock().expect("lock VecLog")
        }

        pub fn get(&self) -> Vec<u8> {
            let handle = self.lock();
            handle.clone()
        }

        pub fn get_lossy(&self) -> String {
            let handle = self.lock();
            String::from_utf8_lossy(&handle).to_string()
        }
    }

    impl io::Write for VecLog {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut handle = self.lock();
            handle.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
