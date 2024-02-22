#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayIter {
  iter: u8,
}

pub const MAX_ITER: u8 = 5;

impl PlayIter {
  pub fn new(iter: u8) -> Self {
    let iter = if iter < 1 { 1 } else { iter };
    let iter = if MAX_ITER < iter { MAX_ITER } else { iter };

    Self {
      iter
    }
  }

  pub fn iter(self) -> u8 { self.iter }
  pub fn set_iter(&mut self, current_iter: u8) -> bool {
    if 0 < current_iter && current_iter <= MAX_ITER {
      self.iter = current_iter;
      true
    } else  {
      false
    }
  }
}

impl Default for PlayIter {
    fn default() -> Self {
        Self { iter: 1 }
    }
}