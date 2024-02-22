use crate::play_iter::PlayIter;

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct PlayStartTick {
    pub tick: u32,
    pub iter: PlayIter
}
