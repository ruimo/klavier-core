use std::ops::{Add, AddAssign, SubAssign};
use std::fmt;

#[derive(Debug)]
pub enum OctaveError {
    InvalidValue(i32),
}

impl fmt::Display for OctaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OctaveError::InvalidValue(value) => f.write_fmt(
                format_args!("Octave({}) should {} <= {} <= {}", value, Octave::MIN_VALUE.value(), value, Octave::MAX_VALUE.value())
            ),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Octave {
    OctM2, OctM1, Oct0, Oct1, Oct2, Oct3, Oct4, Oct5, Oct6, Oct7, Oct8,
}

pub const MAX: Octave = Octave::Oct8;
pub const MIN: Octave = Octave::OctM2;

impl Octave {
    pub const ALL: &'static [Octave] = &[
        Self::OctM2, Self::OctM1, Self::Oct0, Self::Oct1, Self::Oct2,
        Self::Oct3, Self::Oct4, Self::Oct5, Self::Oct6, Self::Oct7, Self::Oct8,
    ];
    pub const MIN_VALUE: Octave = Octave::OctM2;
    pub const MAX_VALUE: Octave = Octave::Oct8;
    pub const BIAS_VALUE: i32 = 2;

    pub const fn value_of(value: i32) -> Result<Octave, OctaveError> {
        Self::from_score_offset(value + Self::BIAS_VALUE)
    }

    pub const fn from_score_offset(idx: i32) -> Result<Octave, OctaveError> {
        if idx < 0 || Self::ALL.len() < (idx as usize) {
            Err(OctaveError::InvalidValue(idx))
        } else {
            Ok(Self::ALL[idx as usize])
        }
    }

    pub const fn offset(self) -> i32 {
        self.value() + Self::BIAS_VALUE
    }

    pub const fn value(self) -> i32 {
        match self {
            Self::OctM2 => -2,
            Self::OctM1 => -1,
            Self::Oct0 => 0,
            Self::Oct1 => 1,
            Self::Oct2 => 2,
            Self::Oct3 => 3,
            Self::Oct4 => 4,
            Self::Oct5 => 5,
            Self::Oct6 => 6,
            Self::Oct7 => 7,
            Self::Oct8 => 8,
        }
    }
}

impl AddAssign<i32> for Octave {
    fn add_assign(&mut self, rhs: i32) {
        *self = Octave::value_of(self.value() + rhs).unwrap();
    }
}

impl SubAssign<i32> for Octave {
    fn sub_assign(&mut self, rhs: i32) {
        *self = Octave::value_of(self.value() - rhs).unwrap();
    }
}

impl Add for Octave {
    type Output = Self;
    fn add(self, other: Self) -> Octave {
        Octave::value_of(self.value() + other.value()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::octave::Octave;

    #[test]
    #[should_panic]
    fn value_too_small() {
        Octave::value_of(-3).unwrap();
    }

    #[test]
    #[should_panic]
    fn value_too_big() {
        Octave::value_of(10).unwrap();
    }

    #[test]
    fn can_read_value() {
        assert_eq!(Octave::value_of(1).unwrap().value(), 1);
    }

    #[test]
    fn can_read_score_offset() {
        assert_eq!(Octave::value_of(1).unwrap().offset(), 3);
    }

    #[test]
    fn can_read_offset() {
        assert_eq!(Octave::value_of(-1).unwrap().offset(), 1);
    }

    #[test]
    fn can_add() {
        assert_eq!(Octave::value_of(1).unwrap() + Octave::value_of(2).unwrap(), Octave::value_of(3).unwrap());
    }

    #[test]
    fn add_assign() {
        let mut oct = Octave::Oct0;
        oct += 1;
        assert_eq!(Octave::Oct1, oct);

        oct += 2;
        assert_eq!(Octave::Oct3, oct);
    }

    #[test]
    #[should_panic]
    fn add_assign_error() {
        let mut oct = Octave::Oct8;
        oct += 1;
    }

    #[test]
    fn sub_assign() {
        let mut oct = Octave::Oct1;
        oct -= 1;
        assert_eq!(Octave::Oct0, oct);

        oct -= 2;
        assert_eq!(Octave::OctM2, oct);
    }

    #[test]
    #[should_panic]
    fn sub_assign_error() {
        let mut oct = Octave::Oct0;
        oct -= 3;
    }
}
