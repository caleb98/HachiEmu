use bit_set::BitSet;
use macroquad::{prelude::*, texture::Image};
use ::rand::random_range;

const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;
const MEMORY_BYTES: usize = 4096;
const INITIAL_STACK_SIZE: usize = 64;
const TARGET_OPS_PER_SECOND: u16 = 650;

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

pub struct Emulator {
    memory: [u8; MEMORY_BYTES],
    registers: [u8; 16],
    index_register: usize,
    program_counter: usize,
    stack: Vec<u16>,

    delay_timer: u8,
    sound_timer: u8,

    screen: BitSet,
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

        loop {
            update_time -= get_frame_time();
            clear_background(BLACK);

            while update_time <= 0.0 {
                update_time += target_cycle_time;

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
        
                match instruction {
                    // 00E0 Display - Clears the screen
                    0x00E0 => self.screen.clear(),
                    // 00EE Flow - Return from subroutine
                    0x00EE => self.program_counter = self.stack.pop().expect("stack should not be empty when returning from subroutine") as usize,
                    // 0NNN Call - Calls a machine code routine
                    0x0100..=0x01FF => {
                        panic!("Attempted to call machine code routine; not implemented.");
                    },
                    // 1NNN Flow - Goto NNN
                    0x1000..=0x1FFF => self.program_counter = nnn as usize,
                    // 2NNN Flow - Calls subroutine at NNN
                    0x2000..=0x2FFF => {
                        self.stack.push(self.program_counter as u16);
                        self.program_counter = nnn;
                    },
                    // 3XNN Cond - Skips the next instruction if VX equals NN
                    0x3000..=0x3FFF => if self.registers[x] == nn { self.program_counter += 2; },
                    // 4XNN Cond - Skips the next instruction if VX does not equal NN
                    0x4000..=0x4FFF => if self.registers[x] != nn { self.program_counter += 2; },
                    // 5XY0 Cond - Skips the next instruction if VX does not equal NN
                    0x5000..=0x5FFF => if self.registers[x] == self.registers[y] { self.program_counter += 2; },
                    // 6XNN Const - Set VX to NN
                    0x6000..=0x6FFF => self.registers[x] = nn,
                    // 7XNN Const - Adds NN to VX
                    0x7000..=0x7FFF => self.registers[x] = self.registers[x].wrapping_add(nn),
                    // 8... Logical/Arithmetic Operations
                    0x8000..=0x8FFF => match n {
                        // 8XY0 Assign - Sets VX to the value of VY
                        0x0 => self.registers[x] = self.registers[y],
                        // 8XY1 BitOp - Sets VX to VX | VY
                        0x1 => self.registers[x] = self.registers[x] | self.registers[y],
                        // 8XY2 BitOp - Sets VX to VX & VY
                        0x2 => self.registers[x] = self.registers[x] & self.registers[y],
                        // 8XY3 BitOp - Sets VX to VX ^ VY
                        0x3 => self.registers[x] = self.registers[x] ^ self.registers[y],
                        // 8XY4 Math - Adds VY to VX, setting VF if there's an overflow
                        0x4 => {
                            let result = self.registers[x] as u16 + self.registers[y] as u16;
                            self.registers[0xF] =  if result > 0xFF { 1 } else { 0 };
                            self.registers[x] = self.registers[x].wrapping_add(self.registers[y]);
                        },
                        // 8XY5 Math - Subtracts VY from VX. Sets VF to 0 if underflow, 1 otherwise
                        0x5 => {
                            self.registers[0xF] = if self.registers[x] >= self.registers[y] { 1 } else { 0 };
                            self.registers[x] = self.registers[x].wrapping_sub(self.registers[y]);
                        },
                        // 8XY6 BitOp - Shifts VX to the right by 1, setting VF to the shifted bit
                        0x6 => {
                            self.registers[0xF] = self.registers[x] & 1;
                            self.registers[x] >>= 1;
                        },
                        // 8XY7 Math - Sets VX to VY - VX. Sets VF to 0 if underflow, 1 otherwise
                        0x7 => {
                            self.registers[0xF] = if self.registers[y] >= self.registers[x] { 1 } else { 0 };
                            self.registers[x] = self.registers[y].wrapping_sub(self.registers[x]);
                        },
                        // 8XYE BitOp - Shifts VX to the left by 1, setting VF to the shifted bit
                        0xE => {
                            self.registers[0xF] = self.registers[x] & (1 << 7);
                            self.registers[x] = self.registers[x].wrapping_shl(1);
                        },
                        _ => eprintln!("Unrecognized instruction: {instruction:#04X}"),
                    },
                    // 9XY0 Cond - Skips the next instruction if VX does not equal VY
                    0x9000..=0x9FFF => if self.registers[x] != self.registers[y] { self.program_counter += 2; },
                    // ANNN MEM - Sets the I to the address NNN
                    0xA000..=0xAFFF => self.index_register = nnn,
                    // BNNN Flow - Jumps to the address NNN + V0
                    0xB000..=0xBFFF => self.program_counter = self.memory[nnn] as usize + self.registers[0] as usize,
                    // CXNN Rand - Sets VX to the result of a bitwise AND operation on a random u8 number and NN
                    0xC000..=0xCFFF => {
                        let num = random_range(0..=255) as u8;
                        self.registers[x] = num & nn;
                    },
                    // DXYN Display - Draws a sprite at coordinate (VX, VY)
                    0xD000..=0xDFFF => {
                        let x_coord = self.registers[x] % SCREEN_WIDTH as u8;
                        let y_coord = self.registers[y] % SCREEN_HEIGHT as u8;
                        let height = n;
                        self.draw(x_coord, y_coord, height);
                    },
                    // E... Keys and Input
                    0xE000..=0xEFFF => todo!(),
                    // F... Memory and Devices
                    0xF000..=0xFFFF => match nn {
                        // FX07 Timer - Sets VX to the value of the delay timer
                        0x07 => self.registers[x] = self.delay_timer,
                        // FX0A KeyOp - A key press is awaited and then stored in VX (blocking operation)
                        0x0A => todo!(),
                        // FX15 Timer - Sets the delay timer to VX
                        0x15 => self.delay_timer = self.registers[x],
                        // FX18 Sound - Sets the sound timer to VX
                        0x18 => self.sound_timer = self.registers[x],
                        // FX1E MEM - Adds VX to I.
                        0x1E => self.index_register += self.registers[x] as usize,
                        // FX29 MEM - Sets I to the location of the sprite for the character in VX
                        0x29 => self.index_register = FONT_LOAD_INDEX + x,
                        // FX33 BCD - Stores the binary-coded decimal representation of VX in memory using the index register
                        0x33 => {
                            let hundreds = self.registers[x] / 100;
                            let tens = self.registers[x] / 10 % 10;
                            let ones = self.registers[x] % 10;
                            self.memory[self.index_register] = hundreds;
                            self.memory[self.index_register + 1] = tens;
                            self.memory[self.index_register + 2] = ones;
                        },
                        // FX55 MEM - Stores V0 to VX in memory, starting at address I
                        0x55 => {
                            for register in 0..=x {
                                self.memory[self.index_register + register] = self.registers[register];
                            }
                        },
                        // FX64 MEM - Loads V0 to VX from memory, starting at address I
                        0x65 => {
                            for register in 0..=x {
                                self.registers[register] = self.memory[self.index_register + register];
                            }
                        },
                        _ => eprintln!("Unrecognized instruction: {instruction:#04X}"),
                    },
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
                // dest_size: None,
                source: None,
                rotation: 0.0,
                flip_x: false,
                flip_y: false,
                pivot: None,
            });
            
            next_frame().await;
        }
    }

    fn draw(&mut self, x: u8, y: u8, height: u8) {
        self.registers[0xF] = 0;

        // Loop through all the "rows" of the sprite
        for sprite_y in 0..height {
            // Compute the address of the data and fetch it
            let address = self.index_register + sprite_y as usize;
            let sprite_data = self.memory[address];

            // Go through all the bits in the byte of sprite data
            for sprite_x in 0..8 {
                let draw_x = x + sprite_x;
                let draw_y = y + sprite_y;
                let draw_v = (sprite_data >> (7 - sprite_x)) & 1;

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

    fn screen_to_flat(x: u8, y: u8) -> usize {
        (y as usize * SCREEN_WIDTH) + x as usize
    }

    fn flat_to_screen(bit: usize) -> (u8, u8) {
        ((bit % SCREEN_WIDTH) as u8, (bit / SCREEN_WIDTH) as u8)
    }
}