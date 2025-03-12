use std::collections::HashMap;

use bit_set::BitSet;
use macroquad::{prelude::*, texture::Image};
use ::rand::random_range;

const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;
const MEMORY_BYTES: usize = 4096;
const INITIAL_STACK_SIZE: usize = 64;
const TARGET_OPS_PER_SECOND: u16 = 650;
const TIMER_HZ : f32 = 60.0;

const ROM_LOAD_INDEX: usize = 0x0200; // Memory location where roms are loaded from

type FontData = [u8; 80];
const FONT_LOAD_INDEX: usize = 0x0000;
pub const STANDARD_FONT: FontData = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

enum KeyState {
    Inactive,
    Active,
    JustPressed,
    JustReleased,
}

pub struct Emulator {
    memory: [u8; MEMORY_BYTES],
    registers: [u8; 16],
    index_register: usize,
    program_counter: usize,
    stack: Vec<u16>,

    delay_timer: u8,
    sound_timer: u8,

    screen: BitSet,
    key_states: HashMap<KeyCode, KeyState>,
    awaiting_keypress: bool,
    awaiting_keypress_register: usize,
}

impl Emulator {

    pub fn new() -> Emulator {
        Emulator {
            memory: [0; MEMORY_BYTES],
            registers: [0; 16],
            index_register: 0,
            program_counter: 0,
            stack: Vec::with_capacity(INITIAL_STACK_SIZE),

            delay_timer: 0,
            sound_timer: 0,

            screen: BitSet::with_capacity(SCREEN_WIDTH * SCREEN_HEIGHT),
            key_states: HashMap::from([
                (KeyCode::Key1, KeyState::Inactive),
                (KeyCode::Key2, KeyState::Inactive),
                (KeyCode::Key3, KeyState::Inactive),
                (KeyCode::Key4, KeyState::Inactive),
                (KeyCode::Q,    KeyState::Inactive),
                (KeyCode::W,    KeyState::Inactive),
                (KeyCode::E,    KeyState::Inactive),
                (KeyCode::R,    KeyState::Inactive),
                (KeyCode::A,    KeyState::Inactive),
                (KeyCode::S,    KeyState::Inactive),
                (KeyCode::D,    KeyState::Inactive),
                (KeyCode::F,    KeyState::Inactive),
                (KeyCode::Z,    KeyState::Inactive),
                (KeyCode::X,    KeyState::Inactive),
                (KeyCode::C,    KeyState::Inactive),
                (KeyCode::V,    KeyState::Inactive),
            ]),
            awaiting_keypress: false,
            awaiting_keypress_register: 0,
        }
    }

    pub fn load_program(&mut self, data: &[u8]) {
        for (index, value) in data.iter().enumerate() {
            self.memory[ROM_LOAD_INDEX + index] = *value;
        }
    }

    pub fn load_font(&mut self, font_data: &FontData) {
        for (index, value) in font_data.iter().enumerate() {
            self.memory[FONT_LOAD_INDEX + index] = *value;
        }
    }

