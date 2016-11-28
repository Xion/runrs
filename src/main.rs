//!
//! runrs -- Runner for Rust "scripts"
//!

             extern crate clap;
             extern crate conv;
#[macro_use] extern crate custom_derive;
             extern crate crypto;
#[macro_use] extern crate enum_derive;
#[macro_use] extern crate lazy_static;
             extern crate regex;
#[macro_use] extern crate slog;
#[macro_use] extern crate slog_scope;
             extern crate slog_term;
             extern crate toml;


mod args;
mod logging;


use std::borrow::Cow;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

use crypto::digest::Digest;
use crypto::sha1::Sha1;


lazy_static! {
    /// Application version, as filled out by Cargo.
    pub static ref VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
}

lazy_static! {
    /// Main application's directory.
    static ref APP_DIR: PathBuf =
        env::home_dir().unwrap_or_else(|| env::temp_dir()).join(".runrs");
    // TODO: use the app_dirs crate to get this in a more portable way

    /// Directory where the Cargo workspace is located.
    ///
    /// Cargo.toml here will have the [workspace] section containing paths
    /// to previously ran scripts.
    static ref WORKSPACE_DIR: PathBuf = APP_DIR.join("workspace");
}


fn main() {
    let opts = args::parse().unwrap_or_else(|e| {
        write!(&mut io::stderr(), "{}", e).unwrap();  // Error contains the usage string.
        exit(64);  // EX_USAGE
    });

    logging::init(opts.verbosity);
    debug!("Initializing runrs"; "version" => VERSION.unwrap_or("UNKNOWN"));

    ensure_app_dir();
    ensure_workspace();

    let ref script = opts.script;
    info!("Running script"; "path" => script.display().to_string());
    let script_crate_dir = ensure_script_crate(script);

    // TODO: pass the --release flag
    cargo_run(script_crate_dir, &opts.args);
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

    // This initial content of Cargo.toml will be modified whenever a new script crate is added,
    // by adding the crate's relative path (SHA) to [workspace.members].
    writeln!(&mut cargo_toml_fp, "[workspace]\nmembers = []").unwrap();
}

