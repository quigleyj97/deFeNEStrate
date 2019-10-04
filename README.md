# deFeNEStrate

A NES emulator written in Rust

## Building

Nothing unusal in building, (at least not yet). Just run `cargo run` to build
and run deFeNEStrate.

Some basic tests are included, you can run them with `cargo test -- --nocapture`.
The integration tests will spit out a Nintendulator-formatted instruction log
that can be compared with a known-good emulator log. I'm working on having the
test diff against a good log, but I haven't yet matched the instruction
formatting nor have I implemented PPU registers, so you can't use a line-by-line
equivalency check (yet).

## Implementation Plan

6502 Emulator
 - [x] ~~Basic Addressing Modes~~
 - [x] ~~Basic Opcodes~~
 - [ ] Full impl. [ in progress ]
 - [ ] Testing

2A03 specialization
 - [ ] APU
   - [ ] Function generators
   - [ ] DMC Sampler

NES work
 - [x] ~~Move bus ownership to something easier to work with (like CPU)~~
 - [ ] PPU
 - [ ] Cartridge loading
   - [x] ~~Simple mapper implementations~~
   - [x] ~~Load from file~~
     - [ ] Generic load from file
   - [ ] Load from byte arr
 - [ ] Playability testing

## Test status

Known Nestest failures:

| Error code  | Reason                                          |
|-------------|-------------------------------------------------|
| `0x4C`      | ROR,A failed                                    |
| `0x4D`      | ROL,A failed                                    |
| ...         | ...                                             |