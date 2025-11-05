/// MIDI channel number (0-15).
///
/// MIDI supports 16 channels, numbered 0 through 15. Each channel can
/// play different instruments or sounds independently.
///
/// # Examples
///
/// ```
/// # use klavier_core::channel::Channel;
/// let channel = Channel::new(0); // Channel 1 (0-indexed)
/// assert_eq!(channel.as_u8(), 0);
/// ```
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Channel(u8);

impl Channel {
  /// Creates a new MIDI channel.
  ///
  /// # Arguments
  ///
  /// * `value` - The channel number (must be 0-15).
  ///
  /// # Panics
  ///
  /// Panics if the value is >= 16.
  ///
  /// # Examples
  ///
  /// ```
  /// # use klavier_core::channel::Channel;
  /// let channel = Channel::new(5);
  /// assert_eq!(channel.as_u8(), 5);
  /// ```
  pub fn new(value: u8) -> Self {
    if value < 16 {
      Self(value)
    } else {
      panic!("Invalid channel value(={}). Should be < 16.", value);
    }
  }

  /// Returns the channel number as a u8.
  pub fn as_u8(self) -> u8 {
    self.0
  }
}