    pub async fn run(&mut self) {
        // Reset existing state
        self.program_counter = ROM_LOAD_INDEX;
        self.index_register = 0;

        let mut image = Image::gen_image_color(SCREEN_WIDTH as u16, SCREEN_HEIGHT as u16, BLACK);
        let texture = Texture2D::from_image(&image);
        texture.set_filter(FilterMode::Nearest);
        
        let target_cycle_time = 1.0 / TARGET_OPS_PER_SECOND as f32;
        let mut update_time = 0.0;

        let target_timer_time = 1.0 / TIMER_HZ;
        let mut timer_time = 0.0;

        loop {
            // Update the timers
            timer_time -= get_frame_time();
            while timer_time <= 0.0 {
                timer_time += target_timer_time;

                if let Some(new_delay_timer) = self.delay_timer.checked_sub(1) {
                    self.delay_timer = new_delay_timer;
                }

                if let Some(new_sound_timer) = self.sound_timer.checked_sub(1) {
                    self.sound_timer = new_sound_timer;
                }
            }

            // Do actual CPU cycle updates
            update_time -= get_frame_time();
            clear_background(BLACK);
            self.update_key_states();

            while update_time <= 0.0 {
                update_time += target_cycle_time;

                if self.awaiting_keypress {
                    match self.get_awaited_key() {
                        Some(keycode) => {
                            self.registers[self.awaiting_keypress_register] = keycode;
                            self.awaiting_keypress = false;
                            self.awaiting_keypress_register = 0;
                        },
                        None => continue,
                    }
                }

                // Grab the next instruction and increment the program counter
                let high = self.memory[self.program_counter] as u16;
                let low = self.memory[self.program_counter + 1] as u16;
                let instruction = (high << 8) | low;
                self.program_counter += 2;
    
                // Extract some common pieces of the instruction
                let x      = ((instruction & 0x0F00) >> 8) as usize; // 4-bit register id
                let y      = ((instruction & 0x00F0) >> 4) as usize; // 4-bit register id
                let n      = ((instruction & 0x000F) >> 0) as u8;    // 4-bit constant
                let nn     = ((instruction & 0x00FF) >> 0) as u8;    // 8-bit constant
                let nnn    = ((instruction & 0x0FFF) >> 0) as usize; // address

                let nibbles = (
                    (instruction & 0xF000) >> 12 as u8,
                    (instruction & 0x0F00) >>  8 as u8,
                    (instruction & 0x00F0) >>  4 as u8,
                    (instruction & 0x000F) >>  0 as u8,
                );
        
                match nibbles {
                    (0x0, 0x0, 0xE, 0x0) => self.op_00e0(), // 00E0 Display - Clears the screen
                    (0x0, 0x0, 0xE, 0xE) => self.op_00ee(), // 00EE Flow - Return from subroutine
                    (0x0, 0x1,   _,   _) => self.op_0nnn(), // 0NNN Call - Calls a machine code routine
                    (0x1,   _,   _,   _) => self.op_1nnn(nnn), // 1NNN Flow - Goto NNN
                    (0x2,   _,   _,   _) => self.op_2nnn(nnn), // 2NNN Flow - Calls subroutine at NNN
                    (0x3,   _,   _,   _) => self.op_3xnn(x, nn), // 3XNN Cond - Skips the next instruction if VX equals NN
                    (0x4,   _,   _,   _) => self.op_4xnn(x, nn), // 4XNN Cond - Skips the next instruction if VX does not equal NN
                    (0x5,   _,   _,   _) => self.op_5xy0(x, y), // 5XY0 Cond - Skips the next instruction if VX equals VY
                    (0x6,   _,   _,   _) => self.op_6xnn(x, nn), // 6XNN Const - Set VX to NN
                    (0x7,   _,   _,   _) => self.op_7xnn(x, nn), // 7XNN Const - Adds NN to VX
                    (0x8,   _,   _, 0x0) => self.op_8xy0(x, y), // 8XY0 Assign - Sets VX to the value of VY
                    (0x8,   _,   _, 0x1) => self.op_8xy1(x, y), // 8XY1 BitOp - Sets VX to VX | VY
                    (0x8,   _,   _, 0x2) => self.op_8xy2(x, y), // 8XY2 BitOp - Sets VX to VX & VY
                    (0x8,   _,   _, 0x3) => self.op_8xy3(x, y), // 8XY3 BitOp - Sets VX to VX ^ VY
                    (0x8,   _,   _, 0x4) => self.op_8xy4(x, y), // 8XY4 Math - Adds VY to VX, setting VF if there's an overflow
                    (0x8,   _,   _, 0x5) => self.op_8xy5(x, y), // 8XY5 Math - Subtracts VY from VX. Sets VF to 0 if underflow, 1 otherwise
                    (0x8,   _,   _, 0x6) => self.op_8xy6(x), // 8XY6 BitOp - Shifts VX to the right by 1, setting VF to the shifted bit
                    (0x8,   _,   _, 0x7) => self.op_8xy7(x, y), // 8XY7 Math - Sets VX to VY - VX. Sets VF to 0 if underflow, 1 otherwise
                    (0x8,   _,   _, 0xE) => self.op_8xye(x), // 8XYE BitOp - Shifts VX to the left by 1, setting VF to the shifted bit
                    (0x9,   _,   _,   _) => self.op_9xy0(x, y), // 9XY0 Cond - Skips the next instruction if VX does not equal VY
                    (0xA,   _,   _,   _) => self.op_annn(nnn), // ANNN MEM - Sets the I to the address NNN
                    (0xB,   _,   _,   _) => self.op_bnnn(x, nnn), // BNNN Flow - Jumps to the address NNN + V0
                    (0xC,   _,   _,   _) => self.op_cxnn(x, nn), // CXNN Rand - Sets VX to the result of a bitwise AND operation on a random u8 number and NN
                    (0xD,   _,   _,   _) => self.op_dxyn(x, y, n), // DXYN Display - Draws a sprite at coordinate (VX, VY)
                    (0xE,   _, 0x9, 0xE) => self.op_ex9e(x, instruction), // EX9E KeyOp - Skip if key pressed
                    (0xE,   _, 0xA, 0x1) => self.op_exa1(x, instruction), // EXA1 KeyOp - Skip if not pressed
                    (0xF,   _, 0x0, 0x7) => self.op_fx07(x), // FX07 Timer - Sets VX to the value of the delay timer
                    (0xF,   _, 0x0, 0xA) => self.op_fx0a(x), // FX0A KeyOp - A key press is awaited and then stored in VX (blocking operation)
                    (0xF,   _, 0x1, 0x5) => self.op_fx15(x), // FX15 Timer - Sets the delay timer to VX
                    (0xF,   _, 0x1, 0x8) => self.op_fx18(x), // FX18 Timer - Sets the sound timer to VX
                    (0xF,   _, 0x1, 0xE) => self.op_fx1e(x), // FX1E MEM - Adds VX to I.
                    (0xF,   _, 0x2, 0x9) => self.op_fx29(x), // FX29 MEM - Sets I to the location of the sprite for the character in VX
                    (0xF,   _, 0x3, 0x3) => self.op_fx33(x), // FX33 BCD - Stores the binary-coded decimal representation of VX in memory using the index register
                    (0xF,   _, 0x5, 0x5) => self.op_fx55(x), // FX55 MEM - Stores V0 to VX in memory, starting at address I
                    (0xF,   _, 0x6, 0x5) => self.op_fx65(x), // FX65 MEM - Loads V0 to VX from memory, starting at address I
                    _ => eprintln!("Unrecognized instruction: {instruction:#04X}"),
                }
    
                // redraw screen
                for bit in 0..(SCREEN_WIDTH * SCREEN_HEIGHT) {
                    let (x, y) = Self::flat_to_screen(bit);
                    
                    if self.screen.contains(bit) {
                        image.set_pixel(x as u32, y as u32, WHITE);
                    }
                    else {
                        image.set_pixel(x as u32, y as u32, BLACK);
                    }
                }   
            }

            texture.update(&image);
            draw_texture_ex(&texture, 0.0, 0.0, WHITE, DrawTextureParams {
                dest_size: Some(Vec2 { x: screen_width(), y: screen_height() }),
                source: None,
                rotation: 0.0,
                flip_x: false,
                flip_y: false,
                pivot: None,
            });
            
            next_frame().await;
        }
    }

