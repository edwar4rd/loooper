// src/filter/distortion.rs
use crate::filter::Filter;

#[derive(Debug, Clone, Copy)]
pub struct Distortion {
    pub drive: f32,
    pub mix: f32,
}

impl Distortion {
    pub fn new(drive: f32, mix: f32) -> Self {
        assert!(drive >= 1.0, "drive >= 1.0");
        assert!((0.0..=1.0).contains(&mix), "mix must be 0.0 to 1.0");
        Self { drive, mix }
    }

    fn process(&self, x: f32) -> f32 {
        // 1) pre-gain
        let d = x * self.drive;
        // 2) hard clip + tanh
        let clipped = if d > 1.0 {
            1.0
        } else if d < -1.0 {
            -1.0
        } else {
            d.tanh()
        };
        // 3) mix
        x * (1.0 - self.mix) + clipped * self.mix
    }
}

impl Filter for Distortion {
    fn apply(&mut self, sample: f32) -> f32 {
        self.process(sample)
    }
}
