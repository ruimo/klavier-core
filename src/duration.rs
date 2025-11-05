use std::{fmt, hash::Hash};

/// Note duration numerator (note type).
///
/// Represents the type of note: whole note, half note, quarter note, etc.
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Numerator {
    /// Whole note (1)
    Whole,
    /// Half note (1/2)
    Half,
    /// Quarter note (1/4)
    Quarter,
    /// Eighth note (1/8)
    N8th,
    /// Sixteenth note (1/16)
    N16th,
    /// Thirty-second note (1/32)
    N32nd,
    /// Sixty-fourth note (1/64)
    N64th,
    /// One hundred twenty-eighth note (1/128)
    N128th,
}

impl Default for Numerator {
    fn default() -> Self {
        Self::Quarter
    }
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

/// Tuplet denominator for note duration.
///
/// Represents the denominator in tuplets (e.g., 3 for triplets, 5 for quintuplets).
/// A value of 2 means normal (non-tuplet) duration.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from="SerializedDenominator")]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Denominator(u8);

impl Default for Denominator {
    fn default() -> Self {
        Self(2)
    }
}

#[derive(serde::Deserialize)]
struct SerializedDenominator(u8);

impl From<SerializedDenominator> for Denominator {
    fn from(ser: SerializedDenominator) -> Self {
        Denominator::from_value(ser.0).unwrap_or(Denominator(2))
    }
}

impl fmt::Display for Denominator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

/// Number of dots extending a note's duration.
///
/// Each dot adds half the value of the previous duration.
/// For example, a dotted quarter note = quarter + eighth.
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Dots(u8);

impl Default for Dots {
    fn default() -> Self {
        Self::ZERO
    }
}

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

/// Represents the duration of a musical note.
///
/// A duration combines:
/// - **Numerator**: The note type (whole, half, quarter, etc.)
/// - **Denominator**: The tuplet grouping (2 = normal, 3 = triplet, etc.)
/// - **Dots**: Number of dots extending the duration
///
/// # Examples
///
/// ```
/// # use klavier_core::duration::{Duration, Numerator, Denominator, Dots};
/// // Quarter note
/// let quarter = Duration::new(
///     Numerator::Quarter,
///     Denominator::from_value(2).unwrap(),
///     Dots::ZERO
/// );
/// assert_eq!(quarter.tick_length(), 240);
///
/// // Dotted quarter note
/// let dotted_quarter = Duration::new(
///     Numerator::Quarter,
///     Denominator::from_value(2).unwrap(),
///     Dots::ONE
/// );
/// assert_eq!(dotted_quarter.tick_length(), 360);
/// ```
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct Duration {
    /// The note type (whole, half, quarter, etc.).
    pub numerator: Numerator,
    /// The tuplet denominator (2 = normal, 3 = triplet, etc.).
    pub denominator: Denominator,
    /// The number of dots (0-7).
    pub dots: Dots,
}

impl Duration {
    /// Ticks per quarter note (MIDI standard).
    pub const TICK_RESOLUTION: i32 = 240;
    /// Maximum tick length (whole note * 2).
    pub const MAX_TICK_LENGTH: i32 = Duration::TICK_RESOLUTION * 8;
    /// Minimum denominator value.
    pub const MIN_DENOMINATOR: u8 = 2;
    /// Maximum denominator value.
    pub const MAX_DENOMINATOR: u8 = 255;
    /// Maximum number of dots.
    pub const MAX_DOT: u8 = 7;
    /// Maximum numerator ordinal.
    pub const MAX_NUMERATOR: u8 = 7;

    /// Creates a new duration.
    ///
    /// # Arguments
    ///
    /// * `numerator` - The note type.
    /// * `denominator` - The tuplet denominator.
    /// * `dots` - The number of dots.
    pub fn new(numerator: Numerator, denominator: Denominator, dots: Dots) -> Duration {
        Self { numerator, denominator, dots }
    }

    /// Calculates the duration in ticks.
    ///
    /// # Returns
    ///
    /// The duration in ticks, accounting for note type, tuplets, and dots.
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

    /// Creates a new duration with a different numerator.
    pub fn with_numerator(self, numerator: Numerator) -> Duration {
        Self::new(numerator, self.denominator, self.dots)
    }

    /// Creates a new duration with a different denominator.
    pub fn with_denominator(self, denominator: Denominator) -> Duration {
        Self::new(self.numerator, denominator, self.dots)
    }

    /// Creates a new duration with a different number of dots.
    pub fn with_dots(self, dots: Dots) -> Duration {
        Self::new(self.numerator, self.denominator, dots)
    }

    /// Returns the shorter of two durations.
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
        assert_eq!(d.numerator, Numerator::Half);
        assert_eq!(d.denominator.value(), 2);
        assert_eq!(d.dots.value(), 3);
    }

    #[test]
    fn tick_length() {
        assert_eq!(Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ZERO).tick_length(), 240);
        assert_eq!(Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO).tick_length(), 480);
        assert_eq!(Duration::new(Numerator::Quarter, Denominator::from_value(3).unwrap(), Dots::ZERO).tick_length(), 160);
        assert_eq!(Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::from_value(1).unwrap()).tick_length(), 360);
    }
}
