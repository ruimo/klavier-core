use std::hash::{Hash};

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Numerator {
    Whole,
    Half,
    Quarter,
    N8th,
    N16th,
    N32nd,
    N64th,
    N128th,
}

impl Numerator {
    pub const fn ord(self) -> u8 {
        match self {
            Numerator::Whole => 0,
            Numerator::Half => 1,
            Numerator::Quarter => 2,
            Numerator::N8th => 3,
            Numerator::N16th => 4,
            Numerator::N32nd => 5,
            Numerator::N64th => 6,
            Numerator::N128th => 7,
        }
    }

    pub const fn from_ord(ord: u8) -> Option<Numerator> {
        match ord {
            0 => Some(Numerator::Whole),
            1 => Some(Numerator::Half),
            2 => Some(Numerator::Quarter),
            3 => Some(Numerator::N8th),
            4 => Some(Numerator::N16th),
            5 => Some(Numerator::N32nd),
            6 => Some(Numerator::N64th),
            7 => Some(Numerator::N128th),
            _ => None,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from="SerializedDenominator")]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Denominator(u8);

#[derive(serde::Deserialize)]
struct SerializedDenominator(u8);

impl From<SerializedDenominator> for Denominator {
    fn from(ser: SerializedDenominator) -> Self {
        Denominator::from_value(ser.0).unwrap_or(Denominator(2))
    }
}

impl ToString for Denominator {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Denominator {
    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn from_value(value: u8) -> Option<Denominator> {
        if value < 2 {
            None
        } else {
            Some(Denominator(value))
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Dots(u8);

impl Dots {
    pub const ZERO: Dots = Dots(0);
    pub const ONE: Dots = Dots(1);
    pub const TWO: Dots = Dots(2);
    pub const THREE: Dots = Dots(3);
    pub const FOUR: Dots = Dots(4);
    pub const FIVE: Dots = Dots(5);
    pub const SIX: Dots = Dots(6);
    pub const SEVEN: Dots = Dots(7);

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn from_value(value: u8) -> Option<Dots> {
        if 7 < value {
            None
        } else {
            Some(Dots(value))
        }
    }
}

// numerator
// 0:Whole note, 1:half note, ... 7:128th note
//
// denominator
// 2:Normal, 3:3 group notes, ... 255
//
// dot
// 0-7
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Duration {
    numerator: Numerator,
    denominator: Denominator,
    dots: Dots,
}

impl Duration {
    pub const TICK_RESOLUTION: i32 = 240;
    pub const MAX_TICK_LENGTH: i32 = Duration::TICK_RESOLUTION * 8; // Max tick length = whole note * 2
    pub const MIN_DENOMINATOR: u8 = 2;
    pub const MAX_DENOMINATOR: u8 = 255;
    pub const MAX_DOT: u8 = 7;
    pub const MAX_NUMERATOR: u8 = 7;

    pub fn new(numerator: Numerator, denominator: Denominator, dots: Dots) -> Duration {
        Self { numerator, denominator, dots }
    }

    pub const fn numerator(self: Self) -> Numerator {
        self.numerator
    }

    pub const fn denominator(self: Self) -> Denominator {
        self.denominator
    }

    pub const fn dots(self: Self) -> Dots {
        self.dots
    }

    pub const fn tick_length(self) -> u32 {
        let numerator = self.numerator.ord();
        let len =
            if numerator <= 2 {
                (Duration::TICK_RESOLUTION << (2 - numerator)) as u32
            } else {
                (Duration::TICK_RESOLUTION >> (numerator - 2)) as u32
            };

        if self.dots.value() == 0 && self.denominator.value() == 2 {
            len
        } else {
            ((len + (len - (len >> self.dots.value()))) as i64 * 2 / (self.denominator.value() as i64)) as u32
        }
    }

    pub fn with_numerator(self, numerator: Numerator) -> Duration {
        Self::new(numerator, self.denominator, self.dots)
    }

    pub fn with_denominator(self, denominator: Denominator) -> Duration {
        Self::new(self.numerator, denominator, self.dots)
    }

    pub fn with_dots(self, dots: Dots) -> Duration {
        Self::new(self.numerator, self.denominator, dots)
    }

    pub fn min(self, other: Self) -> Self {
        if self.tick_length() < other.tick_length() { self } else { other }
    }
}

#[cfg(test)]
mod tests {
    use crate::duration::Duration;

    use super::{Numerator, Denominator, Dots};

    #[test]
    #[should_panic]
    fn low_denominator() {
        Duration::new(Numerator::Half, Denominator::from_value(1).unwrap(), Dots::ZERO);
    }

    #[test]
    #[should_panic]
    fn high_dot() {
        Duration::new(Numerator::N128th, Denominator::from_value(2).unwrap(), Dots::from_value(8).unwrap());
    }

    #[test]
    fn getter() {
        let d = Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::from_value(3).unwrap());
        assert_eq!(d.numerator(), Numerator::Half);
        assert_eq!(d.denominator().value(), 2);
        assert_eq!(d.dots().value(), 3);
    }

    #[test]
    fn tick_length() {
        assert_eq!(Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ZERO).tick_length(), 240);
        assert_eq!(Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO).tick_length(), 480);
        assert_eq!(Duration::new(Numerator::Quarter, Denominator::from_value(3).unwrap(), Dots::ZERO).tick_length(), 160);
        assert_eq!(Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::from_value(1).unwrap()).tick_length(), 360);
    }
}
