use crate::filter::Filter;

#[derive(Debug, Clone)]
pub struct Delay {
    delay_line: Box<[f32]>,
    delay_line_start: usize,
    /// The current end of the delay line. This can be
    delay_line_end: usize,
    /// If we need to resize the delay line, we can set this to Some(new_size).
    delay_line_desired_length: Option<usize>,
    idx: usize,
    pub feedback: f32,
    pub wet: f32,
}

impl Delay {
    /// Creates a new `Delay` instance with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `sample_count` – The number of samples in the delay line.
    /// * `feedback` – The feedback coefficient (0.0 to 1.0) controlling the decay of the delay.
    ///   * `feedback = 1.0` is not recommended as it can lead to infinite feedback.
    /// * `wet` – The wet/dry mix ratio (0.0 to 1.0) controlling the balance between the dry signal and the delay effect.\
    ///   * `wet = 0.0` means no delay, and `wet = 1.0` means full delay effect.
    ///
    /// # Panics
    ///
    /// This function will panic if `wet` or `feedback` are outside the range [0.0, 1.0].
    ///
    /// # Returns
    ///
    /// A new `Delay` instance with the specified parameters and an empty delay line.
    ///
    pub fn new(sample_count: usize, feedback: f32, wet: f32) -> Self {
        assert!((0.0..=1f32).contains(&wet));
        assert!((0.0..=1f32).contains(&feedback));

        let delay_line = vec![0.0; sample_count].into_boxed_slice();

        Self {
            delay_line,
            delay_line_start: 0,
            delay_line_end: sample_count,
            delay_line_desired_length: None,
            idx: 0,
            feedback,
            wet,
        }
    }

    /// Clears the delay line and thus resets the delay effect.
    ///
    /// Calling this function can create a noticeable click in the audio stream.
    pub fn reset_delay(&mut self) {
        // Reset the delay line to zero
        self.delay_line.fill(0.0);
    }

    /// Try to "resize" the delay line.
    /// This function doesn't allocate new memory, and get as close as possible to the requested size.
    /// This function tries to not create any clicks in the audio stream or cause any audible artifacts.
    ///
    /// If the new size is larger than the current size, it will fill the new space with zeros.
    /// If the new size is smaller, samples will be dropped from the end of the delay line.
    pub fn resize_delay(&mut self, new_size: usize) {
        if new_size == self.delay_line.len() {
            return; // No change needed
        }

        let target_size = new_size.min(self.delay_line.len());
        self.delay_line_desired_length = Some(target_size);
    }
}

impl Filter for Delay {
    fn apply(&mut self, dry: f32) -> f32 {
        delay_sample(
            dry,
            &mut self.delay_line,
            &mut self.idx,
            self.feedback,
            self.wet,
        )
    }
}

/// Simple per-sample delay-line delay function.
///
/// # Arguments
///
/// * `dry`           – 當前乾聲樣本 (input sample)。
/// * `delay_line`    – 延遲線緩衝，長度為 delay_samples；此陣列會在呼叫時直接更新回音狀態。\
///   以長度為零的 `delay_line` 呼叫此函數時將不會有任何回音效果。
/// * `idx`           – 延遲線目前的讀寫位置 (circular buffer index)。傳入 &mut usize，內部會自動 +1 (wrapping)。
/// * `feedback`      – 反饋係數 (0.0–1.0)，控制回音衰減強度。
/// * `wet`           – 濕聲比例 (0.0–1.0)，決定混合乾聲與回音的比重。
///
/// # Panics
///
///  This function will panic if given invalid `wet` or `feedback` values.
///
/// # Returns
///
/// 回傳單個 sample 經過混響後的最終值：`dry * (1–wet) + new_wet * wet`。
fn delay_sample(
    dry: f32,
    delay_line: &mut [f32],
    idx: &mut usize,
    feedback: f32,
    wet: f32,
) -> f32 {
    debug_assert!((0.0..=1f32).contains(&wet));
    debug_assert!((0.0..=1f32).contains(&feedback));

    // 如果 delay 長為零就直接 bypass，只回傳乾聲
    if delay_line.is_empty() {
        return dry;
    }

    let delay_samples = delay_line.len();
    let d_idx = *idx % delay_samples;
    let delayed_out = delay_line[d_idx];

    let new_wet = dry + delayed_out * feedback;
    delay_line[d_idx] = new_wet;

    *idx = idx.wrapping_add(1);
    if *idx >= delay_samples {
        *idx -= delay_samples;
    }

    dry * (1.0 - wet) + new_wet * wet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay() {
        const SAMPLE_COUNT: usize = 4800;
        const FEEDBACK: f32 = 0.1;
        const WET: f32 = 0.8;
        let mut delay = Delay::new(SAMPLE_COUNT, FEEDBACK, WET);
        assert_eq!(delay.delay_line.len(), SAMPLE_COUNT);
        assert_eq!(delay.feedback, FEEDBACK);
        assert_eq!(delay.wet, WET);

        let in_sample = 0.665;
        for _ in 0..SAMPLE_COUNT {
            let out_sample = delay.apply(in_sample);
            assert!(out_sample == in_sample);
        }

        let old_in_sample = in_sample;
        let in_sample = 0.137;
        for _ in 0..SAMPLE_COUNT {
            let out_sample = delay.apply(in_sample);
            assert_eq!(
                out_sample,
                in_sample * (1.0 - WET) + (in_sample + old_in_sample * FEEDBACK) * WET
            );
        }

        let old_old_sample = old_in_sample;
        let old_in_sample = in_sample;
        let in_sample = 0.999;
        for _ in 0..SAMPLE_COUNT {
            let out_sample = delay.apply(in_sample);
            let old_written = old_in_sample + old_old_sample * FEEDBACK;
            assert_eq!(
                out_sample,
                in_sample * (1.0 - WET) + (in_sample + old_written * FEEDBACK) * WET
            );
        }
    }
}
