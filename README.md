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

## Resources

#### General

 - [NesDev wiki](wiki.nesdev.org)
 - [/r/EmuDev discord](https://discord.gg/dkmJAes)

#### 6502 CPU

  - [_6502 Assembly Language Programming_](http://www.obelisk.me.uk/6502/index.html) by Andrew Jacobs
  - [_The 6502 Instruction Set Decoded_](http://nparker.llx.com/a2/opcodes.html) by Neil Parker
    - This includes undocumented opcodes for the Apple II, which don't apply to
      the 2A03 used by the NES.
  - [Rockwell R650x datasheet](http://archive.6502.org/datasheets/rockwell_r650x_r651x.pdf)
  - [MOS MCS6501 datasheet](http://archive.6502.org/datasheets/mos_6501-6505_mpu_preliminary_aug_1975.pdf)
    - This scan has the highest resolution opcode table I can find
  - [nestest](http://www.qmtpro.com/~nes/misc/nestest.txt)
    - Used as a unit test for verifying cycle-count accuracy and functionality.
