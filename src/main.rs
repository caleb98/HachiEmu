use hachi_emu::Emulator;
use macroquad::{miniquad::conf::Platform, prelude::*};

fn conf() -> Conf {
    Conf {
        window_title: String::from("HachiEmu"),
        window_width: 64 * 12,
        window_height: 32 * 12,
        window_resizable: false,
        // platform: Platform {
        //     swap_interval: Some(0),
        //     ..Default::default()
        // },
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let mut emulator = Emulator::new();
    emulator.load_font(&hachi_emu::STANDARD_FONT);

    let program = std::fs::read("roms/5-quirks.ch8").unwrap();
    emulator.load_program(&program);

    emulator.run().await;
}
