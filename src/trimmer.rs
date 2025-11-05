//! Timing and rate adjustment utilities for musical notes.
//!
//! This module provides two main types:
//! - [`Trimmer`]: Adjusts note timing by adding/subtracting ticks
//! - [`RateTrimmer`]: Adjusts note duration/velocity by applying percentage rates

use std::hash::{Hash, Hasher};
use crate::percent::PercentU16;

use super::can_apply::CanApply;

/// Number of trimmer values stored (4 levels of adjustment).
pub const COUNT: usize = 4;

/// Timing adjustment for notes, storing 4 levels of tick offsets.
///
/// Each trimmer can store up to 4 independent timing adjustments that are summed together.
/// This allows for hierarchical timing adjustments (e.g., global, section, measure, note level).
///
/// # Examples
///
/// ```
/// use klavier_core::trimmer::Trimmer;
///
/// // Create a trimmer with offsets at different levels
/// let trimmer = Trimmer::new(10, -5, 0, 2);
/// assert_eq!(trimmer.sum(), 7); // 10 - 5 + 0 + 2 = 7
///
/// // Add more ticks
/// let adjusted = trimmer.added(100);
/// assert_eq!(adjusted.sum(), 107);
/// ```
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from = "TrimmerSerializedForm")]
#[derive(Debug, Eq, Clone, Copy)]
pub struct Trimmer {
    /// Four levels of timing adjustments in ticks.
    values: [i16; COUNT],
    /// Cached sum of all values for performance.
    #[serde(skip)]
    sum: i32,
}

impl Default for Trimmer {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<TrimmerSerializedForm> for Trimmer {
    fn from(from: TrimmerSerializedForm) -> Self {
        Self::from_array(from.values)
    }
}

#[derive(serde::Deserialize)]
struct TrimmerSerializedForm {
    values: [i16; COUNT],
}

impl Trimmer {
    /// A trimmer with all values set to zero (no adjustment).
    pub const ZERO: Trimmer = Trimmer::new(0, 0, 0, 0);

    /// Creates a new trimmer with four timing adjustment values.
    ///
    /// # Arguments
    ///
    /// * `value0` - First level timing adjustment in ticks
    /// * `value1` - Second level timing adjustment in ticks
    /// * `value2` - Third level timing adjustment in ticks
    /// * `value3` - Fourth level timing adjustment in ticks
    pub const fn new(value0: i16, value1: i16, value2: i16, value3: i16) -> Self {
        Self {
            values: [value0, value1, value2, value3],
            sum: value0 as i32 + value1 as i32 + value2 as i32 + value3 as i32,
        }
    }

    /// Creates a trimmer from an array of 4 values.
    ///
    /// # Arguments
    ///
    /// * `values` - Array of 4 timing adjustments in ticks
    pub const fn from_array(values: [i16; 4]) -> Self {
        Self {
            values,
            sum: (values[0] + values[1] + values[2] + values[3]) as i32,
        }
    }

    /// Creates a trimmer from a slice of values.
    ///
    /// # Arguments
    ///
    /// * `values` - Slice containing at least 4 timing adjustments
    ///
    /// # Panics
    ///
    /// Panics if the slice has fewer than 4 elements.
    pub fn from_vec(values: &[i16]) -> Self {
        Self {
            values: [values[0], values[1], values[2], values[3]],
            sum: (values[0] + values[1] + values[2] + values[3]) as i32,
        }
    }

    /// Converts the trimmer to an array of 4 values.
    pub fn to_array(&self) -> [i16; COUNT] {
        [self.values[0], self.values[1], self.values[2], self.values[3]]
    }

    /// Gets the timing adjustment value at the specified index.
    ///
    /// # Arguments
    ///
    /// * `idx` - Index (0-3) of the value to retrieve
    ///
    /// # Returns
    ///
    /// The timing adjustment value as i32.
    pub fn value(&self, idx: usize) -> i32 {
        self.values[idx] as i32
    }

    /// Gets a reference to all timing adjustment values.
    pub fn values(&self) -> &[i16] {
        &self.values
    }

