//! runst -- Runner for Rust "scripts"


use std::env;
use std::io::{self, Write};


fn main() {
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
