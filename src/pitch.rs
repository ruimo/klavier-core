use crate::{solfa::Solfa, key::Key};
use crate::octave::Octave;
use crate::sharp_flat::SharpFlat;
use std::fmt::{self};

use super::octave;

#[derive(Debug)]
pub enum PitchError {
    TooLow(Solfa, Octave, SharpFlat, i32),
    TooHigh(Solfa, Octave, SharpFlat, i32),
    InvalidScoreOffset(i32),
}

impl fmt::Display for PitchError {
    fn fmt(self: &PitchError, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PitchError::TooLow(solfa, octave, sharp_flat, value) => f.write_fmt(
                format_args!("Pitch({:?}, {:?}, {:?}) is too low {}", solfa, octave, sharp_flat, value)
            ),
            PitchError::TooHigh(solfa, octave, sharp_flat, value) => f.write_fmt(
                format_args!("Pitch({:?}, {:?}, {:?}) is too high {}", solfa, octave, sharp_flat, value)
            ),
            PitchError::InvalidScoreOffset(score_offset) => f.write_fmt(
                format_args!("Score offset error({})", score_offset)
            )
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from = "PitchSerializedForm")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pitch {
    solfa: Solfa,
    octave: Octave,
    sharp_flat: SharpFlat,

    #[serde(skip)]    
    value: u8,
    #[serde(skip)]    
    score_offset: i8,
}

impl Default for Pitch {
    fn default() -> Self {
        DEFAULT
    }
}

impl From<PitchSerializedForm> for Pitch {
    fn from(from: PitchSerializedForm) -> Self {
        Self::new(from.solfa, from.octave, from.sharp_flat)
    }
}

#[derive(serde::Deserialize)]
struct PitchSerializedForm {
    solfa: Solfa,
    octave: Octave,
    sharp_flat: SharpFlat,
}

pub const MAX_VALUE: i8 = 127;
pub const MIN_VALUE: i8 = 0;

pub const MIN: Pitch = Pitch::new(Solfa::C, Octave::OctM2, SharpFlat::Null);
pub const MAX: Pitch = Pitch::new(Solfa::G, Octave::Oct8, SharpFlat::Null);
const DEFAULT: Pitch = Pitch::new(Solfa::A, Octave::Oct4, SharpFlat::Null);

pub const MIN_SCORE_OFFSET: i32 = 0;
pub const MAX_SCORE_OFFSET: i32 = 74;

impl Pitch {
    pub const fn new(solfa: Solfa, octave: Octave, sharp_flat: SharpFlat) -> Self {
        match Self::value_of(solfa, octave, sharp_flat) {
            Err(_pe) => panic!("Logic error."),
            Ok(p) => p
        }
    }

    pub fn apply_key(self, key: Key) -> Result<Self, PitchError> {
        if self.sharp_flat == SharpFlat::Null {
            match Key::SOLFAS.get(&key) {
                Some(solfas) =>
                    if solfas.contains(&self.solfa) {
                        let sharp_flat = if key.is_flat() { SharpFlat::Flat } else { SharpFlat::Sharp };
                        Pitch::value_of(self.solfa, self.octave, sharp_flat)
                    } else {
                        Ok(self)
                    }
                None => Ok(self)
            }
        } else {
            Ok(self)
        }
    }

    pub const fn to_value(solfa: Solfa, octave: Octave, sharp_flat: SharpFlat) -> i32 {
        solfa.pitch_offset() + (octave.value() + Octave::BIAS_VALUE) * 12 + sharp_flat.offset()
    }

    pub const fn value_of(solfa: Solfa, octave: Octave, sharp_flat: SharpFlat) -> Result<Self, PitchError> {
        let v: i32 = Self::to_value(solfa, octave, sharp_flat);
        if v < (MIN_VALUE as i32) {
            Err(PitchError::TooLow(solfa, octave, sharp_flat, v))
        } else if (MAX_VALUE as i32) < v {
            Err(PitchError::TooHigh(solfa, octave, sharp_flat, v))
        } else {
            let so = (solfa.score_offset() + 7 * octave.offset()) as i8;
            Ok(
                Self {
                    solfa: solfa,
                    octave: octave,
                    sharp_flat: sharp_flat,
                    value: v as u8,
                    score_offset: so
                }
            )
        }
    }

    pub fn toggle_sharp(self) -> Result<Self, PitchError> {
        let sharp_flat = match self.sharp_flat {
            SharpFlat::Sharp => SharpFlat::DoubleSharp,
            SharpFlat::DoubleSharp => SharpFlat::Null,
            _ => SharpFlat::Sharp
        };
        Self::value_of(self.solfa, self.octave, sharp_flat)
    }

    pub fn toggle_flat(self) -> Result<Self, PitchError> {
        let sharp_flat = match self.sharp_flat {
            SharpFlat::Flat => SharpFlat::DoubleFlat,
            SharpFlat::DoubleFlat => SharpFlat::Null,
            _ => SharpFlat::Flat
        };
        Self::value_of(self.solfa, self.octave, sharp_flat)
    }

