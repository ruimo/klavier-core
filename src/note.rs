
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

/// Error type for tick-related operations.
///
/// This error occurs when a tick calculation results in a negative value,
/// which is not allowed in the MIDI timing system.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TickError {
    /// Indicates that a tick calculation resulted in a negative value.
    Minus,
}

/// Error type for invalid dot count in note duration.
///
/// This error occurs when attempting to set a dot count that is outside
/// the valid range (0 to `Duration::MAX_DOT`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InvalidDot(i32);

/// Represents a musical note with timing, pitch, duration, and velocity information.
///
/// A `Note` is the fundamental building block of musical composition in this library.
/// It contains all the information needed to play a single note, including:
/// - Timing information (start tick)
/// - Pitch (musical note and octave)
/// - Duration (note length)
/// - Velocity (how hard the note is played)
/// - Tie information (for connecting notes)
/// - Trimmers for fine-tuning timing, duration, and velocity
///
/// # Examples
///
/// ```
/// use klavier_core::note::{Note, NoteBuilder};
/// use klavier_core::pitch::Pitch;
/// use klavier_core::solfa::Solfa;
/// use klavier_core::octave::Octave;
/// use klavier_core::sharp_flat::SharpFlat;
/// use klavier_core::duration::{Duration, Numerator, Denominator, Dots};
/// use klavier_core::velocity::Velocity;
///
/// // Create a note using the builder pattern
/// let note = NoteBuilder::default()
///     .base_start_tick(0)
///     .pitch(Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null))
///     .duration(Duration::new(Numerator::Whole, Denominator::from_value(4).unwrap(), Dots::ZERO))
///     .base_velocity(Velocity::new(100))
///     .build()
///     .unwrap();
/// ```
#[derive(serde::Deserialize, serde::Serialize, Default)]
#[derive(Debug, PartialEq, Eq, Clone, Builder)]
#[builder(default)]
pub struct Note {
    /// The base start tick position of the note (before any trimmer adjustments).
    pub base_start_tick: u32,
    
    /// The pitch of the note (musical note, octave, and accidental).
    pub pitch: Pitch,
    
    /// The duration of the note (note length).
    pub duration: Duration,
    
    /// Whether this note is tied to the next note (tie start).
    pub tie: bool,
    
    /// Whether this note is tied from the previous note (tie end).
    pub tied: bool,
    
    /// The base velocity of the note (before any trimmer adjustments).
    pub base_velocity: Velocity,
    
    /// Trimmer for adjusting the start tick position.
    pub start_tick_trimmer: Trimmer,
    
    /// Trimmer for adjusting the duration as a rate multiplier.
    pub duration_trimmer: RateTrimmer,
    
    /// Trimmer for adjusting the velocity.
    pub velocity_trimmer: Trimmer,
    
    /// The MIDI channel for this note.
    pub channel: Channel,
}

impl Note {
    /// Returns the actual start tick of the note after applying the start tick trimmer.
    ///
    /// This method calculates the final start tick by adding the base start tick
    /// and the trimmer adjustment. If the result would be negative, it returns 0.
    ///
    /// # Returns
    ///
    /// The actual start tick position (always >= 0).
    #[inline]
    pub fn start_tick(&self) -> u32 {
        let tick = self.base_start_tick as i64 + self.start_tick_trimmer.sum() as i64;
        if tick < 0 { 0 } else { tick as u32 }
    }

    /// Returns the actual tick length of the note after applying the duration trimmer.
    ///
    /// This method calculates the final duration by applying the rate trimmer
    /// to the base duration's tick length.
    ///
    /// # Returns
    ///
    /// The actual tick length of the note.
    #[inline]
    pub fn tick_len(&self) -> u32 {
        self.duration_trimmer.apply(self.duration.tick_length())
    }

