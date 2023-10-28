use crate::{bar::Bar, rhythm::Rhythm};

#[derive(Debug)]
struct Chunk {
  start_tick: u32,
  end_tick: u32,
  offset: u32,
}

#[derive(Debug)]
pub enum Region<'a> {
  Sequence {
    bars: Vec<&'a Bar>,
    start_tick: u32,
    end_tick: u32,
  },
  Repeat(Vec<Region<'a>>),
  Variation {
    until: &'a Region<'a>,
    variations: Vec<Region<'a>>,
  },
  Compound(Vec<Region<'a>>),
  Null,
}

pub fn render_region(start_rhythm: Rhythm, bars: &Vec<Bar>) -> Region {
  Region::Null
}