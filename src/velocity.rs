#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from="Serialized")]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Velocity(u8);

impl Default for Velocity {
    fn default() -> Self {
        Self(64)
    }
}

pub const MIN_VALUE: u8 = 0;
pub const MAX_VALUE: u8 = 127;

pub const MIN: Velocity = Velocity(0);
pub const MAX: Velocity = Velocity(127);

#[derive(serde::Deserialize)]
struct Serialized(u8);

impl From<Serialized> for Velocity {
    fn from(s: Serialized) -> Self {
        Self::new(s.0)
    }
}

impl Velocity {
    #[inline]
    pub fn new(value: u8) -> Self {
        if MAX_VALUE < value { MAX } else { Velocity(value) }
    }

    #[inline]
    pub fn as_u8(self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for Velocity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
