//! Module for managing the shared Cargo workspace used by scripts we run.

use std::borrow::Cow;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

use crypto::digest::Digest;
use regex::Regex;
use toml;

use super::APP_DIR;
use util::{self, exitcode};


lazy_static! {
    /// Directory where the Cargo workspace is located.
    ///
    /// Cargo.toml here will have the [workspace] section containing paths
    /// to previously ran scripts.
    pub static ref WORKSPACE_DIR: PathBuf = APP_DIR.join("workspace");
}


// TODO: make the functions here result a Result rather than exiting on errors


/// Ensure that the root Cargo workspace exists.
///
/// All the scripts being executed are crates under that workspace
/// and share the same Cargo.lock. This prevents from rebuiding shared dependencies
/// repeatedly, thus massively speeding up the execution of scripts.
pub fn ensure_workspace() {
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
            exit(exitcode::EX_OSFILE);
        });
    }

    let mut cargo_toml_fp = fs::OpenOptions::new()
        .write(true).create_new(true)
        .open(cargo_toml.clone()).unwrap_or_else(|err| {
            error!("Failed to open Cargo.toml of script workspace";
                "path" => cargo_toml.display().to_string(), "error" => format!("{}", err));
            exit(exitcode::EX_OSFILE);
        });

    // This initial content of Cargo.toml will be modified whenever a new script crate is added,
    // by adding the crate's relative path (SHA) to [workspace.members].
    writeln!(&mut cargo_toml_fp, "[workspace]\nmembers = []").unwrap();
}


/// Ensure that a crate for given Rust script exists within the workspace.
/// Returns the path to the crate's directory.
pub fn ensure_script_crate<P: AsRef<Path>>(path: P) -> PathBuf {
    // TODO: split this function

    let path = path.as_ref();

    // TODO: if there is a shebang in the script (like #!/usr/bin/runrs), exclude
    // it from SHA-ing and do not carry it over when copying the script file to
    // its crate
    let sha_hex = util::sha1_file(path).unwrap_or_else(|err| {
        error!("Failed to compute SHA of the script";
            "path" => path.display().to_string(), "error" => format!("{}", err));
        exit(exitcode::EX_OSFILE);
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
            let content = util::read_text_file(&root_cargo_toml).unwrap();

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
            static ref EXTERN_CRATE_RE: Regex = Regex::new(
                r"extern\s+crate\s+(?P<name>\w+)\s*;"
            ).unwrap();
        }
        let deps = {
            let content = util::read_text_file(path).unwrap();
            EXTERN_CRATE_RE.captures_iter(&content)
                .map(|cap| cap.name("name").unwrap().to_owned()).collect::<Vec<_>>()
        };
        trace!("Extracted dependencies of the script";
            "path" => path.display().to_string(), "deps" => format!("{:?}", deps));
        // TODO: consider a way to specify deps versions (like a comment or something)

        if !deps.is_empty() {
            let content = util::read_text_file(&cargo_toml).unwrap();
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
       exit(exitcode::EX_OSFILE);
    });

    crate_dir
}
