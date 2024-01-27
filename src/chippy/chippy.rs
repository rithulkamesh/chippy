/*
  This project has been built with reference to the following resources:
  http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
  https://multigesture.net/articles/how-to-write-an-emulator-chip-8-interpreter/

  Testing resources have been obtained from the following:
  https://github.com/corax89/chip8-test-rom
  https://github.com/Timendus/chip8-test-suite
*/

use sdl2::{event::Event, keyboard::Keycode, pixels::Color};
use std::{fs::File, io::Read};

pub struct Chippy {
    // 4K RAM in a CHIP-8 system
    pub memory: [u8; 4096],

    // 16 general-purpose 8-bit registers, V0-VE, VF is a carry flag
    pub v: [u8; 16],

    // Index Register
    pub i: u16,

    // Program Counter
    pub pc: u16,

    // monochrome display of 64x32 pixels, which can be only on or off at one time.
    pub display: [u8; 64 * 32],

    // A stack to store return addresses
    pub stack: [u16; 16],
    pub sp: usize, // Stack pointer

    // Buzzer will play when sound timer is 0, Both delay and sound timers count at 60Hz
    pub delay_timer: u8,
    pub sound_timer: u8,

    // hexadecimal keypad, 0-9, A-F
    pub keypad: [bool; 16],
}

impl Chippy {
    pub fn new() -> Chippy {
        Chippy {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0x200, // programs start at 0x200
            stack: [0; 16],
            sp: 0,
            display: [0; 64 * 32],
            keypad: [false; 16],
            delay_timer: 0,
            sound_timer: 0,
        }
    }

    // We need to load the game from a file into memory, so we can execute its opcode
    fn load_game(&mut self, game_path: &str) -> Result<(), String> {
        let file = File::open(game_path).map_err(|e| e.to_string())?;

        // Programs start at 0x200, so we need to load the game into memory starting at that address
        // `i` is a temporary pointer to the current address that we're loading the opcode into.
        let mut i = 0x200;
        for byte in file.bytes() {
            self.memory[i] = byte.map_err(|e| e.to_string())?;
            i += 1;
        }

        Ok(())
    }

