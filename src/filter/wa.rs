// src/filter/wah.rs
use crate::filter::Filter;
use std::f32::consts::PI;

/// A simple Wah-Wah effect implemented as a state-variable band-pass
/// filter with an LFO sweeping its center frequency between min and max.
#[derive(Debug, Clone)]
pub struct Wah {
    /// sample rate in Hz
    sr: f32,
    /// LFO phase [0.0..1.0)
    lfo_phase: f32,
    /// LFO frequency in Hz (how fast the sweep)
    lfo_hz: f32,
    /// lowest center freq (Hz)
    min_f: f32,
    /// highest center freq (Hz)
    max_f: f32,
    /// resonance / filter “Q”
    q: f32,
    /// filter state
    low: f32,
    band: f32,
}

impl Wah {
    /// Create a new Wah.
    ///
    /// - `sample_rate` e.g. 48_000.0  
    /// - `lfo_hz` the sweep speed, e.g. 1.0 – 5.0 Hz  
    /// - `min_f` lower corner, e.g. 500.0 Hz  
    /// - `max_f` upper corner, e.g. 3000.0 Hz  
    /// - `q` resonance, e.g. 0.5…1.5
    pub fn new(sample_rate: f32, lfo_hz: f32, min_f: f32, max_f: f32, q: f32) -> Self {
        assert!(min_f > 0.0 && max_f > min_f);
        Wah {
            sr: sample_rate,
            lfo_phase: 0.0,
            lfo_hz,
            min_f,
            max_f,
            q,
            low: 0.0,
            band: 0.0,
        }
    }
}

impl Filter for Wah {
    fn apply(&mut self, x: f32) -> f32 {
        // 1) advance LFO
        let lfo = (2.0 * PI * self.lfo_phase).sin() * 0.5 + 0.5;
        self.lfo_phase += self.lfo_hz / self.sr;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        // 2) compute current center frequency
        let fc = self.min_f + lfo * (self.max_f - self.min_f);
        // normalized filter coefficient
        let f = 2.0 * (PI * fc / self.sr).sin();

        // 3) state-variable filter steps
        //   high = x - low - q*band
        let high = x - self.low - self.q * self.band;
        //   band += f * high
        self.band += f * high;
        //   low  += f * band
        self.low += f * self.band;

        // output the band-pass component
        self.band
    }
}
