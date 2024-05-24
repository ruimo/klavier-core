use std::{num::{ParseFloatError, ParseIntError}, str::FromStr};

use crate::{duration, location::{parse_location, Location}, percent::PercentU16, rhythm, tempo::TempoValue, velocity::Velocity};

#[derive(Debug)]
pub struct Validated<T>(pub T);

#[inline]
pub fn from_str<T, E>(s: &str) -> Result<Validated<T>, E> 
  where Validated<T>: FromStr<Err = E>
{
    Validated::<T>::from_str(s)
}

#[derive(Debug)]
pub struct TextInput<T, E> {
    validated: Result<Validated<T>, E>,
    buffer: String,
}

impl<T, E> TextInput<T, E>
    where Validated<T>: FromStr<Err = E>
{
    pub fn from_string(s: String) -> Self {
        let validated: Result<Validated<T>, E> = from_str(&s);
        Self { validated, buffer: s }
    }

    pub fn mutate<R, M>(&mut self, mutator: M) -> R
      where M: FnOnce(&mut String) -> R
    {
        let r = (mutator)(&mut self.buffer);
        self.validated = from_str(&self.buffer);
        r
    }

    #[inline]
    pub fn input(&self) -> &str {
        &self.buffer
    }

    #[inline]
    pub fn validate(&self) -> &Result<Validated<T>, E> {
        &self.validated
    }
}

pub enum LocationParseError {
    InvalidFormat,
}

impl FromStr for Validated<Option<Location>> {
    type Err = LocationParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Validated(None))
        } else {
            match parse_location(s) {
                Some(loc) => Ok(Validated(Some(loc))),
                None => Err(LocationParseError::InvalidFormat),
            }
        }
    }
}

impl FromStr for Validated<Option<i16>> {
    type Err = ParseIntError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Validated(None))
        } else {
            s.parse::<i16>().map(|i| Validated(Some(i)))
        }
    }
}

impl FromStr for Validated<Option<TempoValue>> {
    type Err = ParseIntError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Validated(None))
        } else {
            s.parse::<u16>().map(|u| Validated(Some(TempoValue::new(u))))
        }
    }
}

impl FromStr for Validated<Option<Velocity>> {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Validated(None))
        } else {
            s.parse::<u8>().map(|u| Validated(Some(Velocity::new(u))))
        }
    }
}

impl FromStr for Validated<Option<rhythm::Numerator>> {
    type Err = rhythm::NumeratorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Validated(None))
        } else {
            from_str(s)
        }
    }
}

#[derive(Debug)]
pub enum DurationDenominatorError {
    ParseIntError,
    InvalidValue,
}

impl FromStr for Validated<Option<duration::Denominator>> {
    type Err = DurationDenominatorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Validated(None))
        } else {
            match s.parse::<u8>() {
                Ok(u) => match duration::Denominator::from_value(u) {
                    Some(d) => Ok(Validated(Some(d))),
                    None => Err(DurationDenominatorError::InvalidValue),
                }
                Err(_) => Err(DurationDenominatorError::ParseIntError)
            }
        }
    }
}

impl FromStr for Validated<Option<PercentU16>> {
    type Err = ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Validated(None))
        } else {
            match s.parse::<f32>() {
                Ok(pct) => Ok(Validated(Some(pct.into()))),
                Err(e) => Err(e),
            }
        }
    }
}