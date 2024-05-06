use crate::{play_iter::PlayIter, repeat::{AccumTick, Chunk}};

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct PlayStartTick {
    pub tick: u32,
    pub iter: PlayIter
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ToAccumTickError {
    CannotFind {
        specified_iter: PlayIter, max_iter: u8,
    },
}

impl PlayStartTick {
    pub fn new(tick: u32, iter: u8) -> Self {
        Self {
            tick, iter: PlayIter::new(iter)
        }
    }

    pub fn to_accum_tick(&self, chunks: &[(AccumTick, Chunk)]) -> Result<AccumTick, ToAccumTickError> {
        let mut cur_iter: u8 = 1;

        for (accum_tick, chunk) in chunks {
            if chunk.contains(self.tick) {
                if cur_iter == self.iter.iter() {
                    return Ok(accum_tick + self.tick - chunk.start_tick());
                }
                cur_iter += 1;
            }
        }

        Err(ToAccumTickError::CannotFind { specified_iter: self.iter, max_iter: cur_iter - 1 })
    }
}
