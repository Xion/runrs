//! Module implementing logging for the application.

use slog::{self, DrainExt, Level, Record};
use slog_scope;
use slog_term;


/// Initialize logging for the application.
pub fn init() {
    // TODO: use slog_stream crate to better control log formatting;
    // example: https://github.com/slog-rs/misc/blob/master/examples/global_file_logger.rs
    let stderr = slog_term::streamer().sync().stderr()
        .use_custom_timestamp(move |io| write!(io, ""));  // No log timestamps.
    // TODO: accept a parameter to control logging verbosity
    let drain = slog::filter(
        |record: &Record| record.level().is_at_least(Level::Trace),
        stderr.build());

    let logger = slog::Logger::root(drain.fuse(), o!());
    slog_scope::set_global_logger(logger);
}
