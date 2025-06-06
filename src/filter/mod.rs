mod reverb;
pub use reverb::Reverb;

pub trait Filter {
    /// Apply the filter to a single sample.
    /// This function SHOULD NOT panic, nor should it allocate memory or perform any
    /// other potentially blocking operations.
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
