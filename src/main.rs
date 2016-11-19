//!
//! runst -- Runner for Rust "scripts"
//!

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate slog;
#[macro_use] extern crate slog_scope;
             extern crate slog_term;


use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;


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
    // TODO: parse command line flags, including logging verbosity
    logging::init();
    debug!("Initializing runst"; "version" => VERSION.unwrap_or("UNKNOWN"));

    let args: Vec<String> = env::args().skip(1).collect();
    trace!("Parsing command line arguments"; "args" => format!("{:?}", args));
    if args.len() == 0 {
        error!("No script name provided");
        exit(2);
    }
    let ref script = args[0];

    ensure_app_dir();
    ensure_workspace();

    info!("Running script"; "path" => *script);

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

/// Ensure that the application directory exists.
fn ensure_app_dir() {
    if APP_DIR.exists() {
        trace!("Application directory exists, skipping creation";
            "app_dir" => APP_DIR.display().to_string());
        return;
    }

    trace!("Creating application directory"; "app_dir" => APP_DIR.display().to_string());
    fs::create_dir_all(&*APP_DIR).unwrap_or_else(|err| {
        error!("Failled to create application directory";
            "app_dir" => APP_DIR.display().to_string(), "error" => format!("{}", err));
        exit(72);  // EX_OSFILE
    });
    debug!("Application directory created"; "app_dir" => APP_DIR.display().to_string());
}

/// Ensure that the root Cargo workspace exists.
///
/// All the scripts being executed are crates under that workspace
/// and share the same Cargo.lock. This prevents from rebuiding shared dependencies
/// repeatedly, thus massively speeding up the execution of scripts.
fn ensure_workspace() {
    let cargo_toml = WORKSPACE_DIR.join("Cargo.toml");
    if cargo_toml.exists() {
        trace!("Script workspace exists, skipping creation";
            "dir" => WORKSPACE_DIR.display().to_string());
        return;
    }

    if WORKSPACE_DIR.exists() {
        warn!("Script workspace directory found without Cargo.toml inside";
            "dir" => WORKSPACE_DIR.display().to_string());
    } else {
        fs::create_dir_all(&*WORKSPACE_DIR).unwrap_or_else(|err| {
            error!("Failed to create script workspace directory";
                "dir" => WORKSPACE_DIR.display().to_string(), "error" => format!("{}", err));
            exit(72);  // EX_OSFILE
        });
    }

    let mut cargo_toml_fp = fs::OpenOptions::new()
        .write(true).create_new(true)
        .open(cargo_toml.clone()).unwrap_or_else(|err| {
            error!("Failed to open Cargo.toml of script workspace";
                "path" => cargo_toml.display().to_string(), "error" => format!("{}", err));
            exit(72);  // EX_OSFILE
        });

    // This is the only thing necerssary to define the workspace.
    // Cargo itself should handle everything else.
    writeln!(&mut cargo_toml_fp, "[workspace]").unwrap();
}


mod logging {
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
}