    /// Converts the trimmer to a vector of values.
    pub fn to_vec(&self) -> Vec<i16> {
        vec![self.values[0], self.values[1], self.values[2], self.values[3]]
    }

    /// Returns the sum of all timing adjustments.
    ///
    /// This is the total tick offset that will be applied to a note.
    pub fn sum(&self) -> i32 {
        self.sum
    }

    /// Creates a new trimmer by applying a function to modify the values.
    ///
    /// # Arguments
    ///
    /// * `f` - Function that modifies the array of values
    ///
    /// # Returns
    ///
    /// A new trimmer with updated values and recalculated sum.
    pub fn updated<F>(self, f: F) -> Trimmer where F: FnOnce(&mut [i16; 4]) {
        let mut values = self.values;
        f(&mut values);
        Self::from_array(values)
    }

    /// Adds ticks to the trimmer, distributing overflow across multiple levels.
    ///
    /// When adding ticks would exceed `i16::MAX` or `i16::MIN` at one level,
    /// the overflow is carried to the next level. This allows for large timing
    /// adjustments while maintaining the 4-level structure.
    ///
    /// # Arguments
    ///
    /// * `tick` - Number of ticks to add (can be negative)
    ///
    /// # Returns
    ///
    /// A new trimmer with the ticks added.
    ///
    /// # Examples
    ///
    /// ```
    /// use klavier_core::trimmer::Trimmer;
    ///
    /// let trimmer = Trimmer::new(0, 1, 2, 3);
    /// let adjusted = trimmer.added(100);
    /// assert_eq!(adjusted.value(0), 100);
    /// assert_eq!(adjusted.sum(), 106); // 100 + 1 + 2 + 3
    /// ```
    pub fn added(mut self, mut tick: i32) -> Self {
        for i in 0..4 {
            let t: i32 = self.values[i] as i32 + tick;
            if (i16::MAX as i32) < t {
                tick = t - i16::MAX as i32;
                self.values[i] = i16::MAX;
            } else if t < (i16::MIN as i32) {
                tick = t - i16::MIN as i32;
                self.values[i] = i16::MIN;
            } else {
                self.values[i] = t as i16;
                break;
            }
        }

        self.sum = self.values[0] as i32 +self.values[1] as i32 +
            self.values[2] as i32 + self.values[3] as i32;
        self
    }
}

impl PartialEq for Trimmer {
    fn eq(&self, other: &Self) -> bool {
        if self.sum == other.sum {
            self.values == other.values
        } else {
            false
        }
    }
}

impl Hash for Trimmer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sum.hash(state)
    }
}

/// Rate adjustment for notes, storing 4 levels of percentage multipliers.
///
/// Each rate trimmer can store up to 4 independent percentage adjustments that are multiplied together.
/// This allows for hierarchical rate adjustments (e.g., global tempo, section dynamics, measure expression, note articulation).
/// The final rate is clamped to a maximum of 200% (2.0).
///
/// # Examples
///
/// ```
/// use klavier_core::trimmer::RateTrimmer;
/// use klavier_core::can_apply::CanApply;
///
/// // Create a rate trimmer with different multipliers
/// let rate = RateTrimmer::new(0.9, 1.1, 1.0, 1.0);
/// assert_eq!(rate.apply(100), 99); // 100 * 0.9 * 1.1 = 99
///
/// // Maximum rate is 200%
/// let rate = RateTrimmer::new(2.0, 2.0, 1.0, 1.0);
/// assert_eq!(rate.sum().to_f32(), 2.0); // Clamped to 200%
/// ```
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from = "RateTrimmerSerializedForm")]
#[derive(Debug, Eq, Clone, Copy)]
pub struct RateTrimmer {
    /// Four levels of rate adjustments as percentages.
    values: [PercentU16; 4],
    /// Cached product of all rates (clamped to 200%).
    #[serde(skip)]
    sum: PercentU16,
}

impl Default for RateTrimmer {
    fn default() -> Self {
        Self::ONE
    }
}