    pub fn toggle_natural(self) -> Result<Self, PitchError> {
        let sharp_flat = match self.sharp_flat {
            SharpFlat::Natural => SharpFlat::Null,
            _ => SharpFlat::Natural
        };
        Self::value_of(self.solfa, self.octave, sharp_flat)
    }

    pub fn from_score_offset(score_offset: i32) -> Self {
        let score_offset = 
            if score_offset < MIN_SCORE_OFFSET {
                MIN_SCORE_OFFSET
            } else if MAX_SCORE_OFFSET < score_offset {
                MAX_SCORE_OFFSET
            } else {
                score_offset
            };

        let (solfa, octave) = Self::score_offset_to_solfa_octave(score_offset);
        Self::value_of(solfa, octave, SharpFlat::Null).unwrap()
    }

    pub fn with_score_offset_delta(self, score_offset_delta: i32) -> Result<Self, PitchError> {
        let score_offset = self.score_offset as i32 + score_offset_delta;
        if score_offset < MIN_SCORE_OFFSET || MAX_SCORE_OFFSET < score_offset {
            return Err(PitchError::InvalidScoreOffset(score_offset));
        }

        let (solfa, octave) = Self::score_offset_to_solfa_octave(score_offset);
        Self::value_of(solfa, octave, self.sharp_flat)
    }

    pub fn score_offset_to_solfa_octave(score_offset: i32) -> (Solfa, Octave) {
        let octave_offset = score_offset / 7;
        let solfa_offset = score_offset - (octave_offset * 7);
        let solfa = Solfa::from_score_offset(solfa_offset as i32);
        let octave = Octave::from_score_offset(octave_offset as i32).unwrap();
        (solfa, octave)
    }

    #[inline]
    pub const fn score_offset(self) -> i8 { self.score_offset }

    #[inline]
    pub const fn sharp_flat(self) -> SharpFlat {
        self.sharp_flat
    }

    pub fn up(self) -> Result<Self, PitchError> {
        let mut solfa = self.solfa;
        let mut octave = self.octave;

        if solfa == Solfa::B {
            if octave != octave::MAX {
                octave += 1;
                solfa = Solfa::C;
            } else {
                return Err(PitchError::TooHigh(solfa, octave, self.sharp_flat, -1));
            }
        } else {
            solfa += 1;
        }
        Self::value_of(solfa, octave, self.sharp_flat)
    }

    pub fn down(self) -> Result<Self, PitchError> {
        let mut solfa = self.solfa;
        let mut octave = self.octave;

        if solfa == Solfa::C {
            if octave != octave::MIN {
                octave -= 1;
                solfa = Solfa::B;
            } else {
                return Err(PitchError::TooLow(solfa, octave, self.sharp_flat, -1));
            }
        } else {
            solfa -= 1;
        }
        Self::value_of(solfa, octave, self.sharp_flat)
    }

    #[inline]
    pub fn solfa(self) -> Solfa {
        self.solfa
    }

    #[inline]
    pub fn octave(self) -> Octave {
        self.octave
    }

    #[inline]
    pub fn value(self) -> u8 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use crate::key::Key;
    use crate::pitch::MAX_SCORE_OFFSET;
    use crate::pitch::MIN_SCORE_OFFSET;
    use crate::pitch::Pitch;
    use crate::pitch::MIN;
    use crate::pitch::MAX;
    use crate::solfa::Solfa;
    use crate::octave::Octave;
    use crate::sharp_flat::SharpFlat;
    use serde_json::Value;
    use serde_json::json;

    #[test]
    #[should_panic]
    fn too_low() {
        Pitch::new(Solfa::C, Octave::value_of(-2).unwrap(), SharpFlat::Flat);
    }

    #[test]
    fn lowest() {
        assert_eq!(0, MIN.value);
        assert_eq!(MIN_SCORE_OFFSET, MIN.score_offset() as i32);
    }

    #[test]
    fn f7() {
        assert_eq!(113, Pitch::new(Solfa::F, Octave::Oct7, SharpFlat::Null).value);
    }

    #[test]
    fn highest() {
        assert_eq!(127, MAX.value);
        assert_eq!(MAX_SCORE_OFFSET, MAX.score_offset() as i32);
    }

    #[test]
    #[should_panic]
    fn too_high() {
        Pitch::new(Solfa::G, Octave::Oct8, SharpFlat::Sharp);
    }

    #[test]
    fn lowest_score_offset() {
        assert_eq!(0, Pitch::new(Solfa::C, Octave::OctM2, SharpFlat::Null).score_offset());
    }

    #[test]
    fn score_offset() {
        assert_eq!(1, Pitch::new(Solfa::D, Octave::OctM2, SharpFlat::Null).score_offset());
        assert_eq!(6, Pitch::new(Solfa::B, Octave::OctM2, SharpFlat::Null).score_offset());
        assert_eq!(7, Pitch::new(Solfa::C, Octave::OctM1, SharpFlat::Null).score_offset());
    }

