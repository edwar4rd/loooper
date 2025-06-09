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
            delay_line_end: sample_count.saturating_sub(1),
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

        // Reset the indices
        let current_length = self.delay_line_length();
        self.idx = 0;
        self.delay_line_start = 0;
        if let Some(desired_length) = self.delay_line_desired_length {
            self.delay_line_end = desired_length.saturating_sub(1);
            self.delay_line_desired_length = None;
        } else {
            self.delay_line_end = current_length.saturating_sub(1);
        }
        assert_eq!(self.delay_line_length(), current_length);
    }

    pub fn delay_line_length(&self) -> usize {
        if self.delay_line_end > self.delay_line_start {
            self.delay_line_end - self.delay_line_start + 1
        } else {
            self.delay_line.len() - self.delay_line_start + self.delay_line_end + 1
        }
    }

    /// Try to "resize" the delay line.
    /// This function doesn't allocate new memory, and get as close as possible to the requested size.
    /// This function tries to not create any clicks in the audio stream or cause any audible artifacts.
    ///
    /// If the new size is larger than the current size, the current sample will be kept,
    /// and the current sample will be repeated until the delay line reaches the new size.
    /// If the new size is smaller, samples will be dropped from the end of the delay line.
    ///
    /// The function will keep the delay line at the same note if
    pub fn resize(&mut self, new_size: usize) {
        let current_size = self.delay_line_length();
        if new_size == current_size {
            return; // No change needed
        }

        if new_size < current_size {
            self.delay_line_start += current_size - new_size;
            if self.delay_line_start >= self.delay_line.len() {
                self.delay_line_start -= self.delay_line.len();
            }
            assert_eq!(self.delay_line_length(), new_size);
        } else {
            let target_size = new_size.min(self.delay_line.len());
            if current_size == target_size {
                return; // No change possible
            }

            // TODO: make the delay line grow by repeating the last sample
            // self.delay_line_desired_length = Some(target_size);
            self.delay_line_end += target_size - current_size;
            if self.delay_line_end >= self.delay_line.len() {
                self.delay_line_end -= self.delay_line.len();
            }
            assert_eq!(self.delay_line_length(), target_size);
        }
    }
}

impl Delay {
    fn increment_index(&mut self) {
        if self.idx == self.delay_line_end {
            self.idx = self.delay_line_start; // Wrap around if we reach the end of the delay line
        } else {
            self.idx += 1;
        }
    }
}

impl Filter for Delay {
    fn apply(&mut self, dry: f32) -> f32 {
        match self.delay_line_desired_length {
            Some(_) => {
                // If we are resizing the delay line, we need to write the sample at the current index
                // and then increment the index, but use the old sample for the output.
                let mut dev_null = 0.0;
                let delayed_out = self.delay_line[self.idx];
                
                delay_sample(dry, delayed_out, &mut dev_null, self.feedback, self.wet)
            }
            None => {
                let delayed_out = &mut self.delay_line[self.idx];
                let out = delay_sample(dry, *delayed_out, delayed_out, self.feedback, self.wet);
                self.increment_index();
                out
            }
        }
    }
}