    fn op_fx0a(&mut self, x: usize) {
        self.awaiting_keypress = true;
        self.awaiting_keypress_register = x;
    }
    
    fn op_fx65(&mut self, x: usize) {
        for register in 0..=x {
            self.registers[register] = self.memory[self.index_register + register];
        }
    }
    
    fn op_fx55(&mut self, x: usize) {
        for register in 0..=x {
            self.memory[self.index_register + register] = self.registers[register];
        }
    }
    
    fn op_fx33(&mut self, x: usize) {
        let hundreds = self.registers[x] / 100;
        let tens = self.registers[x] / 10 % 10;
        let ones = self.registers[x] % 10;
        self.memory[self.index_register] = hundreds;
        self.memory[self.index_register + 1] = tens;
        self.memory[self.index_register + 2] = ones;
    }
    
    fn op_fx29(&mut self, x: usize) {
        self.index_register = FONT_LOAD_INDEX + (x * 5)
    }
    
    fn op_fx1e(&mut self, x: usize) {
        self.index_register += self.registers[x] as usize
    }
    
    fn op_fx18(&mut self, x: usize) {
        self.sound_timer = self.registers[x]
    }
    
    fn op_fx15(&mut self, x: usize) {
        self.delay_timer = self.registers[x]
    }
    
    fn op_fx07(&mut self, x: usize) {
        self.registers[x] = self.delay_timer
    }
    
    fn op_exa1(&mut self, x: usize, instruction: u16) {
        let keycode = Self::key_value_to_keycode(&(self.registers[x] & 0xF))
            .expect(format!("Expected valid keycode in op: {instruction:#04X}").as_str());

        if let KeyState::Inactive | KeyState::JustReleased = self.key_states[&keycode] {
            self.program_counter += 2;
        }
    }
    
    fn op_ex9e(&mut self, x: usize, instruction: u16) {
        let keycode = Self::key_value_to_keycode(&(self.registers[x] & 0xF))
            .expect(format!("Expected valid keycode in op: {instruction:#04X}").as_str());

        if let KeyState::Active | KeyState::JustPressed = self.key_states[&keycode] {
            self.program_counter += 2;
        }
    }
    
