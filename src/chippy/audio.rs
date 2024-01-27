use sdl2::audio::AudioCallback;
pub struct Square {
    pub phase_inc: f32,
    pub phase: f32,
}

impl AudioCallback for Square {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = if self.phase < 0.5 { 0.5 } else { -0.5 };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}
