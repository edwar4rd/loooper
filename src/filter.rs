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
pub fn reverb_sample(
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
