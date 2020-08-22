# deFeNEStrate

A NES emulator written in Rust, with build targets for desktop and WebAssembly

## Building

Nothing unusal in building, (at least not yet). Just run `cargo run` to build
and run deFeNEStrate.

Some basic tests are included, you can run them with `cargo test -- --nocapture`.
The integration tests will spit out a Nintendulator-formatted instruction log
that can be compared with a known-good emulator log.

## Assets

 - Droid Sans Mono, licensed under [Apache 2.0](./static/Apache License.txt)
