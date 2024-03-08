#![macro_use]

macro_rules! io_data_error {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        std::io::Error::new(std::io::ErrorKind::InvalidData, message)
    }};
}

macro_rules! io_data_err {
    ($($arg:tt)*) => { Err(io_data_error!($($arg)*)) };
}

mod difference;
mod generated_file;
mod progress;

pub use difference::Difference;
pub use generated_file::write_generated;
pub use progress::Progress;
