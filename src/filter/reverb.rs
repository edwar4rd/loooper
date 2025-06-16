use crate::filter::Delay;
use crate::filter::Filter;

pub struct Reverb {
    combs: Vec<Delay>,
    allpasses: Vec<Delay>,
    gain: f32,
}

impl Reverb {
    /// Create a new reverb processor.
    ///
    /// * `sample_rate` – sample rate in Hz
    /// * `comb_delays_ms` – comb delay times in ms
    /// * `comb_fb` – comb feedback (0.0–1.0)
    /// * `allpass_delays_ms` – all-pass delay times in ms
    /// * `allpass_fb` – all-pass feedback (0.0–1.0)
    /// * `gain` – overall output gain
    pub fn new(
        sample_rate: usize,
        comb_delays_ms: &[usize],
        comb_fb: f32,
        allpass_delays_ms: &[usize],
        allpass_fb: f32,
        gain: f32,
    ) -> Self {
        let to_samples = |ms: usize| (sample_rate * ms) / 1000;
        let combs = comb_delays_ms
            .iter()
            .map(|&ms| Delay::new(to_samples(ms), comb_fb, 1.0))
            .collect();
        let allpasses = allpass_delays_ms
            .iter()
            .map(|&ms| Delay::new(to_samples(ms), allpass_fb, 1.0))
            .collect();
        Reverb {
            combs,
            allpasses,
            gain,
        }
    }
}

impl Filter for Reverb {
    fn apply(&mut self, input: f32) -> f32 {
        let mut sum = 0.0;
        for comb in &mut self.combs {
            sum += comb.apply(input);
        }
        let norm = sum * (1.0 / (self.combs.len() as f32));

        let mut out = norm;
        for ap in &mut self.allpasses {
            out = ap.apply(out);
        }

        out * self.gain
    }
}
