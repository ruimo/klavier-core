#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Channel(u8);

impl Channel {
  pub fn new(value: u8) -> Self {
    if value < 16 {
      Self(value)
    } else {
      panic!("Invalid channel value(={}). Should be < 16.", value);
    }
  }

  pub fn as_u8(self) -> u8 {
    self.0
  }
}

impl Default for Channel {
    fn default() -> Self {
        Self(0)
    }
}