impl From<RateTrimmerSerializedForm> for RateTrimmer {
    fn from(from: RateTrimmerSerializedForm) -> Self {
        Self::from_array(from.values)
    }
}

#[derive(serde::Deserialize)]
struct RateTrimmerSerializedForm {
    values: [PercentU16; 4],
}

impl CanApply<u32> for RateTrimmer {
    fn apply(self, value: u32) -> u32 {
        self.sum.apply(value)
    }
}

impl RateTrimmer {
    /// A rate trimmer with all values set to 100% (no adjustment).
    pub const ONE: RateTrimmer = RateTrimmer {
        values: [PercentU16::HUNDRED, PercentU16::HUNDRED, PercentU16::HUNDRED, PercentU16::HUNDRED],
        sum: PercentU16::HUNDRED,
    };

    /// Creates a new rate trimmer with four percentage multipliers.
    ///
    /// The final rate is the product of all four values, clamped to a maximum of 200% (2.0).
    ///
    /// # Arguments
    ///
    /// * `rate0` - First level rate multiplier (e.g., 1.0 = 100%, 0.5 = 50%, 1.5 = 150%)
    /// * `rate1` - Second level rate multiplier
    /// * `rate2` - Third level rate multiplier
    /// * `rate3` - Fourth level rate multiplier
    pub fn new(rate0: f32, rate1: f32, rate2: f32, rate3: f32) -> RateTrimmer {
        RateTrimmer {
            values: [PercentU16::from(rate0), PercentU16::from(rate1), PercentU16::from(rate2), PercentU16::from(rate3)],
            sum: PercentU16::from(rate0 * rate1 * rate2 * rate3),
        }
    }

    /// Creates a rate trimmer from an array of 4 percentage values.
    ///
    /// # Arguments
    ///
    /// * `values` - Array of 4 percentage values
    pub fn from_array(values: [PercentU16; 4]) -> Self {
        Self {
            values,
            sum: PercentU16::from(values[0].to_f32() * values[1].to_f32() * values[2].to_f32() * values[3].to_f32()),
        }
    }

    /// Creates a rate trimmer from a slice of percentage values.
    ///
    /// # Arguments
    ///
    /// * `values` - Slice containing at least 4 percentage values
    ///
    /// # Panics
    ///
    /// Panics if the slice has fewer than 4 elements.
    pub fn from_vec(values: &[PercentU16]) -> Self {
        Self {
            values: [values[0], values[1], values[2], values[3]],
            sum: PercentU16::from(values[0].to_f32() * values[1].to_f32() * values[2].to_f32() * values[3].to_f32()),
        }
    }

    /// Converts the rate trimmer to a vector of percentage values.
    pub fn to_vec(&self) -> Vec<PercentU16> {
        vec![self.values[0], self.values[1], self.values[2], self.values[3]]
    }

    /// Gets the rate value at the specified index.
    ///
    /// # Arguments
    ///
    /// * `idx` - Index (0-3) of the value to retrieve
    pub fn value(&self, idx: usize) -> PercentU16 {
        self.values[idx]
    }

    /// Gets a reference to all rate values.
    pub fn values(&self) -> &[PercentU16] {
        &self.values
    }

    /// Returns the product of all rate multipliers (clamped to 200%).
    ///
    /// This is the final rate that will be applied to a value.
    pub fn sum(&self) -> PercentU16 {
        self.sum
    }
}

