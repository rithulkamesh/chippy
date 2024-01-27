/*
  This project has been built with reference to the following resources:
  http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
  https://multigesture.net/articles/how-to-write-an-emulator-chip-8-interpreter/
  https://tobiasvl.github.io/blog/write-a-chip-8-emulator/

  Testing resources have been obtained from the following:
  https://github.com/corax89/chip8-test-rom
  https://github.com/Timendus/chip8-test-suite
  https://github.com/mattmikolay/chip-8/
*/

use sdl2::{audio::AudioSpecDesired, event::Event, keyboard::Keycode, pixels::Color};
use std::{fs::File, io::Read};

use super::audio::Square;

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

    // Audio handling through SDL
    audio_subsystem: sdl2::AudioSubsystem,
    audio_device: sdl2::audio::AudioDevice<Square>,
}

impl Chippy {
    pub fn new() -> Chippy {
        let sdl_context = sdl2::init().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();
        let desired_spec = AudioSpecDesired {
            freq: Some(44100), // Hz
            channels: Some(1),
            samples: None,
        };
        let audio_device = audio_subsystem
            .open_playback(None, &desired_spec, |spec| {
                // Initialize the square wave for audio
                Square {
                    phase_inc: 440.0 / spec.freq as f32,
                    phase: 0.0,
                }
            })
            .unwrap();
        Chippy {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0, // programs start at 0x200
            stack: [0; 16],
            sp: 0,
            display: [0; 64 * 32],
            keypad: [false; 16],
            delay_timer: 0,
            sound_timer: 0,
            audio_subsystem,
            audio_device,
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
                // Skip the instruction because we're not emulating any machine code.
                self.pc += 2;
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
                if self.sp > 0 {
                    self.sp -= 1;
                }
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
                let n = opcode & 0x0F;

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
                let x = self.v[((opcode & 0x0F00) >> 8) as usize] as usize % 64;
                let y = self.v[((opcode & 0x00F0) >> 4) as usize] as usize % 32;
                let n = opcode & 0x0F;

                self.v[0xF] = 0; // Reset VF

                for row in 0..n {
                    let sprite = self.memory[(self.i + row) as usize];
                    let mut pixel_row = sprite;

                    let mut pixel_x = x;
                    let pixel_y = (y + row as usize) % 32;

                    for _ in 0..8 {
                        let pixel_value = pixel_row >> 7;
                        let pixel_index = (pixel_y * 64 + pixel_x) as usize;

                        if pixel_value == 1 {
                            if self.display[pixel_index] != 0 {
                                self.display[pixel_index] = 0;
                                self.v[0xF] = 1; // Set VF if collision occurs
                            } else {
                                self.display[pixel_index] = 1;
                            }
                        }

                        pixel_row <<= 1;
                        pixel_x = (pixel_x + 1) % 64;
                    }
                }

                self.i += n;
            }

            // 0xEx**: Skip if key
            0xE000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                match opcode & 0x00FF {
                    // 0xEx9E: Skip next instruction if key with the value of Vx is pressed
                    0x009E => {
                        if self.keypad[self.v[x] as usize] {
                            self.pc += 2;
                        }
                    }
                    // 0xExA1: Skip next instruction if key with the value of Vx is not pressed
                    0x00A1 => {
                        if !self.keypad[self.v[x] as usize] {
                            self.pc += 2;
                        }
                    }
                    _ => {
                        println!("Unknown opcode: {:X}", opcode);
                    }
                }
            }

