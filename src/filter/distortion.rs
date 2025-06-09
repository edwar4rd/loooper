use crate::filter::Filter;

#[derive(Debug, Clone, Copy)]
pub struct Distortion {
    /// How much gain to apply before clipping.
    /// A value of 1.0 means no gain, 2.0 means double the input signal, etc.
    /// Less than 1.0 is not allowed.
    pub drive: f32,
    /// How much of the distorted signal to mix with the original signal.
    /// A value of 0.0 means no distortion, 1.0 means only the processed signal is output.
    /// Must be in the range [0.0, 1.0].
    pub mix: f32,
}

impl Distortion {
    /// Creates a new `Distortion` instance with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `drive` – The drive coefficient (must be greater than or equal to 1.0).`
    /// * `mix` – How much of the distorted signal to mix with the original signal (must be in the range [0.0, 1.0]).
    ///   * A value of 0.0 means no distortion, while a value of 1.0 means only the processed signal is output.
    ///
    /// # Panics
    ///
    /// This function will panic if `drive` or `mix` is out of the allowed range:
    ///
    /// # Returns
    ///
    /// A new `Distortion` instance with the specified parameters.
    ///
    pub fn new(drive: f32, mix: f32) -> Self {
        assert!(drive >= 1.0, "drive must be greater than 1.0");
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
