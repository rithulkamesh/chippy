mod chippy;
extern crate sdl2;

use chippy::chippy::Chippy;

fn main() -> Result<(), String> {
    let mut chippy_i: Chippy = Chippy::new();
    chippy_i.run("/home/rithulk/dev/chippy/test/ibm.ch8")
}
