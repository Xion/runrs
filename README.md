# runrs

Run Rust modules like scripts

## What?

`runrs` lets you execute Rust source files (_*.rs_) as if they were compiled binaries or scripts.

    $ cat >hello.rs
    fn main() {
        println!("Hello, world!");
    }
    ^D
    $ runrs ./hello.rs
    Hello, world!

It can execute any self-contained Rust program, as long as it's a single file with a `main` function.

External crates are supported, too! Just make sure the `extern crate` declarations are in their usual place.

## How?

`runrs` creates an ad-hoc binary crate (`cargo new --bin`) for each new script it runs.

All those crates live within a single
[Cargo _workspace_](https://github.com/rust-lang/rfcs/blob/master/text/1525-cargo-workspace.md).
This allows them to share their dependencies, avoiding repeated recompilation of common library crates.

## Why?

* For easier [scripting](http://www.chriskrycho.com/2016/using-rust-for-scripting.html) with Rust.
* Because Haskell has `runghc` and Rust shouldn't be worse.
* Why not?

## Where to?

This is of course an early prototype and there is clearly a room for improvement:

* handle shebangs correctly
* handle weird crate name abnormalities
  (like dash vs. underscore, or stuff like `extern crate crypto;` translating to _rust-crypto_ crate)
* better interface (e.g. accept Rust code given via stdin)
* tests!