impl PartialEq for RateTrimmer {
    fn eq(&self, other: &Self) -> bool {
        if self.sum == other.sum {
            self.values == other.values
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::percent::PercentU16;
    use crate::trimmer::Trimmer;

    #[test]
    fn sum() {
        let v = Trimmer::new(0, 1, 2, 3);
        assert_eq!(v.value(0), 0);
        assert_eq!(v.value(1), 1);
        assert_eq!(v.value(2), 2);
        assert_eq!(v.value(3), 3);

        assert_eq!(v.values[0], 0);
        assert_eq!(v.values[1], 1);
        assert_eq!(v.values[2], 2);
        assert_eq!(v.values[3], 3);

        assert_eq!(v.sum(), 6);
    }

    #[test]
    fn eq() {
        let v0 = Trimmer::new(0, 1, 2, 3);
        let v1 = Trimmer::new(0, 1, 2, 3);
        let v2 = Trimmer::new(1, 0, 2, 3);

        assert!(v0 == v1);
        assert!(v0 != v2);
    }

    #[test]
    fn updated() {
        let v0 = Trimmer::new(0, 1, 2, 3);
        let v1 = v0.updated(|values| {
            values[1] = 10;
        });
        assert_eq!(v1.value(0), 0);
        assert_eq!(v1.value(1), 10);
        assert_eq!(v1.value(2), 2);
        assert_eq!(v1.value(3), 3);

        assert_eq!(v0.value(0), 0);
        assert_eq!(v0.value(1), 1);
        assert_eq!(v0.value(2), 2);
        assert_eq!(v0.value(3), 3);
    }

    use crate::trimmer::RateTrimmer;
    use crate::can_apply::CanApply;

    #[test]
    fn rate_sum() {
        let v = RateTrimmer::new(0.9, 1.5, 1.1, 1.0);
        assert_eq!(v.value(0).to_f32(), 0.9);
        assert_eq!(v.value(1).to_f32(), 1.5);
        assert_eq!(v.value(2).to_f32(), 1.1);
        assert_eq!(v.value(3).to_f32(), 1.0);

        assert_eq!(v.values()[0].to_f32(), 0.9);
        assert_eq!(v.values()[1].to_f32(), 1.5);
        assert_eq!(v.values()[2].to_f32(), 1.1);
        assert_eq!(v.values()[3].to_f32(), 1.0);

        assert!((v.sum().to_f32() - 0.9 * 1.5 * 1.1 * 1.0).abs() < 0.01);
    }

    #[test]
    fn rate_eq() {
        let v0 = RateTrimmer::new(0.0, 1.0, 2.0, 3.0);
        let v1 = RateTrimmer::new(0.0, 1.0, 2.0, 3.0);
        let v2 = RateTrimmer::new(1.0, 0.0, 2.0, 3.0);

        assert!(v0 == v1);
        assert!(v0 != v2);
    }

    #[test]
    fn rate() {
        let v = RateTrimmer::new(0.5, 1.0, 2.0, 3.0);
        assert_eq!(v.sum().to_f32(), 2.0); // Max is 200%
        assert_eq!(v.apply(100), 200);

        let v = RateTrimmer::new(0.5, 1.0, 1.5, 2.0);
        assert_eq!(v.sum().to_f32(), 1.5);
        assert_eq!(v.apply(100), 150);

        let v = RateTrimmer::from_array([PercentU16::HUNDRED, PercentU16::HUNDRED, PercentU16::HUNDRED, PercentU16::HUNDRED]);
        assert_eq!(v.sum(), PercentU16::HUNDRED);
    }

    #[test]
    fn added() {
        let v0 = Trimmer::new(0, 1, 2, 3);
        assert_eq!(
            v0.added(100),
            Trimmer::new(100, 1, 2, 3)
        );
        assert_eq!(
            v0.added(i16::MAX as i32 + 1),
            Trimmer::new(i16::MAX, 2, 2, 3)
        );
        assert_eq!(
            v0.added(i16::MAX as i32 * 2),
            Trimmer::new(i16::MAX, i16::MAX, 3, 3)
        );
        assert_eq!(
            v0.added(i16::MAX as i32 * 3),
            Trimmer::new(i16::MAX, i16::MAX, i16::MAX, 6)
        );
        assert_eq!(
            v0.added(i16::MAX as i32 * 4 - 6),
            Trimmer::new(i16::MAX, i16::MAX, i16::MAX, i16::MAX)
        );
        assert_eq!(
            v0.added(i16::MAX as i32 * 4 - 5),
            Trimmer::new(i16::MAX, i16::MAX, i16::MAX, i16::MAX)
        );
    }
}
