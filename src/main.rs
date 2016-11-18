//!
//! runst -- Runner for Rust "scripts"
//!

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate slog;
             extern crate slog_stdlog;
             extern crate slog_term;

// `log` crate has to after `slog` as we want to use logging macros from `log`.
#[macro_use] extern crate log;


use std::env;
use std::io::{self, Write};
use std::path::PathBuf;


lazy_static! {
    /// Application version, as filled out by Cargo.
    pub static ref VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
}

lazy_static! {
    /// Main application's directory.
    static ref APP_DIR: PathBuf =
        env::home_dir().unwrap_or_else(|| env::temp_dir()).join(".runst");
    // TODO: use the app_dirs crate to get this in a more portable way

    /// Directory where the Cargo workspace is located.
    ///
    /// Cargo.toml here will have the [workspace] section containing paths
    /// to previously ran scripts.
    static ref WORKSPACE_DIR: PathBuf = APP_DIR.join("workspace");
}


fn main() {
    logging::init().unwrap();
    trace!("runst {}", VERSION.map(|v| format!("v{}", v))
        .unwrap_or_else(|| "(UNKNOWN VERSION)".to_owned()));

    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 0 {
        panic!("No script name provided");
    }
    let ref script = args[0];

    writeln!(&mut io::stderr(), "Running {}...", script).unwrap();

    // TODO:
    // 1. create a directory for the cargo [workspace] if it doesn't exist
    // 2. hash the file contents (minus hashbang) and put it under there
    //    (possibly in sharded subdirs like Git does with blobs)
    // 3. generate the boilerplate Cargo.toml and put along with input .rs
    // 4. add the new crate to [workspace] in root Cargo.toml
    // 5. cd && cargo run
    // The [workspace] thingie will allow for reusing compiled dependencies
    // via a single Cargo.lock
}


mod logging {
    use log::SetLoggerError;
    use slog::{self, DrainExt};
    use slog_stdlog;
    use slog_term;

    use super::VERSION;

    pub fn init () -> Result<(), SetLoggerError> {
        let logger = slog::Logger::root(
            slog_term::streamer().stderr().build().fuse(),
            o!("version" => VERSION.unwrap_or("UNKNOWN")));
        slog_stdlog::set_logger(logger)
    }
}