/// Simple per-sample delay-line delay function.
///
/// # Arguments
///
/// * `dry`           – 當前乾聲樣本 (input sample)。
/// * `delayed_out`   - 當前在 delay line 中的樣本。
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
fn delay_sample(dry: f32, delayed_out: f32, new_written: &mut f32, feedback: f32, wet: f32) -> f32 {
    debug_assert!((0.0..=1f32).contains(&wet));
    debug_assert!((0.0..=1f32).contains(&feedback));

    let new_wet = dry + delayed_out * feedback;
    *new_written = new_wet;

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

        test_delay_with_const(&mut delay);
    }

    #[test]
    #[should_panic]
    fn test_delay_twice() {
        const SAMPLE_COUNT: usize = 4800;
        const FEEDBACK: f32 = 0.1;
        const WET: f32 = 0.8;
        let mut delay = Delay::new(SAMPLE_COUNT, FEEDBACK, WET);
        test_delay_with_const(&mut delay);
        test_delay_with_const(&mut delay);
    }

    #[test]
    fn test_delay_twice_with_reset() {
        const SAMPLE_COUNT: usize = 4800;
        const FEEDBACK: f32 = 0.1;
        const WET: f32 = 0.8;
        let mut delay = Delay::new(SAMPLE_COUNT, FEEDBACK, WET);
        test_delay_with_const(&mut delay);
        delay.reset_delay();
        test_delay_with_const(&mut delay);
    }

    #[test]
    fn test_delay_length() {
        const SAMPLE_COUNT: usize = 4800;
        const FEEDBACK: f32 = 0.1;
        const WET: f32 = 0.8;
        let mut delay = Delay::new(SAMPLE_COUNT, FEEDBACK, WET);
        assert_eq!(delay.delay_line_length(), SAMPLE_COUNT);

        // Resize the delay line to a smaller size
        delay.resize(2400);

        // Run the delay for a while to give it time to finish resizing
        for _ in 0..10 * SAMPLE_COUNT {
            delay.apply(0.0);
        }

        // Now the delay line should be resized
        assert_eq!(delay.delay_line_length(), 2400);
        delay.reset_delay();
        assert_eq!(delay.delay_line_length(), 2400);
        test_delay_with_const(&mut delay);

        delay.resize(3600);

        // Run the delay for a while to give it time to finish resizing
        for _ in 0..10 * SAMPLE_COUNT {
            delay.apply(0.0);
        }

        // Now the delay line should be resized
        assert_eq!(delay.delay_line_length(), 3600);
        delay.reset_delay();
        assert_eq!(delay.delay_line_length(), 3600);
        test_delay_with_const(&mut delay);

        delay.resize(6000);

        // Run the delay for a while to give it time to finish resizing
        for _ in 0..10 * SAMPLE_COUNT {
            delay.apply(0.0);
        }

        // Now the delay line should be resized
        assert_eq!(delay.delay_line_length(), 4800);
        delay.reset_delay();
        assert_eq!(delay.delay_line_length(), 4800);
        test_delay_with_const(&mut delay);
    }

    fn test_delay_with_const(delay: &mut Delay) {
        let wet = delay.wet;
        let feedback = delay.feedback;
        let sample_count = delay.delay_line_length();

        {
            let in_sample = 0.5;
            let out_sample = delay.apply(in_sample);
            assert!(out_sample == in_sample);
        }
        let in_sample = 0.665;
        for _ in 1..sample_count {
            let out_sample = delay.apply(in_sample);
            assert!(out_sample == in_sample);
        }

        {
            let old_in_sample = 0.5;
            let in_sample = 0.242;
            let out_sample = delay.apply(in_sample);
            assert_eq!(
                out_sample,
                in_sample * (1.0 - wet) + (in_sample + old_in_sample * feedback) * wet
            );
        }
        let old_in_sample = in_sample;
        let in_sample = 0.137;
        for _ in 1..sample_count {
            let out_sample = delay.apply(in_sample);
            assert_eq!(
                out_sample,
                in_sample * (1.0 - wet) + (in_sample + old_in_sample * feedback) * wet
            );
        }

        {
            let old_old_sample = 0.5;
            let old_in_sample = 0.242;
            let in_sample = 0.326;
            let out_sample = delay.apply(in_sample);
            let old_written = old_in_sample + old_old_sample * feedback;
            assert_eq!(
                out_sample,
                in_sample * (1.0 - wet) + (in_sample + old_written * feedback) * wet
            );
        }
        let old_old_sample = old_in_sample;
        let old_in_sample = in_sample;
        let in_sample = 0.999;
        for _ in 1..sample_count {
            let out_sample = delay.apply(in_sample);
            let old_written = old_in_sample + old_old_sample * feedback;
            assert_eq!(
                out_sample,
                in_sample * (1.0 - wet) + (in_sample + old_written * feedback) * wet
            );
        }
    }
}
