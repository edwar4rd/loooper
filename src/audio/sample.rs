use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SamplePad {
    pub buffer: Arc<[f32]>,
    pub pos: usize,
    pub playing: bool,
}

impl SamplePad {
    pub fn load_from_wav(path: &Path) -> color_eyre::Result<Self> {
        let mut reader = hound::WavReader::open(path)?;
        let buffer = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / i16::MAX as f32)
            .collect();
        Ok(SamplePad {
            buffer,
            pos: 0,
            playing: false,
        })
    }

    pub fn next_sample(&mut self) -> f32 {
        if !self.playing {
            return 0.0;
        }
        let s = self.buffer[self.pos];
        self.pos += 1;
        if self.pos >= self.buffer.len() {
            self.playing = false;
        }
        s
    }
}
