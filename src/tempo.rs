use super::{note::TickError, have_start_tick::{HaveBaseStartTick, HaveStartTick}};

pub const MIN_TEMPO_VALUE: u16 = 1;
pub const MAX_TEMPO_VALUE: u16 = 999;

pub const MIN_TEMPO: TempoValue = TempoValue(MIN_TEMPO_VALUE);
pub const MAX_TEMPO: TempoValue = TempoValue(MAX_TEMPO_VALUE);

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

pub enum TempoError {
    InvalidValue(u16),
}

impl TempoValue {
    pub const fn new(value: u16) -> Self {
        if MAX_TEMPO_VALUE < value {
            panic!("Too large value.");
        } else {
            TempoValue(value)
        }
    }

    pub fn value(self) -> u16 {
        self.0
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }

    pub fn up(self) -> Result<Self, TempoError> {
        let new_value = self.0 + 1;
        if MAX_TEMPO_VALUE < new_value {
            Err(TempoError::InvalidValue(new_value))
        } else {
            Ok(TempoValue::new(new_value))
        }
    }

    pub fn down(self) -> Result<Self, TempoError> {
        let cur = self.0;
        if cur <= MIN_TEMPO_VALUE {
            Err(TempoError::InvalidValue(MIN_TEMPO_VALUE))
        } else {
            let new_value = cur - 1;
            Ok(TempoValue::new(new_value))
        }
    }

    pub fn safe_new(value: u16) -> TempoValue {
        Self::new(if MAX_TEMPO_VALUE < value { MAX_TEMPO_VALUE } else { value })
    }

}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tempo {
    pub start_tick: u32,
    pub value: TempoValue,
}

impl Default for Tempo {
    fn default() -> Self {
        Self {
            start_tick: 0,
            value: Default::default()
        }
    }
}

impl Tempo {
    pub fn new(start_tick: u32, value: u16) -> Self {
        Self {
            start_tick,
            value: TempoValue::new(value),
        }
    }

    pub fn drag(&self, tick_delta: i32, tempo_delta: i32) -> Self {
        Self {
             start_tick: (self.start_tick as i64 + tick_delta as i64) as u32,
             value: TempoValue::safe_new((self.value.as_u16() as i32 + tempo_delta) as u16),
             ..*self
        }
    }

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
