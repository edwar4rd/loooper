pub trait Filter {
    /// Apply the filter to a single sample.
    ///
    /// # Arguments
    ///
    /// * `sample` – The input sample to filter.
    ///
    /// # Returns
    ///
    /// The filtered sample.
    fn apply(&mut self, sample: f32) -> f32;
}

#[derive(Debug, Clone)]
pub struct Reverb {
    delay_line: Box<[f32]>,
    idx: usize,
    pub feedback: f32,
    pub wet: f32,
}

impl Reverb {
    /// Creates a new `Reverb` instance with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `sample_count` – The number of samples in the delay line.
    /// * `idx` – The initial index for the delay line (usually 0).
    /// * `feedback` – The feedback coefficient (0.0 to 1.0) controlling the decay of the reverb.
    /// * `wet` – The wet/dry mix ratio (0.0 to 1.0) controlling the balance between the dry signal and the reverb effect.
    ///
    /// # Panics
    ///
    /// This function will panic if `wet` or `feedback` are outside the range [0.0, 1.0].
    ///
    /// # Returns
    ///
    /// A new `Reverb` instance with the specified parameters and an empty delay line.
    ///
    pub fn new(sample_count: usize, idx: usize, feedback: f32, wet: f32) -> Self {
        assert!((0.0..=1f32).contains(&wet));
        assert!((0.0..=1f32).contains(&feedback));

        let delay_line = vec![0.0; sample_count].into_boxed_slice();

        Self {
            delay_line,
            idx,
            feedback,
            wet,
        }
    }
}

impl Filter for Reverb {
    fn apply(&mut self, dry: f32) -> f32 {
        reverb_sample(
            dry,
            &mut self.delay_line,
            &mut self.idx,
            self.feedback,
            self.wet,
        )
    }
}

/// Simple per-sample delay-line reverb function.
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
fn reverb_sample(
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
