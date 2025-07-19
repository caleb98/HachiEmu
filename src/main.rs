use std::env;

use hachi_emu::Emulator;
use macroquad::prelude::*;

fn conf() -> Conf {
    Conf {
        window_title: String::from("HachiEmu"),
        window_width: 64 * 12,
        window_height: 32 * 12,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} <rom-file>", args[0]);
        return;
    }

    let rom_name = &args[1];

    let mut emulator = Emulator::new();
    emulator.load_font(&hachi_emu::STANDARD_FONT);

    let program = std::fs::read(rom_name).unwrap();
    emulator.load_program(&program);

    emulator.run().await;
}