    #[test]
    #[should_panic]
    fn up_err() {
        MAX.up().unwrap();
    } 

    #[test]
    fn up() {
        assert_eq!(
            Pitch::new(Solfa::F, Octave::Oct8, SharpFlat::Null).up().unwrap(),
            Pitch::new(Solfa::G, Octave::Oct8, SharpFlat::Null)
        );
        assert_eq!(
            Pitch::new(Solfa::B, Octave::Oct7, SharpFlat::Null).up().unwrap(),
            Pitch::new(Solfa::C, Octave::Oct8, SharpFlat::Null)
        );
    }

    #[test]
    fn down() {
        assert_eq!(
            Pitch::new(Solfa::C, Octave::Oct8, SharpFlat::Null).down().unwrap(),
            Pitch::new(Solfa::B, Octave::Oct7, SharpFlat::Null)
        );
        assert_eq!(
            Pitch::new(Solfa::D, Octave::Oct7, SharpFlat::Null).down().unwrap(),
            Pitch::new(Solfa::C, Octave::Oct7, SharpFlat::Null)
        );
    }

    #[test]
    #[should_panic]
    fn down_err() {
        MIN.down().unwrap();
    } 

    #[test]
    fn from_score_offset() {
        assert_eq!(MIN, Pitch::from_score_offset(MIN_SCORE_OFFSET - 1));
        assert_eq!(MAX, Pitch::from_score_offset(MAX_SCORE_OFFSET + 1));

        assert_eq!(MIN, Pitch::from_score_offset(MIN_SCORE_OFFSET));
        assert_eq!(MAX, Pitch::from_score_offset(MAX_SCORE_OFFSET));
    }

    #[test]
    fn with_score_offset_delta() {
        assert_eq!(
            Pitch::new(Solfa::C, Octave::Oct8, SharpFlat::Flat).with_score_offset_delta(1).unwrap(),
            Pitch::new(Solfa::D, Octave::Oct8, SharpFlat::Flat)
        );

        assert_eq!(
            Pitch::new(Solfa::C, Octave::Oct7, SharpFlat::Sharp).with_score_offset_delta(8).unwrap(),
            Pitch::new(Solfa::D, Octave::Oct8, SharpFlat::Sharp)
        );

        assert_eq!(
            Pitch::new(Solfa::C, Octave::Oct7, SharpFlat::Sharp).with_score_offset_delta(-8).unwrap(),
            Pitch::new(Solfa::B, Octave::Oct5, SharpFlat::Sharp)
        );
    }

    #[test]
    #[should_panic]
    fn with_score_offset_delta_min_error() {
        let _ = MIN.with_score_offset_delta(-1).unwrap();
    }

    #[test]
    #[should_panic]
    fn with_score_offset_delta_max_error() {
        let _ = MAX.with_score_offset_delta(1).unwrap();
    }

    #[test]
    fn can_serialize_pitch() {
        let json_str = serde_json::to_string(&Pitch::new(
            Solfa::C, Octave::Oct2, SharpFlat::DoubleFlat
        )).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({
                "octave": "Oct2",
                "sharp_flat": "DoubleFlat",
                "solfa": "C"
            })
        );
    }

    #[test]
    fn can_deserialize_pitch() {
        let pitch: Pitch = serde_json::from_str(r#"
            {
                "octave": "Oct2",
                "sharp_flat": "DoubleFlat",
                "solfa": "C"
            }"#).unwrap();
        let expected = Pitch::new(Solfa::C, Octave::Oct2, SharpFlat::DoubleFlat);
        assert_eq!(pitch, expected);
        assert_eq!(pitch.score_offset, expected.score_offset);
    }

    #[test]
    fn apply() {
        let pitch = Pitch::new(Solfa::F, Octave::Oct1, SharpFlat::Null);
        assert_eq!(pitch.apply_key(Key::SHARP_1).unwrap(), Pitch::new(Solfa::F, Octave::Oct1, SharpFlat::Sharp));

        let pitch = Pitch::new(Solfa::F, Octave::Oct1, SharpFlat::Flat);
        assert_eq!(pitch.apply_key(Key::SHARP_1).unwrap(), Pitch::new(Solfa::F, Octave::Oct1, SharpFlat::Flat));

        let pitch = Pitch::new(Solfa::E, Octave::Oct1, SharpFlat::Null);
        assert_eq!(pitch.apply_key(Key::FLAT_2).unwrap(), Pitch::new(Solfa::E, Octave::Oct1, SharpFlat::Flat));

        let pitch = Pitch::new(Solfa::E, Octave::Oct1, SharpFlat::Sharp);
        assert_eq!(pitch.apply_key(Key::FLAT_2).unwrap(), Pitch::new(Solfa::E, Octave::Oct1, SharpFlat::Sharp));

        let pitch = Pitch::new(Solfa::F, Octave::Oct1, SharpFlat::Null);
        assert_eq!(pitch.apply_key(Key::FLAT_2).unwrap(), Pitch::new(Solfa::F, Octave::Oct1, SharpFlat::Null));
    }
}
