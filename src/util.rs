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
        Ok(metadata) => String::with_capacity(metadata.len() as usize),
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
