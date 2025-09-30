use std::{fmt::{self, Display}, str::FromStr};

use super::duration::Duration;

pub const MIN_NUMERATOR: u8 = 1;
pub const MAX_NUMERATOR: u8 = 99;

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Numerator(u8);

impl Display for Numerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub enum NumeratorError {
    InvalidValue(u8),
    CannotParse(String),
}

impl Numerator {
    pub fn from_value(value: u8) -> Result<Numerator, NumeratorError> {
        if !(MIN_NUMERATOR..=MAX_NUMERATOR).contains(&value) {
            Err(NumeratorError::InvalidValue(value))
        } else {
            Ok(Numerator(value))
        }
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl FromStr for Numerator {
    type Err = NumeratorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match u8::from_str(s) {
            Ok(value) => Self::from_value(value),
            Err(_) => Err(NumeratorError::CannotParse(s.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(serde::Deserialize, serde::Serialize)]
pub enum Denominator {
    D2, D4, D8, D16, D32, D64,
}

pub const DENOMINATORS: [Denominator; 6] = [
    Denominator::D2, Denominator::D4, Denominator::D8,
    Denominator::D16, Denominator::D32, Denominator::D64,
];

pub enum DenominatorError {
    InvalidValue(u8),
}

impl Denominator {
    pub fn from_value(value: u8) -> Result<Denominator, DenominatorError> {
        match value {
            2 => Ok(Denominator::D2),
            4 => Ok(Denominator::D4),
            8 => Ok(Denominator::D8),
            16 => Ok(Denominator::D16),
            32 => Ok(Denominator::D32),
            64 => Ok(Denominator::D64),
            _ => Err(DenominatorError::InvalidValue(value)),
        }
    }

    pub fn value(self) -> u8 {
        match self {
            Denominator::D2 => 2,
            Denominator::D4 => 4,
            Denominator::D8 => 8,
            Denominator::D16 => 16,
            Denominator::D32 => 32,
            Denominator::D64 => 64,
        }
    }
}

impl fmt::Display for Denominator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value())
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RhythmError {
    NumeratorError(u8),
    DenominatorError(u8)
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Rhythm {
    pub numerator: Numerator,
    pub denominator: Denominator,
}

impl Rhythm {
    pub fn new(numerator: u8, denominator: u8) -> Rhythm {
        match Self::value_of(numerator, denominator) {
            Err(_pe) => panic!("Logic error."),
            Ok(r) => r
        }
    }

    pub fn numerator(self) -> Numerator {
        self.numerator
    }

    pub fn denominator(self) -> Denominator {
        self.denominator
    }

    pub fn value_of(numerator: u8, denominator: u8) -> Result<Rhythm, RhythmError> {
        let numerator = Numerator::from_value(numerator);
        let numerator = match numerator {
            Err(NumeratorError::InvalidValue(v)) => return Err(RhythmError::NumeratorError(v)),
            Err(_) => panic!("Logic error."),
            Ok(n) => n,
        };

        let denominator = Denominator::from_value(denominator);
        let denominator = match denominator {
            Err(DenominatorError::InvalidValue(v)) => return Err(RhythmError::DenominatorError(v)),
            Ok(d) => d,
        };

        Ok(Self {numerator, denominator})
    }

    pub fn tick_len(self) -> u32 {
        ((self.numerator.0 as i32) * Duration::TICK_RESOLUTION * 4 / (self.denominator.value() as i32)) as u32
    }
}

impl Default for Rhythm {
    fn default() -> Self {
        Self::new(4, 4)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::json;

    use super::Rhythm;
    use super::RhythmError;

    #[test]
    #[should_panic]
    fn new_with_invalid_denominator() {
        Rhythm::new(1, 3);
    }

    #[test]
    #[should_panic]
    fn new_with_invalid_numerator() {
        Rhythm::new(100, 4);
    }

    #[test]
    fn value_of_with_invalid_denominator() {
        assert_eq!(Rhythm::value_of(1, 3).err().unwrap(), RhythmError::DenominatorError(3))
    }

    #[test]
    fn value_of_with_invalid_numerator() {
        assert_eq!(Rhythm::value_of(100, 2).err().unwrap(), RhythmError::NumeratorError(100))
    }

    #[test]
    fn value_of() {
        assert_eq!(Rhythm::value_of(2, 4).ok().unwrap(), Rhythm::new(2, 4))
    }

    #[test]
    fn tick_len() {
        assert_eq!(Rhythm::value_of(4, 4).ok().unwrap().tick_len(), 240 * 4);
        assert_eq!(Rhythm::value_of(2, 4).ok().unwrap().tick_len(), 240 * 2);
        assert_eq!(Rhythm::value_of(2, 2).ok().unwrap().tick_len(), 480 * 2);
        assert_eq!(Rhythm::value_of(6, 8).ok().unwrap().tick_len(), 120 * 6);
    }

    #[test]
    fn can_serialize_to_json() {
        let json_str = serde_json::to_string(&Rhythm::new(3, 4)).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({
                "numerator": 3,
                "denominator": "D4"
            })
        );
    }

    #[test]
    fn can_deserialize_from_json() {
        let rhythm: Rhythm = serde_json::from_str(r#"
            {
                "numerator": 3,
                "denominator": "D4"
            }
        "#).unwrap();
        assert_eq!(rhythm, Rhythm::new(3, 4));
    }
}
