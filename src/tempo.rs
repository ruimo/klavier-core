use super::{note::TickError, have_start_tick::{HaveBaseStartTick, HaveStartTick}};

/// Minimum tempo value in BPM.
pub const MIN_TEMPO_VALUE: u16 = 1;
/// Maximum tempo value in BPM.
pub const MAX_TEMPO_VALUE: u16 = 999;

/// Minimum tempo constant.
pub const MIN_TEMPO: TempoValue = TempoValue(MIN_TEMPO_VALUE);
/// Maximum tempo constant.
pub const MAX_TEMPO: TempoValue = TempoValue(MAX_TEMPO_VALUE);

/// Tempo value in beats per minute (BPM).
///
/// Represents the speed of music, ranging from 1 to 999 BPM.
/// Values outside this range are clamped.
///
/// # Examples
///
/// ```
/// # use klavier_core::tempo::TempoValue;
/// let allegro = TempoValue::new(120);
/// assert_eq!(allegro.as_u16(), 120);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from="SerializedTempoValue")]
pub struct TempoValue(u16);

impl Default for TempoValue {
    fn default() -> Self {
        Self(120)
    }
}

#[derive(serde::Deserialize)]
struct SerializedTempoValue(u16);

impl From<SerializedTempoValue> for TempoValue {
    fn from(ser: SerializedTempoValue) -> Self {
        if MAX_TEMPO_VALUE < ser.0 { MAX_TEMPO }
        else if ser.0 < MIN_TEMPO_VALUE { MIN_TEMPO }
        else { TempoValue(ser.0) }
    }
}

/// Error type for tempo operations.
pub enum TempoError {
    /// The tempo value is invalid (out of range).
    InvalidValue(u16),
}

impl TempoValue {
    /// Creates a new tempo value.
    ///
    /// # Arguments
    ///
    /// * `value` - The tempo in BPM (1-999).
    ///
    /// # Panics
    ///
    /// Panics if the value is greater than 999.
    pub const fn new(value: u16) -> Self {
        if MAX_TEMPO_VALUE < value {
            panic!("Too large value.");
        } else {
            TempoValue(value)
        }
    }

    /// Returns the tempo value in BPM.
    pub fn value(self) -> u16 {
        self.0
    }

    /// Returns the tempo value as a u16.
    pub fn as_u16(self) -> u16 {
        self.0
    }

    /// Increases the tempo by 1 BPM.
    ///
    /// # Returns
    ///
    /// - `Ok(TempoValue)` - The increased tempo.
    /// - `Err(TempoError)` - If already at maximum tempo.
    pub fn up(self) -> Result<Self, TempoError> {
        let new_value = self.0 + 1;
        if MAX_TEMPO_VALUE < new_value {
            Err(TempoError::InvalidValue(new_value))
        } else {
            Ok(TempoValue::new(new_value))
        }
    }

    /// Decreases the tempo by 1 BPM.
    ///
    /// # Returns
    ///
    /// - `Ok(TempoValue)` - The decreased tempo.
    /// - `Err(TempoError)` - If already at minimum tempo.
    pub fn down(self) -> Result<Self, TempoError> {
        let cur = self.0;
        if cur <= MIN_TEMPO_VALUE {
            Err(TempoError::InvalidValue(MIN_TEMPO_VALUE))
        } else {
            let new_value = cur - 1;
            Ok(TempoValue::new(new_value))
        }
    }

    /// Creates a new tempo value, clamping to valid range.
    ///
    /// Values greater than 999 are clamped to 999.
    pub fn safe_new(value: u16) -> TempoValue {
        Self::new(if MAX_TEMPO_VALUE < value { MAX_TEMPO_VALUE } else { value })
    }
}

/// Tempo change at a specific tick position.
///
/// Represents a tempo marking in the score, specifying when
/// the tempo changes and what the new tempo is.
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Tempo {
    /// The tick position where this tempo change occurs.
    pub start_tick: u32,
    /// The new tempo value in BPM.
    pub value: TempoValue,
}

impl Tempo {
    /// Creates a new tempo change.
    ///
    /// # Arguments
    ///
    /// * `start_tick` - The tick position for this tempo change.
    /// * `value` - The tempo in BPM.
    pub fn new(start_tick: u32, value: u16) -> Self {
        Self {
            start_tick,
            value: TempoValue::new(value),
        }
    }

    /// Creates a new tempo with adjusted position and value (for dragging).
    ///
    /// # Arguments
    ///
    /// * `tick_delta` - Amount to adjust the tick position.
    /// * `tempo_delta` - Amount to adjust the tempo value.
    pub fn drag(&self, tick_delta: i32, tempo_delta: i32) -> Self {
        Self {
             start_tick: (self.start_tick as i64 + tick_delta as i64) as u32,
             value: TempoValue::safe_new((self.value.as_u16() as i32 + tempo_delta) as u16),
        }
    }

    /// Creates a new tempo with adjusted tick position.
    ///
    /// # Arguments
    ///
    /// * `tick_delta` - Amount to adjust the tick position.
    ///
    /// # Returns
    ///
    /// - `Ok(Tempo)` - The tempo with adjusted position.
    /// - `Err(TickError)` - If the resulting tick would be negative.
    pub fn with_tick_added(&self, tick_delta: i32) -> Result<Self, TickError> {
        let tick = self.start_tick as i64 + tick_delta as i64;
        if tick < 0 {
            Err(TickError::Minus)
        } else {
            Ok(
                Self {
                    start_tick: tick as u32,
                    ..*self
                }
            )
        }
    }

    pub fn up(&self) -> Result<Self, TempoError> {
        self.value.up().map(|tv| {
            Tempo {
                value: tv,
                ..*self
            }
        })
    }

    pub fn down(&self) -> Result<Self, TempoError> {
        self.value.down().map(|tv| {
            Tempo {
                value: tv,
                ..*self
            }
        })
    }

}

impl HaveBaseStartTick for Tempo {
    fn base_start_tick(&self) -> u32 {
        self.start_tick
    }
}

impl HaveStartTick for Tempo {
    fn start_tick(&self) -> u32 {
        self.start_tick
    }
}

#[cfg(test)]
mod tests {
    use crate::tempo::{Tempo, TempoValue};
    use serde_json::Value;
    use serde_json::json;

    #[test]
    fn can_deserialize_tempo() {
        let tempo: Tempo = serde_json::from_str(r#"
            {
                "start_tick": 123,
                "value": 234
            }"#).unwrap();
        assert_eq!(tempo, Tempo {
            start_tick: 123,
            value: TempoValue(234)
        });
    }

    #[test]
    fn can_serialize_tempo() {
        let json_str = serde_json::to_string(&Tempo {
            start_tick: 123,
            value: TempoValue(234)
        }).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({
                "start_tick": 123,
                "value": 234
            })
        );
    }
}