/// Ensure that a crate for given Rust script exists within the workspace.
/// Returns the path to the crate's directory.
fn ensure_script_crate<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();

    // TODO: if there is a shebang in the script (like #!/usr/bin/runrs), exclude
    // it from SHA-ing and do not carry it over when copying the script file to
    // its crate
    let sha_hex = sha1_file(path).unwrap_or_else(|err| {
        error!("Failed to compute SHA of the script";
            "path" => path.display().to_string(), "error" => format!("{}", err));
        exit(72);  // EX_OSFILE
    }).result_str();

    // TODO: shard by 2-char prefix, like Git blobs
    let crate_dir = WORKSPACE_DIR.join(sha_hex.clone());
    let cargo_toml = crate_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        trace!("Script crate already exists, skipping creation";
            "script" => path.display().to_string(), "sha" => sha_hex);
        return crate_dir;
    }

    if crate_dir.exists() {
        warn!("Script crate directory found without Cargo.toml inside";
            "dir" => crate_dir.display().to_string());
    } else {
        debug!("Initializing the script crate";
            "script" => path.display().to_string(), "sha" => sha_hex);

        // Add the new script crate path to [workspace.members] of the root Cargo.toml.
        // Since this root is "virtual" (i.e. doesn't correspond to any crate on its own),
        // this is the only way to define the workspace.
        //
        // Note that we do this before actually creating the script crate via `cargo new`
        // because it prevents Cargo from emitting a warning about workspace misconfiguration.
        trace!("Fixing root Cargo.toml to point to the script crate";
            "crate_dir" => crate_dir.display().to_string());
        {
            let root_cargo_toml = WORKSPACE_DIR.join("Cargo.toml");
            let mut fp = File::open(&root_cargo_toml).unwrap();
            let mut content = String::new();
            fp.read_to_string(&mut content).unwrap();

            let mut root: toml::Value = content.parse().unwrap();
            {
                let ws_members = root.lookup_mut("workspace.members").unwrap();
                let mut ws_members_vec: Vec<_> = ws_members.as_slice().unwrap().to_owned();
                ws_members_vec.push(toml::Value::String(sha_hex.clone()));
                *ws_members = toml::Value::Array(ws_members_vec);
                // TODO: prevent duplicates
            }

            let mut fp = fs::OpenOptions::new().write(true).open(&root_cargo_toml).unwrap();
            write!(&mut fp, "{}", toml::encode_str(&root)).unwrap();
        }

        // Run `cargo new --bin $SCRIPT_SHA` in the workspace directory
        // to actually create the script crate.
        let package_name: Cow<str> = match path.file_stem().and_then(|s| s.to_str()) {
            // Package name must be unique across the workspace,
            // so we'll use the SHA in it as well.
            Some(stem) => Cow::Owned(format!("{}-{}", stem, sha_hex)),
            None => Cow::Borrowed(&sha_hex),
        };
        let mut cargo_cmd = Command::new("cargo");
        cargo_cmd.arg("new")
            .arg("--bin")
            .args(&["--vcs", "none"])
            .args(&["--name", &*package_name])
            // TODO: only colorize if stdin is a tty
            .args(&["--color", "always"])
            .current_dir(WORKSPACE_DIR.clone())
            .arg(&sha_hex);

        trace!("Running `cargo new` for the script crate";
            "sha" => sha_hex, "name" => &*package_name, "cmd" => format!("{:?}", cargo_cmd));
        let cargo_proc = cargo_cmd.spawn().unwrap_or_else(|err| {
            error!("Failed to run cargo";
                "cmd" => format!("{:?}", cargo_cmd), "error" => format!("{}", err));
            exit(2);
        });
        let output = cargo_proc.wait_with_output().unwrap();
        if !output.status.success() {
            error!("cargo returned an error";
                "cmd" => format!("{:?}", cargo_cmd), "status" => format!("{}", output.status));
            io::stderr().write(&output.stderr).unwrap();
            exit(2);
        }

        // Extract the script's dependencies from the `extern crate` declarations
        // and then add them to [dependencies] of the script's Cargo.toml.
        lazy_static! {
            // TODO: this is of course a fragile way to do this; use the `syn` crate
            // to parse the script into Rust AST and pick the decls from that
            static ref EXTERN_CRATE_RE: regex::Regex = regex::Regex::new(
                r"extern\s+crate\s+(?P<name>\w+)\s*;"
            ).unwrap();
        }
        let deps = {
            // TODO: isn't there a create to read file contents?...
            // (also, this is not the first time this file is read here)
            let mut fp = File::open(path).unwrap();
            let mut content = String::new();
            fp.read_to_string(&mut content).unwrap();
            EXTERN_CRATE_RE.captures_iter(&content)
                .map(|cap| cap.name("name").unwrap().to_owned()).collect::<Vec<_>>()
        };
        trace!("Extracted dependencies of the script";
            "path" => path.display().to_string(), "deps" => format!("{:?}", deps));
        // TODO: consider a way to specify deps versions (like a comment or something)

        if !deps.is_empty() {
            let mut fp = File::open(&cargo_toml).unwrap();
            let mut content = String::new();
            fp.read_to_string(&mut content).unwrap();

            let mut root: toml::Value = content.parse().unwrap();
            {
                // Ain't the toml crate's interface delightful?
                let deps_value = root.lookup_mut("dependencies").unwrap();
                let mut deps_map = deps_value.as_table().unwrap().to_owned();
                for dep in deps {
                    deps_map.insert(dep, toml::Value::String("*".into()));
                }
                *deps_value = toml::Value::Table(deps_map);
            }

            let mut fp = fs::OpenOptions::new().write(true).open(&cargo_toml).unwrap();
            write!(&mut fp, "{}", toml::encode_str(&root)).unwrap();
        }

        debug!("Script crate initialized successfully";
            "script" => path.display().to_string(), "sha" => sha_hex);
    }

    // Copy the script into the crate's directory as its main.rs.
    // TODO: remove any shebangs
    let main_rs = crate_dir.join("src").join("main.rs");
    trace!("Copying script as src/main.rs";
        "from" => path.display().to_string(), "to" => main_rs.display().to_string());
    fs::copy(path, main_rs.clone()).unwrap_or_else(|err| {
        error!("Failed to copy the script into crate src/";
            "script" => path.display().to_string(), "target" => main_rs.display().to_string(),
            "error" => format!("{}", err));
       exit(72);  // EX_OSFILE
    });

    crate_dir
}

/// Compute SHA1 hash of the contents of given file.
fn sha1_file<P: AsRef<Path>>(path: P) -> io::Result<Sha1> {
    let path = path.as_ref();
    let mut file = try!(File::open(path));
    let mut sha = Sha1::new();

    // TODO: feed the file contents to the hasher gradually rather than all at once,
    // to handle files of ludicrous sizes
    let mut contents = Vec::new();
    let size = try!(file.read_to_end(&mut contents));
    sha.input(&contents);

    trace!("SHA1 of a file";
        "path" => path.display().to_string(), "size" => size, "sha" => sha.result_str());
    Ok(sha)
}

/// Execute `cargo run` within given directory.
/// Regardless whether or not it succceeds, this function does not return.
fn cargo_run<P: AsRef<Path>>(path: P, args: &[String]) -> ! {
    let path = path.as_ref();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(path.clone())
        .arg("run").arg("--quiet");
    if !args.is_empty() {
        cmd.arg("--").args(args);
    }

    trace!("About to `cargo run`";
        "dir" => path.display().to_string(), "cmd" => format!("{:?}", cmd));

    // On Unix, we can replace the app's process completely with Cargo
    // but on Windows, we have to run its as a child process and wait for it.
    if cfg!(unix) {
        use std::os::unix::process::CommandExt;

        // This calls execvp() and doesn't return unless an error occurred.
        let error = cmd.exec();
        debug!("`cargo run` failed";
            "dir" => path.display().to_string(), "error" => format!("{}", error));

        panic!("Failed to execute the script: {}", error);
    } else {
        let mut run = cmd.spawn()
            .unwrap_or_else(|e| panic!("Failed to execute the script: {}", e));

        // Propagate the same exit code that Cargo -- and conversely, the script -- returned.
        let exit_status = run.wait().unwrap_or_else(|e| {
            panic!("Failed to obtain status code for the script: {}", e)
        });
        let exit_code = exit_status.code().unwrap_or(75);  // EX_TEMPFAIL
        exit(exit_code);
    }
}
