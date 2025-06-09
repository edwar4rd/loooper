#[derive(Debug, Clone, Copy)]
pub struct Oscillator {
    freq: f32,
    increment_time: f32,
    phase: f32,
    phase_increment: f32,
}

impl Oscillator {
    pub fn new(freq: f32, sample_rate: usize) -> Self {
        let increment_time = 1.0 / sample_rate as f32;
        Oscillator {
            freq,
            increment_time,
            phase: 0.0,
            phase_increment: std::f32::consts::TAU * freq * increment_time,
        }
    }

    #[inline]
    pub fn set_freq(&mut self, freq: f32) -> &mut Self {
        self.freq = freq;
        self.phase_increment = std::f32::consts::TAU * self.freq * self.increment_time;
        self
    }

    #[inline]
    pub fn increment(&mut self) -> f32 {
        self.phase += self.phase_increment;
        self.phase %= std::f32::consts::TAU;
        self.level()
    }

    #[inline]
    pub fn level(&self) -> f32 {
        self.phase.sin()
    }
}