    /// Creates a new note with the pitch raised by one semitone.
    ///
    /// # Returns
    ///
    /// - `Ok(Note)` - A new note with the pitch raised by one semitone.
    /// - `Err(PitchError)` - If the pitch is already at the maximum value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use klavier_core::note::Note;
    /// # use klavier_core::pitch::Pitch;
    /// # use klavier_core::solfa::Solfa;
    /// # use klavier_core::octave::Octave;
    /// # use klavier_core::sharp_flat::SharpFlat;
    /// let note = Note {
    ///     pitch: Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
    ///     ..Default::default()
    /// };
    /// let higher_note = note.up_score_offset().unwrap();
    /// assert_eq!(higher_note.pitch, Pitch::new(Solfa::D, Octave::Oct4, SharpFlat::Null));
    /// ```
    pub fn up_score_offset(&self) -> Result<Self, PitchError> {
        self.pitch.up().map(|p| {
            Self {
                pitch: p,
                ..*self
            }
        })
    }
    
    /// Creates a new note with the pitch lowered by one semitone.
    ///
    /// # Returns
    ///
    /// - `Ok(Note)` - A new note with the pitch lowered by one semitone.
    /// - `Err(PitchError)` - If the pitch is already at the minimum value.
    pub fn down_score_offset(&self) -> Result<Self, PitchError> {
        self.pitch.down().map(|p| {
            Self {
                pitch: p,
                ..*self
            }
        })
    }
    
    /// Creates a new note with the specified duration.
    ///
    /// # Arguments
    ///
    /// * `d` - The new duration for the note.
    ///
    /// # Returns
    ///
    /// A new note with the specified duration.
    pub fn with_duration(&self, d: Duration) -> Self {
        Self {
            duration: d,
            ..*self
        }
    }
    
    /// Creates a new note with the specified duration numerator.
    ///
    /// This method only updates the duration if the numerator is different
    /// from the current one, optimizing for cases where no change is needed.
    ///
    /// # Arguments
    ///
    /// * `numerator` - The new numerator for the note's duration.
    ///
    /// # Returns
    ///
    /// A new note with the specified duration numerator.
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
    
    /// Creates a new note with the start tick adjusted by the specified delta.
    ///
    /// # Arguments
    ///
    /// * `tick_delta` - The amount to add to the start tick (can be negative).
    /// * `is_trim` - If `true`, adjusts the trimmer; if `false`, adjusts the base start tick.
    ///
    /// # Returns
    ///
    /// - `Ok(Note)` - A new note with the adjusted start tick.
    /// - `Err(TickError::Minus)` - If the resulting tick would be negative.
    ///
    /// # Examples
    ///
    /// ```
    /// # use klavier_core::note::Note;
    /// let note = Note {
    ///     base_start_tick: 100,
    ///     ..Default::default()
    /// };
    /// let later_note = note.with_tick_added(50, false).unwrap();
    /// assert_eq!(later_note.base_start_tick, 150);
    /// ```
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
    
    /// Creates a new note with both timing and pitch adjusted (for drag operations).
    ///
    /// This method is typically used for interactive editing where a note is
    /// being dragged both horizontally (time) and vertically (pitch).
    ///
    /// # Arguments
    ///
    /// * `tick_delta` - The amount to add to the start tick.
    /// * `score_offset_delta` - The amount to adjust the pitch (in semitones).
    ///
    /// # Returns
    ///
    /// A new note with adjusted timing and pitch.
    ///
    /// # Panics
    ///
    /// Panics if the pitch adjustment would result in an invalid pitch.
    pub fn drag(&self, tick_delta: i32, score_offset_delta: i32) -> Self {
        let tick = self.base_start_tick as i64 + tick_delta as i64;
        let pitch = self.pitch.with_score_offset_delta(score_offset_delta).unwrap();
        Self {
            base_start_tick: tick as u32,
            pitch,
            ..*self
        }
    }

    /// Creates a new note with additional dots added to the duration.
    ///
    /// Dots extend the duration of a note. Each dot adds half the value of the
    /// previous duration component.
    ///
    /// # Arguments
    ///
    /// * `dots_to_add` - The number of dots to add (can be negative to remove dots).
    ///
    /// # Returns
    ///
    /// - `Ok(Note)` - A new note with the adjusted dot count.
    /// - `Err(InvalidDot)` - If the resulting dot count is outside the valid range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use klavier_core::note::Note;
    /// # use klavier_core::duration::{Duration, Numerator, Denominator, Dots};
    /// let note = Note {
    ///     duration: Duration::new(Numerator::Whole, Denominator::from_value(4).unwrap(), Dots::ZERO),
    ///     ..Default::default()
    /// };
    /// let dotted_note = note.add_dots(1).unwrap();
    /// assert_eq!(dotted_note.duration.dots, Dots::ONE);
    /// ```
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

