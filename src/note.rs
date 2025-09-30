
use std::rc::Rc;

use once_cell::unsync::Lazy;

use crate::channel::Channel;
use crate::clipper::Clipper;
use crate::clipper;
use crate::can_apply::CanApply;
use crate::duration::Duration;
use crate::trimmer::RateTrimmer;
use crate::pitch::Pitch;
use super::duration::{Numerator, Dots, Denominator};
use super::have_start_tick::{HaveBaseStartTick, HaveStartTick};
use super::percent::PercentU16;
use super::pitch::PitchError;
use super::trimmer::Trimmer;
use super::velocity::{Velocity, self};
use derive_builder::Builder;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TickError {
    Minus,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InvalidDot(i32);

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[derive(Debug, PartialEq, Eq, Clone, Builder)]
#[builder(default)]
pub struct Note {
    pub base_start_tick: u32,
    pub pitch: Pitch,
    pub duration: Duration,
    pub tie: bool,
    pub tied: bool,
    pub base_velocity: Velocity,
    pub start_tick_trimmer: Trimmer,
    pub duration_trimmer: RateTrimmer,
    pub velocity_trimmer: Trimmer,
    pub channel: Channel,
}

impl Note {
    #[inline]
    pub fn start_tick(&self) -> u32 {
        let tick = self.base_start_tick as i64 + self.start_tick_trimmer.sum() as i64;
        if tick < 0 { 0 } else { tick as u32 }
    }

    #[inline]
    pub fn tick_len(&self) -> u32 {
        self.duration_trimmer.apply(self.duration.tick_length())
    }

    pub fn up_score_offset(&self) -> Result<Self, PitchError> {
        self.pitch.up().map(|p| {
            Self {
                pitch: p,
                ..*self
            }
        })
    }
    
    pub fn down_score_offset(&self) -> Result<Self, PitchError> {
        self.pitch.down().map(|p| {
            Self {
                pitch: p,
                ..*self
            }
        })
    }
    
    pub fn with_duration(&self, d: Duration) -> Self {
        Self {
            duration: d,
            ..*self
        }
    }
    
    pub fn with_duration_numerator(&self, numerator: Numerator) -> Self {
        Self {
            duration: 
                if self.duration.numerator != numerator {
                    self.duration.with_numerator(numerator)
                } else {
                    self.duration
                },
            ..*self
        }
    }
    
    pub fn with_tick_added(&self, tick_delta: i32, is_trim: bool) -> Result<Self, TickError> {
        let tick = self.base_start_tick as i64 + tick_delta as i64;
        if tick < 0 {
            Err(TickError::Minus)
        } else if is_trim {
            let mut copied = self.clone();
            copied.start_tick_trimmer = self.start_tick_trimmer.added(tick_delta);
            Ok(copied)
        } else {
            Ok(
                Self {
                    base_start_tick: tick as u32,
                    ..*self
                }
            )
        }
    }
    
    pub fn drag(&self, tick_delta: i32, score_offset_delta: i32) -> Self {
        let tick = self.base_start_tick as i64 + tick_delta as i64;
        let pitch = self.pitch.with_score_offset_delta(score_offset_delta).unwrap();
        Self {
            base_start_tick: tick as u32,
            pitch,
            ..*self
        }
    }

    pub fn add_dots(&self, dots_to_add: i32) -> Result<Self, InvalidDot> {
        let new_dots = self.duration.dots.value() as i32 + dots_to_add;
        if new_dots < 0 || (Duration::MAX_DOT as i32) < new_dots {
            Err(InvalidDot(new_dots))
        } else {
            Ok(
                Self {
                    duration: self.duration.with_dots(Dots::from_value(new_dots as u8).unwrap()),
                    ..*self
                }
            )
        }
    }

    pub fn toggle_sharp(&self) -> Result<Self, PitchError> {
        self.pitch.toggle_sharp().map(|pitch| {
            Self {
                pitch,
                ..*self
            }
        })
    }

    pub fn toggle_flat(&self) -> Result<Self, PitchError> {
        self.pitch.toggle_flat().map(|pitch| {
            Self {
                pitch,
                ..*self
            }
        })
    }

    pub fn toggle_natural(&self) -> Result<Self, PitchError> {
        self.pitch.toggle_natural().map(|pitch| {
            Self {
                pitch,
                ..*self
            }
        })
    }

    pub fn toggle_tie(&self) -> Note {
        let mut tie = self.tie;
        let mut tied = self.tied;

        if ! tie && ! tied {
            tie = true;
            tied = false;
        } else if tie && ! tied {
            tie = false;
            tied = true;
        } else if ! tie && tied {
            tie = true;
            tied = true;
        } else {
            tie = false;
            tied = false;
        }

        Self {
            tie, tied,
            ..*self
        }
    }

    #[inline]
    pub fn base_velocity(&self) -> Velocity {
        self.base_velocity
    }

    pub fn velocity(&self) -> Velocity {
        let mut v = self.base_velocity.as_u8() as i32;
        v += self.velocity_trimmer.sum();
        if v < 0 { velocity::MIN }
        else if 127 < v { velocity::MAX }
        else { Velocity::new(v as u8) }
    }
}

impl Note {
    pub const MIN_TICK: i32 = 0;
    pub const MAX_SCORE_OFFSET: i32 = 76;
    pub const TICK_CLIPPER: Clipper<i32> = clipper::for_i32(0, i32::MAX);
    pub const VELOCITY_CLIPPER: Clipper<i16> = clipper::for_i16(0, 127);
    #[allow(clippy::declare_interior_mutable_const)]
    pub const LONGEST_TICK_LEN: Lazy<u32> = Lazy::new(||
        Duration::new(Numerator::Whole, Denominator::from_value(2).unwrap(), Dots::SEVEN).tick_length() * (PercentU16::MAX.to_f32() as u32)
    );
}

pub const MAX_TICK_LEN: i32 = Duration::TICK_RESOLUTION * 8;

impl HaveBaseStartTick for Note {
    fn base_start_tick(&self) -> u32 {
        self.base_start_tick
    }
}

impl HaveStartTick for Note {
    fn start_tick(&self) -> u32 {
        self.start_tick()
    }
}

impl HaveBaseStartTick for Rc<Note> {
    fn base_start_tick(&self) -> u32 {
        self.base_start_tick
    }
}

impl HaveStartTick for Rc<Note> {
    fn start_tick(&self) -> u32 {
        <Note>::start_tick(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{note::Note, pitch::{Pitch, self}, solfa::Solfa, octave::Octave, sharp_flat::SharpFlat, duration::{Duration, Numerator, Denominator, Dots}, trimmer::RateTrimmer, velocity::Velocity};

    use super::NoteBuilder;
    
    #[test]
    fn tick_len() {
        let note = Note {
            base_start_tick: 123,
            pitch: Pitch::new(Solfa::A, Octave::Oct1, SharpFlat::Null),
            duration: Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
            base_velocity: Velocity::new(10),
            duration_trimmer: RateTrimmer::new(1.0, 0.5, 2.0, 1.5), // duration_trimmer
            ..Default::default()
        };
        assert_eq!(note.tick_len(), 720);
    }
    
    #[test]
    fn up_score_offset() {
        let note = Note {
            base_start_tick: 123,
            pitch: pitch::MAX,
            duration: Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
            base_velocity: Velocity::new(10),
            duration_trimmer: RateTrimmer::new(1.0, 0.5, 2.0, 1.5),
            ..Default::default()
        };
        assert!(note.up_score_offset().is_err());
        
        let note = Note {
            base_start_tick: 123,
            pitch: Pitch::new(Solfa::A, Octave::Oct1, SharpFlat::Null),
            duration: Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
            base_velocity: Velocity::new(10),
            duration_trimmer: RateTrimmer::new(1.0, 0.5, 2.0, 1.5),
            ..Default::default()
        };
        assert_eq!(note.up_score_offset().unwrap().pitch, Pitch::new(Solfa::B, Octave::Oct1, SharpFlat::Null));
    }
    
    #[test]
    fn with_tick_added() {
        let note = Note {
            base_start_tick: 123,
            pitch: pitch::MAX,
            duration: Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
            base_velocity: Velocity::new(10),
            duration_trimmer: RateTrimmer::new(1.0, 0.5, 2.0, 1.5),
            ..Default::default()
        };
        assert_eq!(note.with_tick_added(10, true).unwrap().start_tick(), 133);
        assert_eq!(note.with_tick_added(-122, true).unwrap().start_tick(), 1);
        assert_eq!(note.with_tick_added(-123, true).unwrap().start_tick(), 0);
        assert!(note.with_tick_added(-124, true).is_err());
    }
    
    #[test]
    fn builder() {
        let note_builder: NoteBuilder = NoteBuilder::default()
          .base_start_tick(12u32)
          .base_velocity(Velocity::new(99))
          .clone();
        
        let note0 = note_builder.clone().base_start_tick(123u32).build().unwrap();
        let note1 = note_builder.clone().build().unwrap();
        
        assert_eq!(note0.base_start_tick, 123);
        assert_eq!(note1.base_start_tick, 12);
    }
}
