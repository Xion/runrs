//! Utility module.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crypto::digest::Digest;
use crypto::sha1::Sha1;


/// Reads the contents of the file into a String.
pub fn read_text_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let path = path.as_ref();

    let mut fp = try!(File::open(path));
    let mut content = match fp.metadata() {
        Ok(metadata) => {
            trace!("Reading text file";
                "path" => path.display().to_string(), "size" => metadata.len());
            String::with_capacity(metadata.len() as usize + 1)
        },
        Err(err) => {
            warn!("Failed to obtain file size when reading it";
                "path" => path.display().to_string(), "error" => format!("{}", err));
            String::new()
        },
    };

    try!(fp.read_to_string(&mut content));
    Ok(content)
}


/// Compute SHA1 hash of the contents of given file.
pub fn sha1_file<P: AsRef<Path>>(path: P) -> io::Result<Sha1> {
    let path = path.as_ref();

    let mut file = try!(File::open(path));
    let mut sha = try!(digest(Sha1::new(), &mut file));

    trace!("SHA1 of a file";
        "path" => path.display().to_string(),
        "size" => file.metadata().map(|m| m.len().to_string()).unwrap_or("N/A".into()),
        "sha" => sha.result_str());
    Ok(sha)
}

/// Compute one of the crypto crate's digest for bytes read from given input.
/// The input is read until the end.
fn digest<D: Digest, R: Read>(digest: D, mut input: R) -> io::Result<D> {
    // TODO: make a PR to rust-crypto to include Default impl for all digests,
    // so that we can accept it as a generic type argument
    let mut digest = digest;

    const BUF_SIZE: usize = 256;
    let mut buf = [0; BUF_SIZE];
    loop {
        let c = try!(input.read(&mut buf));
        digest.input(&buf[0..c]);
        if c < BUF_SIZE { break }
    }
    Ok(digest)
}


// Module defining standard exit codes that are normally found in POSIX header files.
#[allow(dead_code)]
pub mod exitcode {
    /// Type of the exit codes.
    /// This should be the same as the argument type of std::process::exit.
    pub type ExitCode = i32;

    pub const EX_OK: ExitCode = 0;
    pub const EX_USAGE: ExitCode = 64;
    pub const EX_NOINPUT: ExitCode = 66;
    pub const EX_UNAVAILABLE: ExitCode = 69;
    pub const EX_OSFILE: ExitCode = 72;
    pub const EX_IOERR: ExitCode = 74;
    pub const EX_TEMPFAIL: ExitCode = 75;
}
