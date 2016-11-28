//! Module implementing logging for the application.

use slog::{self, DrainExt, Level, Record};
use slog_scope;
use slog_term;


const DEFAULT_LEVEL: Level = Level::Info;

// Arrays of log levels, indexed by verbosity.
const POSITIVE_VERBOSITY_LEVELS: &'static [Option<Level>] = &[
    Some(DEFAULT_LEVEL),
    Some(Level::Debug),
    Some(Level::Trace),
];
const NEGATIVE_VERBOSITY_LEVELS: &'static [Option<Level>] = &[
    Some(DEFAULT_LEVEL),
    Some(Level::Warning),
    Some(Level::Error),
    Some(Level::Critical),
    None,  // No logging at all.
];


/// Initialize logging with given verbosity.
/// The verbosity value has the same meaning as in args::Options::verbosity.
pub fn init(verbosity: isize) {
    // TODO: use slog_stream crate to better control log formatting;
    // example: https://github.com/slog-rs/misc/blob/master/examples/global_file_logger.rs
    let stderr = slog_term::streamer().sync().stderr()
        .use_custom_timestamp(move |io| write!(io, ""));  // No log timestamps.

    // Determine the log filtering level based on verbosity.
    // If the argument is excessive, log that but clamp to the highest/lowest log level.
    let mut verbosity = verbosity;
    let mut excessive = false;
    let level = if verbosity >= 0 {
        if verbosity >= POSITIVE_VERBOSITY_LEVELS.len() as isize {
            excessive = true;
            verbosity = POSITIVE_VERBOSITY_LEVELS.len() as isize - 1;
        }
        POSITIVE_VERBOSITY_LEVELS[verbosity as usize]
    } else {
        verbosity = -verbosity;
        if verbosity >= NEGATIVE_VERBOSITY_LEVELS.len() as isize {
            excessive = true;
            verbosity = NEGATIVE_VERBOSITY_LEVELS.len() as isize - 1;
        }
        NEGATIVE_VERBOSITY_LEVELS[verbosity as usize]
    };

    // Create the logger with the correct verbosity filter applied.
    let logger = match level {
        Some(ref level) => {
            let level = level.clone();
            let drain = slog::filter(
                move |record: &Record| record.level().is_at_least(level),
                stderr.build());
            slog::Logger::root(drain.fuse(), o!())
        },
        None => slog::Logger::root(slog::Discard, o!()),
    };

    // Initialize logging, possibly mentioning the excessive verbosity option.
    slog_scope::set_global_logger(logger);
    if excessive {
        warn!("-v/-q flag passed too many times";
            "final_log_level" => level.map(|l| l.as_str()).unwrap_or("None"));
    }
}
