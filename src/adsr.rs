#[derive(Debug, Clone, Copy, PartialEq)]
enum ADSRPhase {
    A,
    D,
    S,
    R,
    End,
}

#[derive(Debug, Clone, Copy)]
pub struct ADSR {
    attack: f32,
    decay: f32,
    sustain_level: f32,
    release: f32,
    pos: f32,
    phase: ADSRPhase,
    level: f32,
}

impl Default for ADSR {
    fn default() -> Self {
        Self::new(0.01, 0.1, 0.8, 0.1)
    }
}

impl ADSR {
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        Self {
            attack,
            decay,
            sustain_level: sustain,
            release,
            phase: ADSRPhase::A,
            pos: 0.0,
            level: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.phase = ADSRPhase::A;
        self.pos = 0.0;
        self.level = 0.0;
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn release(&mut self) {
        if self.phase != ADSRPhase::S {
            return;
        }

        if self.release == 0.0 {
            self.phase = ADSRPhase::End;
            self.level = 0.0;
            return;
        }

        // assert_eq!(self.phase, ADSRPhase::S);
        self.phase = ADSRPhase::R;
        self.level = self.sustain_level;
        self.pos = 0.0;
    }

    pub fn forward(&mut self, time: f32) -> f32 {
        if time == 0.0 {
            return self.level();
        }

        match self.phase {
            ADSRPhase::A => {
                self.pos += time;
                if self.pos >= self.attack {
                    self.phase = ADSRPhase::D;
                    let time = self.pos - self.attack;
                    self.level = 1.0;
                    self.pos = 0.0;
                    self.forward(time)
                } else {
                    self.level = self.pos / self.attack;
                    self.level
                }
            }
            ADSRPhase::D => {
                self.pos += time;
                if self.pos >= self.decay {
                    self.phase = ADSRPhase::S;
                    self.pos = 0.0;
                    self.level = self.sustain_level;
                    return self.level;
                }
                let decay_rate = (1.0 - self.sustain_level) / self.decay;
                self.level = 1.0 - (self.pos * decay_rate);
                self.level
            }
            ADSRPhase::S => {
                self.level = self.sustain_level;
                self.level
            }
            ADSRPhase::R => {
                self.pos += time;
                if self.pos >= self.release {
                    self.phase = ADSRPhase::End;
                    self.level = 0.0;
                    return 0.0;
                }
                let release_rate = self.sustain_level / self.release;
                self.level = self.sustain_level - (self.pos * release_rate);
                self.level
            }
            ADSRPhase::End => 0.0,
        }
    }
}

#[test]
fn test_adsr() {
    let mut adsr = ADSR::new(0.01, 0.1, 0.8, 0.1);
    let mut levels = vec![];

    assert_eq!(adsr.level(), 0.0);
    for _ in 0..100000 {
        levels.push(adsr.forward(1. / 48000.));
    }
    assert!(levels.iter().all(|&x| (0.0..=1.0).contains(&x)));
    assert_eq!(adsr.level(), 0.8);
    adsr.release();
    levels.clear();
    for _ in 0..100000 {
        levels.push(adsr.forward(1. / 48000.));
    }
    assert!(levels.iter().all(|&x| (0.0..=0.8).contains(&x)));
    assert_eq!(adsr.level(), 0.0);

    adsr.reset();
    assert_eq!(adsr.level(), 0.0);
    levels.clear();
    let mut time = 0.0;
    for _ in 0..100 {
        levels.push(adsr.forward(0.0023));
        time += 0.0023;
        assert_eq!(adsr.level(), *levels.last().unwrap());
        println!("{},{}", time, adsr.level());
    }
    adsr.release();
    for _ in 0..100 {
        levels.push(adsr.forward(0.0023));
        time += 0.0023;
        assert_eq!(adsr.level(), *levels.last().unwrap());
        println!("{},{}", time, adsr.level());
    }
    assert_eq!(levels.len(), 200);
    // println!("{:?}", levels);
}
