//!
//! runrs -- Runner for Rust "scripts"
//!

             extern crate clap;
             extern crate conv;
#[macro_use] extern crate custom_derive;
             extern crate crypto;
#[macro_use] extern crate enum_derive;
             extern crate isatty;
#[macro_use] extern crate lazy_static;
             extern crate regex;
#[macro_use] extern crate slog;
#[macro_use] extern crate slog_scope;
             extern crate slog_term;
             extern crate toml;


mod args;
mod cargo;
mod logging;
mod util;
mod workspace;


use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::exit;

use util::exitcode;


lazy_static! {
    /// Application version, as filled out by Cargo.
    pub static ref VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
}

lazy_static! {
    /// Main application's directory.
    pub static ref APP_DIR: PathBuf =
        env::home_dir().unwrap_or_else(|| env::temp_dir()).join(".runrs");
    // TODO: use the app_dirs crate to get this in a more portable way
}


fn main() {
    let opts = args::parse().unwrap_or_else(|e| {
        write!(&mut io::stderr(), "{}", e).unwrap();  // Error contains the usage string.
        exit(exitcode::EX_USAGE);
    });

    logging::init(opts.verbosity);
    debug!("Initializing runrs"; "version" => VERSION.unwrap_or("UNKNOWN"));

    ensure_app_dir();
    workspace::ensure_workspace();

    let ref script = opts.script;
    info!("Running script"; "path" => script.display().to_string());
    let script_crate_dir = workspace::ensure_script_crate(script);

    cargo::run(script_crate_dir, &opts.args, opts.build_mode);
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
        exit(exitcode::EX_OSFILE);
    });
    debug!("Application directory created"; "app_dir" => APP_DIR.display().to_string());
}