    fn op_dxyn(&mut self, x: usize, y: usize, n: u8) {
        let x_coord = self.registers[x] % SCREEN_WIDTH as u8;
        let y_coord = self.registers[y] % SCREEN_HEIGHT as u8;
        let height = n;
        self.draw(x_coord, y_coord, height);
    }
    
    fn op_cxnn(&mut self, x: usize, nn: u8) {
        let num = random_range(0..=255) as u8;
        self.registers[x] = num & nn;
    }
    
    fn op_bnnn(&mut self, x: usize, nnn: usize) {
        // TODO: Make configurable
        // Original CHIP-8 behavior:
        // self.program_counter = nnn + self.registers[0] as usize;
        // CHIP-48/SUPER-CHIP
        self.program_counter = nnn + self.registers[x] as usize;
    }
    
    fn op_annn(&mut self, nnn: usize) {
        self.index_register = nnn
    }
    
    fn op_9xy0(&mut self, x: usize, y: usize) {
        if self.registers[x] != self.registers[y] { self.program_counter += 2; }
    }
    
    fn op_8xye(&mut self, x: usize) {
        // TODO: the following line is a quirk on some systems, make it configurable
        // self.registers[x] = self.registers[y];
        let vf_result = (self.registers[x] >> 7) & 1;
        self.registers[x] <<= 1;
        self.registers[0xF] = vf_result;
    }
    
    fn op_8xy7(&mut self, x: usize, y: usize) {
        let vf_result = if self.registers[y] >= self.registers[x] { 1 } else { 0 };
        self.registers[x] = self.registers[y].wrapping_sub(self.registers[x]);
        self.registers[0xF] = vf_result;
    }
    
    fn op_8xy6(&mut self, x: usize) {
        let vf_result = self.registers[x] & 1;
        self.registers[x] >>= 1;
        self.registers[0xF] = vf_result;
    }
    
    fn op_8xy5(&mut self, x: usize, y: usize) {
        let vf_result = if self.registers[x] >= self.registers[y] { 1 } else { 0 };
        self.registers[x] = self.registers[x].wrapping_sub(self.registers[y]);
        self.registers[0xF] = vf_result;
    }
    
    fn op_8xy4(&mut self, x: usize, y: usize) {
        let result = self.registers[x] as u16 + self.registers[y] as u16;
        self.registers[x] = self.registers[x].wrapping_add(self.registers[y]);
        self.registers[0xF] =  if result > 0xFF { 1 } else { 0 };
    }
    
    fn op_8xy3(&mut self, x: usize, y: usize) {
        self.registers[x] = self.registers[x] ^ self.registers[y];
        // self.registers[0xF] = 0; // Quirk for CHIP-8, make configurable
    }
    
    fn op_8xy2(&mut self, x: usize, y: usize) {
        self.registers[x] = self.registers[x] & self.registers[y];
        // self.registers[0xF] = 0; // Quirk for CHIP-8, make configurable
    }
    
    fn op_8xy1(&mut self, x: usize, y: usize) {
        self.registers[x] = self.registers[x] | self.registers[y];
        // self.registers[0xF] = 0; // Quirk for CHIP-8, make configurable
    }
    
    fn op_8xy0(&mut self, x: usize, y: usize) {
        self.registers[x] = self.registers[y]
    }
    
    fn op_7xnn(&mut self, x: usize, nn: u8) {
        self.registers[x] = self.registers[x].wrapping_add(nn)
    }
    
    fn op_6xnn(&mut self, x: usize, nn: u8) {
        self.registers[x] = nn
    }
    
    fn op_5xy0(&mut self, x: usize, y: usize) {
        if self.registers[x] == self.registers[y] { self.program_counter += 2; }
    }
    
    fn op_4xnn(&mut self, x: usize, nn: u8) {
        if self.registers[x] != nn { self.program_counter += 2; }
    }
    
    fn op_3xnn(&mut self, x: usize, nn: u8) {
        if self.registers[x] == nn { self.program_counter += 2; }
    }
    
    fn op_2nnn(&mut self, nnn: usize) {
        self.stack.push(self.program_counter as u16);
        self.program_counter = nnn;
    }
    
    fn op_1nnn(&mut self, nnn: usize) {
        self.program_counter = nnn as usize
    }
    