    /// Creates a new note with the sharp accidental toggled.
    ///
    /// If the note has no accidental, it becomes sharp.
    /// If the note is already sharp, it becomes natural.
    ///
    /// # Returns
    ///
    /// - `Ok(Note)` - A new note with the sharp toggled.
    /// - `Err(PitchError)` - If the operation would result in an invalid pitch.
    pub fn toggle_sharp(&self) -> Result<Self, PitchError> {
        self.pitch.toggle_sharp().map(|pitch| {
            Self {
                pitch,
                ..*self
            }
        })
    }

    /// Creates a new note with the flat accidental toggled.
    ///
    /// If the note has no accidental, it becomes flat.
    /// If the note is already flat, it becomes natural.
    ///
    /// # Returns
    ///
    /// - `Ok(Note)` - A new note with the flat toggled.
    /// - `Err(PitchError)` - If the operation would result in an invalid pitch.
    pub fn toggle_flat(&self) -> Result<Self, PitchError> {
        self.pitch.toggle_flat().map(|pitch| {
            Self {
                pitch,
                ..*self
            }
        })
    }

    /// Creates a new note with the natural accidental toggled.
    ///
    /// If the note has an accidental (sharp or flat), it becomes natural.
    /// If the note is already natural, this may have no effect depending on the key signature.
    ///
    /// # Returns
    ///
    /// - `Ok(Note)` - A new note with the natural toggled.
    /// - `Err(PitchError)` - If the operation would result in an invalid pitch.
    pub fn toggle_natural(&self) -> Result<Self, PitchError> {
        self.pitch.toggle_natural().map(|pitch| {
            Self {
                pitch,
                ..*self
            }
        })
    }

    /// Creates a new note with the tie state toggled through its cycle.
    ///
    /// The tie state cycles through four states:
    /// 1. No tie: `tie=false, tied=false`
    /// 2. Tie start: `tie=true, tied=false`
    /// 3. Tie end: `tie=false, tied=true`
    /// 4. Tie middle: `tie=true, tied=true`
    ///
    /// # Returns
    ///
    /// A new note with the tie state advanced to the next state in the cycle.
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

    /// Returns the base velocity of the note (before trimmer adjustments).
    ///
    /// # Returns
    ///
    /// The base velocity value.
    #[inline]
    pub fn base_velocity(&self) -> Velocity {
        self.base_velocity
    }

    /// Returns the actual velocity of the note after applying the velocity trimmer.
    ///
    /// The velocity is clamped to the valid MIDI range (0-127).
    ///
    /// # Returns
    ///
    /// The actual velocity value (0-127).
    pub fn velocity(&self) -> Velocity {
        let mut v = self.base_velocity.as_u8() as i32;
        v += self.velocity_trimmer.sum();
        if v < 0 { velocity::MIN }
        else if 127 < v { velocity::MAX }
        else { Velocity::new(v as u8) }
    }
}

impl Note {
    /// The minimum tick value (always 0).
    pub const MIN_TICK: i32 = 0;
    
    /// The maximum score offset value (76 semitones, covering the full MIDI range).
    pub const MAX_SCORE_OFFSET: i32 = 76;
    
    /// Clipper for tick values, ensuring they stay within valid range.
    pub const TICK_CLIPPER: Clipper<i32> = clipper::for_i32(0, i32::MAX);
    
    /// Clipper for velocity values, ensuring they stay within MIDI range (0-127).
    pub const VELOCITY_CLIPPER: Clipper<i16> = clipper::for_i16(0, 127);
    
    /// The longest possible tick length for a note.
    ///
    /// This is calculated as a whole note with 7 dots at the slowest denominator,
    /// multiplied by the maximum duration trimmer rate.
    #[allow(clippy::declare_interior_mutable_const)]
    pub const LONGEST_TICK_LEN: Lazy<u32> = Lazy::new(||
        Duration::new(Numerator::Whole, Denominator::from_value(2).unwrap(), Dots::SEVEN).tick_length() * (PercentU16::MAX.to_f32() as u32)
    );
}

/// The maximum tick length for a note (8 measures at standard resolution).
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