            // 0xFx**: Timers
            0xF000 => {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                match opcode & 0x00FF {
                    // 0xFx07: Set Vx = delay timer value
                    0x0007 => {
                        self.v[x] = self.delay_timer;
                    }
                    // 0xFx15: Set delay timer = Vx
                    0x0015 => {
                        self.delay_timer = self.v[x];
                    }
                    // 0xFx18: Set sound timer = Vx
                    0x0018 => {
                        self.sound_timer = self.v[x];
                    }
                    // 0xFX1E: Add to index
                    0x001E => {
                        self.i += self.v[x] as u16;
                    }
                    // 0xFx0A: Wait for a key press, store the value of the key in Vx
                    0x000A => {
                        let mut key_pressed = false;
                        for i in 0..16 {
                            if self.keypad[i] {
                                self.v[x] = i as u8;
                                key_pressed = true;
                            }
                        }
                        if !key_pressed {
                            self.pc -= 2;
                        }
                    }
                    // 0xFx29: Font Character, point to the font character in memory
                    0x0029 => {
                        let x: usize = ((opcode & 0x0F00) >> 8) as usize;
                        let character: u8 = self.v[x];
                        self.i = character as u16 * 5;
                    }
                    // 0xFx33: Store BCD representation of Vx in memory locations I, I+1, and I+2
                    0x0033 => {
                        self.memory[self.i as usize] = self.v[x] / 100;
                        self.memory[self.i as usize + 1] = (self.v[x] / 10) % 10;
                        self.memory[self.i as usize + 2] = (self.v[x] % 100) % 10;
                    }
                    // 0xFx55: Store registers V0 through Vx in memory starting at location I
                    0x0055 => {
                        for i in 0..=x {
                            self.memory[self.i as usize + i] = self.v[i];
                        }
                    }
                    // 0xFx65: Read registers V0 through Vx from memory starting at location I
                    0x0065 => {
                        for i in 0..=x {
                            self.v[i] = self.memory[self.i as usize + i];
                        }
                    }
                    _ => {
                        println!("Unknown opcode: {:X}", opcode);
                    }
                }
            }
            _ => {
                println!("Unimplmented or Unknown opcode: {:X}", opcode)
            }
        }

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                println!("BEEP!");
            }
            self.sound_timer -= 1;
        }

        self.pc += 2;
    }

    fn update_display(&mut self, canvas: &mut sdl2::render::Canvas<sdl2::video::Window>) {
        // Scale the display by 20x for better visibility
        canvas.set_scale(20.0, 20.0).unwrap();

        // Draw the display
        for (i, &pixel) in self.display.iter().enumerate() {
            let x = (i % 64) as i32;
            let y = (i / 64) as i32;
            if pixel == 1 {
                canvas.set_draw_color(Color::RGB(255, 255, 255));
            } else {
                canvas.set_draw_color(Color::RGB(0, 0, 0));
            }
            canvas.draw_point((x, y)).unwrap();
        }
    }

    fn init_font(&mut self) {
        let characters = [
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

        for (i, character) in characters.iter().enumerate() {
            self.memory[i] = *character;
        }
    }

    fn play_sound(&mut self) {
        // Decrement the sound timer
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }

        // Play sound if the sound timer is nonzero
        if self.sound_timer > 0 {
            self.audio_device.resume();
        } else {
            self.audio_device.pause();
        }
    }

    fn map_keycode_to_chip8_key(&self, keycode: Keycode) -> Option<usize> {
        match keycode {
            Keycode::Num1 => Some(0x1),
            Keycode::Num2 => Some(0x2),
            Keycode::Num3 => Some(0x3),
            Keycode::Num4 => Some(0xC),
            Keycode::Q => Some(0x4),
            Keycode::W => Some(0x5),
            Keycode::E => Some(0x6),
            Keycode::R => Some(0xD),
            Keycode::A => Some(0x7),
            Keycode::S => Some(0x8),
            Keycode::D => Some(0x9),
            Keycode::F => Some(0xE),
            Keycode::Z => Some(0xA),
            Keycode::X => Some(0x0),
            Keycode::C => Some(0xB),
            Keycode::V => Some(0xF),
            _ => None,
        }
    }

    //  Implemented from https://github.com/Rust-SDL2/rust-sdl2/blob/master/examples/window-properties.rs
    pub fn run(&mut self, game_path: &str) -> Result<(), String> {
        self.load_game(game_path)?;
        self.init_font();
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
                    Event::KeyDown {
                        keycode: Some(keycode),
                        repeat: false,
                        ..
                    } => {
                        if let Some(index) = self.map_keycode_to_chip8_key(keycode) {
                            self.keypad[index] = true;
                        }
                    }
                    // Handle key release events
                    Event::KeyUp {
                        keycode: Some(keycode),
                        ..
                    } => {
                        if let Some(index) = self.map_keycode_to_chip8_key(keycode) {
                            self.keypad[index] = false;
                        }
                    }
                    _ => {}
                }
            }
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.clear();
            self.emulate_cycle();
            self.play_sound();
            self.update_display(&mut canvas);
            canvas.present();
        }

        Ok(())
    }
}
