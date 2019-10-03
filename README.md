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
| ~~`0x1F`~~  | ~~`ADC` BCD flag~~ FIXED                        |
| ~~`0x20`~~  | ~~`ADC` other failure~~ FIXED                   |
| ~~`0x21`~~  | ~~`ADC`~~ FIXED                                 |
| ~~`0x22`~~  | ~~`ADC`~~ FIXED                                 |
| ~~`0x23`~~  | ~~`LDA` flag failure~~ FIXED                    |
| ~~`0x24`~~  | ~~`LDA` flag failure~~ FIXED                    |
| ~~`0x26`~~  | ~~`CMP` flag failure~~ FIXED                    |
| ~~`0x27`~~  | ~~`CMP` flag failure~~ FIXED                    |
| ~~`0x28`~~  | ~~`CMP` flag failure~~ FIXED                    |
| ~~`0x29`~~  | ~~`CMP` flag failure~~ FIXED                    |
| ~~`0x2B`~~  | ~~`CPY` flag failure~~ FIXED                    |
| ~~`0x2D`~~  | ~~`CPY` flag failure~~ FIXED                    |
| ~~`0x2E`~~  | ~~`CPY` flag failure~~ FIXED                    |
| ~~`0x2F`~~  | ~~`CPY` flag failure~~ FIXED                    |
| ~~`0x32`~~  | ~~`CPX` flag failure~~ FIXED                    |
| ~~`0x34`~~  | ~~`CPX` flag failure~~ FIXED                    |
| ~~`0x35`~~  | ~~`CPX` flag failure~~ FIXED                    |
| ~~`0x36`~~  | ~~`CPX` flag failure~~ FIXED                    |
| ~~`0x3A`~~  | ~~`LDX` flag failure~~ FIXED                    |
| ~~`0x3C`~~  | ~~`LDY` flag failure~~ FIXED                    |
| ~~`0x71`~~  | ~~`SBC`~~ FIXED                                 |
| ~~`0x72`~~  | ~~`SBC`~~ FIXED                                 |
| ~~`0x73`~~  | ~~`SBC`~~ FIXED                                 |
| ~~`0x74`~~  | ~~`SBC`~~ FIXED                                 |
| ~~`0x75`~~  | ~~`SBC`~~ FIXED                                 |
| `0x3E`      | `INX`/`DEX`/`INY`/`DEY` did something bad NEW   |
| `0x3F`      | `INY`/`DEY` flag failure                        |
| ~~`0x40`~~  | ~~`INX`/`DEX` flag failure~~ FIXED              |
| ~~`0x41`~~  | ~~`TAY` did something bad~~ FIXED               |
| ~~`0x42`~~  | ~~`TAX` did something bad~~ FIXED               |
| ~~`0x43`~~  | ~~`TYA` did something bad~~ FIXED               |
| ~~`0x44`~~  | ~~`TXA` did something bad~~ FIXED               |
| ~~`0x45`~~  | ~~`TXS` did something bad, or flag f...~~ FIXED |
| `0x47`      | `JSR` didn't work as expected                   |
