use std::hash::{Hash, Hasher};
use crate::percent::PercentU16;

use super::can_apply::CanApply;

pub const COUNT: usize = 4;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from = "TrimmerSerializedForm")]
#[derive(Debug, Eq, Clone, Copy)]
pub struct Trimmer {
    values: [i16; COUNT],
    #[serde(skip)]    
    sum: i32,
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
    pub const ZERO: Trimmer = Trimmer::new(0, 0, 0, 0);

    pub const fn new(value0: i16, value1: i16, value2: i16, value3: i16) -> Self {
        Self {
            values: [value0, value1, value2, value3],
            sum: (value0 + value1 + value2 + value3) as i32,
        }
    }

    pub const fn from_array(values: [i16; 4]) -> Self {
        Self {
            values: values,
            sum: (values[0] + values[1] + values[2] + values[3]) as i32,
        }
    }

    pub fn from_vec(values: &Vec<i16>) -> Self {
        Self {
            values: [values[0], values[1], values[2], values[3]],
            sum: (values[0] + values[1] + values[2] + values[3]) as i32,
        }
    }

    pub fn to_array(&self) -> [i16; COUNT] {
        [self.values[0], self.values[1], self.values[2], self.values[3]]
    }

    pub fn value(&self, idx: usize) -> i32 {
        self.values[idx] as i32
    }

    pub fn values(&self) -> &[i16] {
        &self.values
    }

    pub fn to_vec(&self) -> Vec<i16> {
        vec![self.values[0], self.values[1], self.values[2], self.values[3]]
    }

    pub fn sum(&self) -> i32 {
        self.sum
    }

    pub fn updated<F>(self, mut f: F) -> Trimmer where F: FnMut(&mut [i16; 4]) {
        let mut values = self.values;
        f(&mut values);
        Self::from_array(values)
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

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from = "RateTrimmerSerializedForm")]
#[derive(Debug, Eq, Clone, Copy)]
pub struct RateTrimmer {
    values: [PercentU16; 4],
    #[serde(skip)]
    sum: PercentU16,
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
    pub const ONE: RateTrimmer = RateTrimmer {
        values: [PercentU16::HUNDRED, PercentU16::HUNDRED, PercentU16::HUNDRED, PercentU16::HUNDRED],
        sum: PercentU16::HUNDRED,
    };

    pub fn new(rate0: f32, rate1: f32, rate2: f32, rate3: f32) -> RateTrimmer {
        RateTrimmer {
            values: [PercentU16::from(rate0), PercentU16::from(rate1), PercentU16::from(rate2), PercentU16::from(rate3)],
            sum: PercentU16::from(rate0 * rate1 * rate2 * rate3),
        }
    }

    pub fn from_array(values: [PercentU16; 4]) -> Self {
        Self {
            values,
            sum: PercentU16::from(values[0].to_f32() * values[1].to_f32() * values[2].to_f32() * values[3].to_f32()),
        }
    }

    pub fn from_vec(values: &Vec<PercentU16>) -> Self {
        Self {
            values: [values[0], values[1], values[2], values[3]],
            sum: PercentU16::from(values[0].to_f32() * values[1].to_f32() * values[2].to_f32() * values[3].to_f32()),
        }
    }

    pub fn to_vec(self: &Self) -> Vec<PercentU16> {
        vec![self.values[0], self.values[1], self.values[2], self.values[3]]
    }

    pub fn value(self: &Self, idx: usize) -> PercentU16 {
        self.values[idx]
    }

    pub fn values(self: &Self) -> &[PercentU16] {
        &self.values
    }

    pub fn sum(self: &Self) -> PercentU16 {
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
}
