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

    // TODO for V1:
    // 1. create a temporary directory
    // 2. symlink (or copy?) given file there
    // 3. analyze it for extern-crate declarations
    // 4. create a minimal Cargo.toml with those crates as [dependencies]
    // 5. cd && cargo run

    // V2:
    // 1. use a single Cargo.toml "environment", with different scripts as [[bin]] entries
    //   (to reuse compiled dependencies)
}