    fn op_00ee(&mut self) {
        self.program_counter = self.stack.pop().expect("stack should not be empty when returning from subroutine") as usize
    }
    
    fn op_00e0(&mut self) {
        self.screen.clear()
    }

    fn op_0nnn(&mut self) {
        panic!("Attempted to call machine code routine; not implemented.");
    }
    
    fn draw(&mut self, x: u8, y: u8, height: u8) {
        self.registers[0xF] = 0;

        // Loop through all the "rows" of the sprite
        for sprite_y in 0..height {
            if sprite_y + y >= SCREEN_HEIGHT as u8 {
                return;
            }

            // Compute the address of the data and fetch it
            let address = self.index_register + sprite_y as usize;
            let sprite_data = self.memory[address];

            // Go through all the bits in the byte of sprite data
            for sprite_x in 0..8 {
                let draw_x = x + sprite_x;
                let draw_y = y + sprite_y;
                let draw_v = (sprite_data >> (7 - sprite_x)) & 1;

                if draw_x >= SCREEN_WIDTH as u8 {
                    continue;
                }

                // Flip the bits based on the sprite data
                if draw_v == 1 {
                    let bit = Self::screen_to_flat(draw_x, draw_y);
                    if self.screen.contains(bit) {
                        self.screen.remove(bit);
                        self.registers[0xF] = 1; // on -> off sets VF
                    }
                    else {
                        self.screen.insert(bit);
                    }
                }
                else if draw_v != 0 {
                    panic!("Invalid draw value in draw.");
                }
            }
        }
    }

    fn update_key_states(&mut self) {
        for (keycode, state) in self.key_states.iter_mut() {
            if is_key_down(*keycode) {
                match state {
                    KeyState::Inactive
                    | KeyState::JustReleased => *state = KeyState::JustPressed,
                    KeyState::JustPressed => *state = KeyState::Active,
                    KeyState::Active => {}, // no-op
                }
            }
            else {
                match state {
                    KeyState::Active
                    | KeyState::JustPressed => *state = KeyState::JustReleased,
                    KeyState::JustReleased => *state = KeyState::Inactive,
                    KeyState::Inactive => {}, // no-op
                }
            }
        }
    }
    
    fn get_awaited_key(&self) -> Option<u8> {
        for (keycode, state) in self.key_states.iter() {
            if let KeyState::JustPressed = state {
                return Self::keycode_to_key_value(&keycode);
            }
        }

        None
    }

    fn screen_to_flat(x: u8, y: u8) -> usize {
        (y as usize * SCREEN_WIDTH) + x as usize
    }

    fn flat_to_screen(bit: usize) -> (u8, u8) {
        ((bit % SCREEN_WIDTH) as u8, (bit / SCREEN_WIDTH) as u8)
    }

    fn keycode_to_key_value(keycode: &KeyCode) -> Option<u8> {
        match keycode {
            KeyCode::Key1 => Some(0x1),
            KeyCode::Key2 => Some(0x2),
            KeyCode::Key3 => Some(0x3),
            KeyCode::Key4 => Some(0xC),
            KeyCode::Q    => Some(0x4),
            KeyCode::W    => Some(0x5),
            KeyCode::E    => Some(0x6),
            KeyCode::R    => Some(0xD),
            KeyCode::A    => Some(0x7),
            KeyCode::S    => Some(0x8),
            KeyCode::D    => Some(0x9),
            KeyCode::F    => Some(0xE),
            KeyCode::Z    => Some(0xA),
            KeyCode::X    => Some(0x0),
            KeyCode::C    => Some(0xB),
            KeyCode::V    => Some(0xF),
            _ => None,
        }
    }

    fn key_value_to_keycode(key_value: &u8) -> Option<KeyCode> {
        match key_value {
            0x1 => Some(KeyCode::Key1),
            0x2 => Some(KeyCode::Key2),
            0x3 => Some(KeyCode::Key3),
            0xC => Some(KeyCode::Key4),
            0x4 => Some(KeyCode::Q),
            0x5 => Some(KeyCode::W),
            0x6 => Some(KeyCode::E),
            0xD => Some(KeyCode::R),
            0x7 => Some(KeyCode::A),
            0x8 => Some(KeyCode::S),
            0x9 => Some(KeyCode::D),
            0xE => Some(KeyCode::F),
            0xA => Some(KeyCode::Z),
            0x0 => Some(KeyCode::X),
            0xB => Some(KeyCode::C),
            0xF => Some(KeyCode::V),
            _ => None
        }
    }
}

