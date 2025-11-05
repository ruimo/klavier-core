/// MIDI velocity value (0-127).
///
/// Velocity represents how hard a note is played, affecting its volume
/// and sometimes its timbre. In MIDI, velocity ranges from 0 (silent)
/// to 127 (maximum).
///
/// # Examples
///
/// ```
/// # use klavier_core::velocity::Velocity;
/// let soft = Velocity::new(40);
/// let loud = Velocity::new(100);
/// assert_eq!(soft.as_u8(), 40);
/// assert_eq!(loud.as_u8(), 100);
/// ```
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(from="Serialized")]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Velocity(u8);

impl Default for Velocity {
    fn default() -> Self {
        Self(64)
    }
}

/// Minimum velocity value (0).
pub const MIN_VALUE: u8 = 0;

/// Maximum velocity value (127).
pub const MAX_VALUE: u8 = 127;

/// Minimum velocity constant.
pub const MIN: Velocity = Velocity(0);

/// Maximum velocity constant.
pub const MAX: Velocity = Velocity(127);

#[derive(serde::Deserialize)]
struct Serialized(u8);

impl From<Serialized> for Velocity {
    fn from(s: Serialized) -> Self {
        Self::new(s.0)
    }
}

impl Velocity {
    /// Creates a new velocity value.
    ///
    /// Values greater than 127 are clamped to 127.
    ///
    /// # Arguments
    ///
    /// * `value` - The velocity value (0-127, values > 127 are clamped).
    ///
    /// # Examples
    ///
    /// ```
    /// # use klavier_core::velocity::Velocity;
    /// let v1 = Velocity::new(64);
    /// assert_eq!(v1.as_u8(), 64);
    ///
    /// let v2 = Velocity::new(200); // Clamped to 127
    /// assert_eq!(v2.as_u8(), 127);
    /// ```
    #[inline]
    pub fn new(value: u8) -> Self {
        if MAX_VALUE < value { MAX } else { Velocity(value) }
    }

    /// Returns the velocity value as a u8.
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
