use crate::can_apply::CanApply;

// 0.0% - 6553.5%
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, Eq)]
pub struct PercentU16 {
    value: u16,
}

// 0 - 200%
const MAX_VALUE: i32 = 2000;
const MIN_VALUE: i32 = 0;
const HUNDRED_VALUE: u16 = 1000;

impl PercentU16 {
    pub const MAX: PercentU16 = PercentU16 { value: MAX_VALUE as u16 };
    pub const MIN: PercentU16 = PercentU16 { value: MIN_VALUE as u16 };
    pub const ZERO: PercentU16 = PercentU16::MIN;
    pub const HUNDRED: PercentU16 = Self::from_value(HUNDRED_VALUE);

    pub const fn from_value(value: u16) -> PercentU16 {
        PercentU16 {
            value,
        }
    }

    pub fn to_f32(self) -> f32 {
        (self.value as f32) / 1000.0
    }
}

impl CanApply<u32> for PercentU16 {
    fn apply(self, value: u32) -> u32 {
        let i: i64 = value as i64;
        ((i * self.value as i64) / HUNDRED_VALUE as i64) as u32
    }
}

impl From<f32> for PercentU16 {
    fn from(rate: f32) -> Self {
        let i: i32 = (rate * 1000.0) as i32; // 1.0 means 100% = 100.0% = 1000
        if i < MIN_VALUE {
            PercentU16 { value: MIN_VALUE as u16 }
        } else if MAX_VALUE < i {
            PercentU16 { value: MAX_VALUE as u16 }
        } else {
            PercentU16 { value: i as u16 }
        }
    }
}

impl PartialEq for PercentU16 {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

#[cfg(test)]
mod tests {
    use crate::percent::PercentU16;
    use crate::percent::CanApply;

    #[test]
    fn too_high() {
        assert_eq!(PercentU16::MAX, PercentU16::from(201.0));
    }

    #[test]
    fn too_low() {
        assert_eq!(PercentU16::from(-0.1), PercentU16::from(0.0));
    }

    #[test]
    fn to_f32() {
        assert_eq!(PercentU16::from(1.5).to_f32(), 1.5);
    }

    #[test]
    fn apply() {
        assert_eq!(PercentU16::from(0.5).apply(60), 30);
    }
}
