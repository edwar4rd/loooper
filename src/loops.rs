#[derive(Debug)]
pub struct LoopState {
    /// The length of the loop in beats.
    pub beat_count: u32,
    /// Whether the loop should start immediately after count-in.
    pub starting: bool,
    /// Whether the loop should be layered on top of prievious recording.
    pub layering: bool,
    pub wah: bool,
    pub reverb: bool,
    pub distortion: bool,
}

impl Default for LoopState {
    fn default() -> Self {
        LoopState {
            beat_count: 4,
            starting: false,
            layering: false,
            wah: false,
            reverb: false,
            distortion: false,
        }
    }
}
