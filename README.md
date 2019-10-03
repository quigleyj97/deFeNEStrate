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
| ~~`0x1A`~~  | ~~`AND`~~ FIXED                                 |
| ~~`0x1E`~~  | ~~`ADC` overflow/carry~~ FIXED                  |
| `0x1F`      | `ADC` BCD flag                                  |
| `0x20`      | `ADC` other failure                             |
| `0x21`      | `ADC`                                           |
| `0x22`      | `ADC`                                           |
| `0x23`      | `LDA` flag failure                              |
| `0x24`      | `LDA` flag failure                              |
| `0x26`      | `CMP` flag failure                              |
| `0x27`      | `CMP` flag failure                              |
| `0x28`      | `CMP` flag failure                              |
| `0x2B`      | `CPY` flag failure                              |
| `0x2D`      | `CPY` flag failure                              |
| `0x2E`      | `CPY` flag failure                              |
| `0x2F`      | `CPY` flag failure                              |
| `0x32`      | `CPX` flag failure                              |
| `0x34`      | `CPX` flag failure                              |
| `0x35`      | `CPX` flag failure                              |
| `0x36`      | `CPX` flag failure                              |
| `0x3A`      | `LDX` flag failure                              |
| `0x3C`      | `LDY` flag failure                              |
| `0x71`      | `SBC`                                           |
| `0x72`      | `SBC`                                           |
| `0x73`      | `SBC`                                           |
| `0x74`      | `SBC`                                           |
| `0x3F`      | `INY`/`DEY` flag failure                        |
| `0x40`      | `INX`/`DEX` flag failure                        |
| `0x41`      | `TAY` did something bad                         |
| `0x42`      | `TAX` did something bad                         |
| `0x43`      | `TYA` did something bad                         |
| `0x44`      | `TXA` did something bad                         |
| `0x45`      | `TXS` did something bad, or flag failure        |
| `0x47`      | `JSR` didn't work as expected                   |