    // Some Common placeholders:
    // nnn or addr - A 12-bit value, the lowest 12 bits of the instruction
    // n or nibble - A 4-bit value, the lowest 4 bits of the instruction
    // x - A 4-bit value, the lower 4 bits of the high byte of the instruction
    // y - A 4-bit value, the upper 4 bits of the low byte of the instruction
    fn emulate_cycle(&mut self) {
        let opcode = (self.memory[self.pc as usize] as u16) << 8
            | self.memory[(self.pc + 1) as usize] as u16;

        match opcode & 0xF000 {
            // 0xAnnn: Set I to nnn
            0xA000 => self.i = opcode & 0x0FFF,
            // 0x00E0: Clear the display,
            0x00E0 => {
                for pixel in &mut self.display {
                    *pixel = 0;
                }
            }
            // 0x0nnn: Call machine language routine
            0x0000 => {
                // println!("0x0000 unimplemented due to not emulating any old machine code");
            }
            // 0x1nnn: Jump to address nnn
            0x1000 => self.pc = opcode & 0x0FFF,
            // 0x2nnn: Call subroutine at nnn
            0x2000 => {
                self.stack[self.sp] = self.pc;
                self.sp += 1;
                self.pc = opcode & 0x0FFF;
            }
            // 0x00EE: Return from subroutine
            0x00EE => {
                self.sp -= 1;
                self.pc = self.stack[self.sp];
            }
            // 0x3xnn: Skip next instruction if Vx = nn
            0x3000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let nn = (opcode & 0x00FF) as u8;
                if self.v[x] == nn {
                    self.pc += 2;
                }
            }
            // 0x4xnn: Skip next instruction if Vx != nn
            0x4000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let nn = (opcode & 0x00FF) as u8;
                if self.v[x] != nn {
                    self.pc += 2;
                }
            }
            // 0x5xy0: Skip next instruction if Vx = Vy
            0x5000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 4) as usize;
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }
            // 0x9xy0: Skip next instruction if Vx != Vy
            0x9000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 4) as usize;
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }
            // 0x6xnn: Set Vx = nn
            0x6000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let nn = (opcode & 0x00FF) as u8;
                self.v[x] = nn;
            }
            // 0x7xnn: Set Vx = Vx + nn
            0x7000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let nn = (opcode & 0x00FF) as u8;
                self.v[x] = self.v[x].wrapping_add(nn);
            }
            // 0x8xy*: Arithmetic operations
            0x8000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 4) as usize;
                let n = opcode & 0x000F;

                match n {
                    // 0x8xy0: Vx = Vy
                    0x0000 => self.v[x] = self.v[y],
                    // 0x8xy1: Vx = Vx | Vy
                    0x0001 => self.v[x] |= self.v[y],
                    // 0x8xy2: Vx = Vx & Vy
                    0x0002 => self.v[x] &= self.v[y],
                    // 0x8xy3: Vx = Vx ^ Vy
                    0x0003 => self.v[x] ^= self.v[y],
                    // 0x8xy4: Vx = Vx + Vy, set VF = carry
                    0x0004 => {
                        let (result, overflow) = self.v[x].overflowing_add(self.v[y]);
                        self.v[x] = result;
                        self.v[0xF] = overflow as u8;
                    }
                    // 0x8xy5: Set Vx = Vx - Vy, set VF = NOT borrow
                    0x0005 => {
                        let (result, borrow) = self.v[x].overflowing_sub(self.v[y]);
                        self.v[x] = result;
                        self.v[0xF] = (!borrow) as u8;
                    }
                    // 0x8xy7: Set Vx = Vy - Vx, set VF = NOT borrow
                    0x0007 => {
                        let (result, borrow) = self.v[y].overflowing_sub(self.v[x]);
                        self.v[x] = result;
                        self.v[0xF] = (!borrow) as u8;
                    }
                    // 0x8xy6: Right shift Vx by 1, set VF = least significant bit of Vx before shift
                    0x0006 => {
                        self.v[0xF] = self.v[x] & 0x1;
                        self.v[x] >>= 1;
                    }
                    // 0x8xyE: Left shift Vx by 1, set VF = most significant bit of Vx before shift
                    0x000E => {
                        self.v[0xF] = (self.v[x] >> 7) & 0x1;
                        self.v[x] <<= 1;
                    }

                    _ => {
                        println!("Unknown opcode: {:X}", opcode);
                    }
                }
            }
            // 0xBnnn: Jump to address nnn + V0
            0xB000 => {
                let nnn = opcode & 0x0FFF;
                self.pc = nnn + self.v[0] as u16;
            }
            // 0xCxnn: Set Vx = random byte & nn
            0xC000 => {
                let x: usize = ((opcode & 0x0F00) >> 8) as usize;
                let nn: u8 = (opcode & 0x00FF) as u8;
                self.v[x] = rand::random::<u8>() & nn;
            }
            // 0xDxyn: DISPLAY
            0xD000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 4) as usize;
                let n = (opcode & 0x000F) as usize;

                if n > 0 {
                    // Calculate sprite address in memory
                    for row in 0..n {
                        let sprite_byte = self.memory[(self.i as usize) + row];

                        for col in 0..8 {
                            let pixel_value = (sprite_byte >> (7 - col)) & 0x01;
                            let pixel_x = (self.v[x] as usize + col) % 64;
                            let pixel_y = (self.v[y] as usize + row) % 32;
                            let pixel_index = pixel_y * 64 + pixel_x;

                            if pixel_y < 32 && pixel_x < 64 {
                                // XOR the pixel value to the display
                                self.display[pixel_index] ^= pixel_value;
                            }
                        }
                    }
                }
            }
            _ => {
                println!("Unimplmented or Unknown opcode: {:X}", opcode)
            }
        }
        self.pc += 2;
    }

    fn update_display(&mut self, canvas: &mut sdl2::render::Canvas<sdl2::video::Window>) {
        // Scale the display by 20x for better visibility
        canvas.set_scale(20.0, 20.0).unwrap();

        // Draw the display
        for (i, pixel) in self.display.iter().enumerate() {
            let x = (i % 64) as i32;
            let y = (i / 64) as i32;
            if *pixel == 1 {
                canvas.set_draw_color(Color::RGB(255, 255, 255));
            } else {
                canvas.set_draw_color(Color::RGB(0, 0, 0));
            }
            canvas.draw_point((x, y)).unwrap();
        }
    }

    //  Implemented from https://github.com/Rust-SDL2/rust-sdl2/blob/master/examples/window-properties.rs
    pub fn run(&mut self, game_path: &str) -> Result<(), String> {
        self.load_game(game_path)?;
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window("Chippy", 1280, 640)
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;

        let mut canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        let mut event_pump = sdl_context.event_pump().map_err(|e| e.to_string())?;

        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        // Exit the application when escape is pressed
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    _ => {}
                }
            }
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.clear();
            self.emulate_cycle();
            self.update_display(&mut canvas);
            canvas.present();
        }

        Ok(())
    }
}
