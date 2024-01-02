use crate::channel::Channel;
use super::{note::TickError, have_start_tick::{HaveBaseStartTick, HaveStartTick}, velocity::Velocity};

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CtrlChg {
    pub start_tick: u32,
    pub velocity: Velocity,
    pub channel: Channel,
}

impl CtrlChg {
    pub fn new(start_tick: u32, velocity: Velocity, channel: Channel) -> Self {
        Self { start_tick, velocity, channel }
    }
    
    pub fn drag(&self, tick_delta: i32) -> Self {
        Self {
             start_tick: (self.start_tick as i64 + tick_delta as i64) as u32,
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
}

impl HaveBaseStartTick for CtrlChg {
    fn base_start_tick(&self) -> u32 {
        self.start_tick
    }
}

impl HaveStartTick for CtrlChg {
    fn start_tick(&self) -> u32 {
        self.start_tick
    }
}

#[cfg(test)]
mod tests {
    use crate::channel::Channel;
    use crate::ctrl_chg::CtrlChg;
    use crate::velocity::Velocity;
    use serde_json::Value;
    use serde_json::json;

    #[test]
    fn can_deserialize_ctrl_chg() {
        let ctrl_chg: CtrlChg = serde_json::from_str(r#"
            {
                "start_tick": 123,
                "velocity": 64,
                "channel": 0
            }"#).unwrap();
        assert_eq!(ctrl_chg, CtrlChg {
            start_tick: 123,
            velocity: Velocity::new(64),
            channel: Channel::default(),
        });
    }

    #[test]
    fn can_serialize_ctrl_chg() {
        let json_str = serde_json::to_string(&CtrlChg {
            start_tick: 123,
            velocity: Velocity::new(64),
            channel: Channel::new(1),
        }).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({
                "start_tick": 123,
                "velocity": 64,
                "channel": 1
            })
        );
    }
}