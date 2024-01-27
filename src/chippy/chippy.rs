/*
  This project has been built with reference to the following resources:
  http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
  https://multigesture.net/articles/how-to-write-an-emulator-chip-8-interpreter/

  Testing resources have been obtained from the following:
  https://github.com/corax89/chip8-test-rom
  https://github.com/Timendus/chip8-test-suite
*/

use std::{fs::File, io::Read};

pub struct Chippy {
    // 4K RAM in a CHIP-8 system
    pub memory: [u8; 4096],

    // 16 general-purpose 8-bit registers
    pub v: [u8; 16],

    // Index Register
    pub i: u16,

    // Program Counter
    pub pc: u16,

    // A stack to store return addresses
    pub stack: [u16; 16],
    pub sp: usize, // Stack pointer

    // monochrome display of 64x32 pixels, which can be only on or off at one time.
    pub display: [u8; 64 * 32],

    // hexadecimal keypad, 0-9, A-F
    pub keypad: [bool; 16],

    pub delay_timer: u8,
    pub sound_timer: u8,
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

    fn execute_opcode(&mut self, opcode: u16) {}

    fn emulate_cycle(&mut self) {
        let opcode = (self.memory[self.pc as usize] as u16) << 8
            | self.memory[(self.pc + 1) as usize] as u16;

        // Decode and execute the opcode (you'll need to implement this part)
        self.execute_opcode(opcode);

        // Update the program counter
        self.pc += 2;
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.load_game("/home/rithulk/dev/chippy/test/ibm.ch8")?;
        crate::chippy::init_window()
    }
}
