#![macro_use]

#[allow(unused_imports)]
use super::*;

macro_rules! io_data_error {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        std::io::Error::new(std::io::ErrorKind::InvalidData, message)
    }};
}

macro_rules! io_data_err {
    ($($arg:tt)*) => { Err(io_data_error!($($arg)*)) };
}

mod dedupe_file_set;
mod difference;
mod generated_file;
mod progress;

pub use dedupe_file_set::{ConcurrentDedupeFileSet, DedupeFileSet};
pub use difference::Difference;
pub use generated_file::write_generated;
pub use progress::Progress;
