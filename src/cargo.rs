//! Module wrapping the interactions with Cargo.

use std::path::Path;
use std::process::{Command, exit};

use util::exitcode;


custom_derive! {
    /// Enum describing a Cargo build mode.
    #[derive(Clone, Debug, Eq, PartialEq,
             IterVariants(BuildModes))]
    pub enum BuildMode {
        /// Debug mode.
        Debug,
        /// Release mode (with optimizations). Equivalent of --release in Cargo.
        Release,
    }
}

impl Default for BuildMode {
    fn default() -> Self { BuildMode::Debug }
}


// TODO: make a Cargo wrapper struct where we can pass common options (in a Builder fashion)
// before invoking a specific Cargo command


/// Execute `cargo run` within given directory.
/// Regardless whether or not it succceeds, this function does not return.
pub fn run<P: AsRef<Path>>(path: P, args: &[String], mode: BuildMode) -> ! {
    let path = path.as_ref();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(path.clone())
        .arg("run").arg("--quiet");  // TODO: don't make it --quiet if -v was passed
    if mode == BuildMode::Release {
        cmd.arg("--release");
    }
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
        let exit_code = exit_status.code().unwrap_or(exitcode::EX_TEMPFAIL);
        exit(exit_code);
    }
}
