# Note

This project is no longer under development. I'm leaving it up as a hopefully
helpful reference for anyone else wishing to write a NES emulator, but after
reading [this article](http://way-cooler.org/blog/2019/04/29/rewriting-way-cooler-in-c.html)
I came to understand that a lot of what I've been working on in this
implementation is not emulator correctness or functionality, but Rust wrappings.
Part of this is because I'm still learning the language, but on the other hand
it's exhausting to have to write out `Rc<MutCell<T>>` and `Pin<Box<dyn T>>` all
the time.

In this particular case, an emulator _requires_ a very intertangled 
design- the very functionality of the system demands it!  I cannot simply
factor this design without significant boilerplate demanded by `rustc`. That's
not to say that this is impossible, as the numerous other successful takes on
Rust emulators prove. But the effort involved saps some of the fun for me, and
I'm not writing an emulator to learn linear logic.

This is not to say that Rust is bad, or that Rust is stupid. Much the opposite,
Rust is designed and developed by very talented engineers and has accomplished
or powers a vast array of technical feats whose impact cannot be understated.
The takeaway is that it's not an appropriate tool for every task (as is true of
most languages), and that my goals are better met in a different language.

The primary goals I've had in mind for this project have been:

 - Device portability
 - Debuggability

With that in mind, I will rewrite this project in TypeScript and continue
development at [ne.ts](quigleyj97/ne.ts). TS is a natural fit since low-level JS
maps well to what I've already written, and the typings provided can continue to
assure safety. Performance concerns are outweighed by the fact that this can be
deployed to literally any device with a browser, and it will be much easier to
write UIs leveraging rich HTML.

# deFeNEStrate

A NES emulator written in Rust

## Building

Nothing unusal in building, (at least not yet). Just run `cargo run` to build
and run deFeNEStrate.

Some basic tests are included, you can run them with `cargo test -- --nocapture`.
The integration tests will spit out a Nintendulator-formatted instruction log
that can be compared with a known-good emulator log.

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

#### Assets

 - Droid Sans Mono, licensed under [Apache 2.0](./static/Apache License.txt)
