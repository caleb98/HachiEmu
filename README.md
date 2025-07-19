# HachiEmu

HachiEmu is a CHIP-8 emulator written in Rust as a project for learning the language. In particular,
it targets the SUPER-CHIP platform which behaves slightly differently from original CHIP-8. I went
with this because it seemed to offer better support for some of the ROMs I was able to find online.

## Running

The easiest way to run HachiEmu is to clone the repo and use `cargo run`:

```bash
git clone git@github.com:caleb98/HachiEmu.git
cd HachiEmu
cargo run ${YOUR_ROM_FILE}
```

## Finding ROMS

Just to be careful about licensing/copyright, no ROMs are included in this repository. However, I
found a lot of neat ones at the following sources:

* https://github.com/mattmikolay/chip-8
* https://github.com/kripod/chip8-roms
* https://johnearnest.github.io/chip8Archive/

A quick Google search will reveal many other sources if you end up needing more!

## Testing

HachiEmu was tested with the excellent [CHIP-8 Test Suite](https://github.com/Timendus/chip8-test-suite)
from [Timendus](https://github.com/Timendus). If you're working on your own CHIP-8 emulator, I
highly recommend checking out the ROMs provided there as they were extremely helpful in smoothing
out the rough edges in my initial implementation and ensuring that everything was working, even in
some of the tricky edge cases.

If you want to verify HachiEmu's functionality, grab the test ROMs there and give them a run! 😊

## Writing Your Own

If this seems like a fun project and you'd like to try writing a CHIP-8 emulator your self, check
out the [Awesome CHIP-8](https://chip-8.github.io/links/) page. It has a ton of info with links to
reference docs, extension specifications, and guides/articles about building a CHIP-8 emulator.