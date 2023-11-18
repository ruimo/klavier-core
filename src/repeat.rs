use std::slice::Iter;

use crate::{bar::{Bar, VarIndex, Repeat}, rhythm::Rhythm, have_start_tick::HaveBaseStartTick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
  start_tick: u32,
  end_tick: u32,
  offset: u32,
}

impl Chunk {
  fn new(start_tick: u32, end_tick: u32, offset: u32) -> Self {
    Self {
      start_tick, end_tick, offset
    }
  }

  fn with_offset(&self, offset: u32) -> Self {
    Self {
      start_tick: self.start_tick,
      end_tick: self.end_tick,
      offset,
    }
  }
}

trait Region: std::fmt::Debug {
  fn to_chunk(&self, offset: &mut u32) -> Vec<Chunk>;
}

// D.C. Region is not a SimpleRegion.
trait SimpleRegion: Region {
}

#[derive(Debug, PartialEq, Eq)]
struct NullRegion;

impl Region for NullRegion {
  fn to_chunk(&self, offset: &mut u32) -> Vec<Chunk> {
    vec![]
  }
}

impl SimpleRegion for NullRegion {
}

#[derive(Debug, PartialEq, Eq)]
pub struct SequenceRegion {
  start_tick: u32,
  end_tick: u32,
}

impl SequenceRegion {
  fn tick_len(&self) -> u32 {
    self.end_tick - self.start_tick
  }
}

impl Region for SequenceRegion {
  fn to_chunk(&self, offset: &mut u32) -> Vec<Chunk> {
    vec![Chunk::new(self.start_tick, self.end_tick, *offset)]
  }
}

impl SimpleRegion for SequenceRegion {
}

#[derive(Debug, PartialEq, Eq)]
pub struct RepeatRegion {
  region: SequenceRegion,
}

impl Region for RepeatRegion {
  fn to_chunk(&self, offset: &mut u32) -> Vec<Chunk> {
    let chunk = self.region.to_chunk(offset)[0];
    *offset += self.region.tick_len();
    vec![chunk, chunk.with_offset(*offset)]
  }
}

impl SimpleRegion for RepeatRegion {
}

#[derive(Debug)]
pub struct VariationRegion {
  common: SequenceRegion,
  variations: Vec<SequenceRegion>,
}

impl Region for VariationRegion {
  fn to_chunk(&self, offset: &mut u32) -> Vec<Chunk> {
    let mut ret: Vec<Chunk> = Vec::with_capacity(self.variations.len() * 2);
    let mut prev_var_tick_len: u32 = 0;
    let mut total_tick = self.common.tick_len();

    for v in self.variations.iter() {
      let mut tick_len = 0;
      ret.extend(self.common.to_chunk(offset));
      tick_len += self.common.tick_len();

      let mut var_offset = *offset - prev_var_tick_len;
      ret.extend(v.to_chunk(&mut var_offset));
      tick_len += v.tick_len();
      prev_var_tick_len = v.tick_len();

      *offset += tick_len;
      total_tick += v.tick_len();
    }

    *offset -= total_tick;
    ret
  }
}

impl SimpleRegion for VariationRegion {
}

#[derive(Debug)]
pub struct CompoundRegion {
  regions: Vec<Box<dyn SimpleRegion>>,
}

impl Region for CompoundRegion {
  fn to_chunk(&self, offset: &mut u32) -> Vec<Chunk> {
    let mut buf: Vec<Chunk> = vec![];
    for r in self.regions.iter() {
      buf.extend(r.to_chunk(offset));
    }
    buf
  }
}

impl SimpleRegion for CompoundRegion {
}

#[derive(Debug)]
pub struct DcRegion {
  body: Box<dyn SimpleRegion>,
  fine_tick_pos: u32,
}

impl Region for DcRegion {
  fn to_chunk(&self, offset: &mut u32) -> Vec<Chunk> {
    vec![]
  }
}

#[derive(Debug)]
enum RenderRegionState {
  Init,
  Idle,
  Seq { start_tick: u32 },
  RepeatStart { start_tick: u32 },
  Variation {
    start_tick: u32,
    region_start_ticks: Vec<u32>,
  },
}

#[derive(Debug, PartialEq, Eq)]
pub enum RenderRegionError {
  OrphanRepeatEnd { tick_pos: u32 },
  DuplicatedRepeatStart { tick_pos: u32 },
  NoRepeatEnd { tick_pos: u32 },
  InvalidRegionIndex { tick_pos: u32, actual: VarIndex, expected: VarIndex },
  RepeatInVariation { tick_pos: u32 },
  VariationNotClosed { tick_pos: u32 },
  DuplicatedFine { prev_tick_pos: u32, next_tick_pos: u32 },
  NoFine { dc_tick_pos: u32 },
  RepeatOrVariationOnDc { tick_pos: u32 },
}

fn render_region(start_rhythm: Rhythm, bars: Iter<Bar>) -> Result<Box<dyn Region>, RenderRegionError> {
  fn create_variation(start_tick: u32, region_start_ticks: Vec<u32>, end_tick: u32) -> Box<dyn SimpleRegion> {
    let mut variations: Vec<SequenceRegion> = vec![];
    let mut iter = region_start_ticks.iter();
    let mut tick = *iter.next().unwrap();
    for t in iter {
      variations.push(SequenceRegion { start_tick: tick, end_tick: *t });
      tick = *t;
    }

    variations.push(SequenceRegion { start_tick: tick, end_tick });

    Box::new(VariationRegion {
      common: SequenceRegion { start_tick, end_tick: region_start_ticks[0] }, variations
    })
  }

  fn create_dc(regions: Vec<Box<dyn SimpleRegion>>, fine_tick_pos: u32) -> DcRegion {
    let len = regions.len();
    if len == 0 {
      panic!("Logic error.");
    } else if len == 1 {
      DcRegion {
        body: regions.into_iter().next().unwrap(),
        fine_tick_pos
      }
    } else {
      DcRegion {
        body: Box::new(CompoundRegion { regions }),
        fine_tick_pos
      }
    }
  }

  let mut regions: Vec<Box<dyn SimpleRegion>> = vec![];
  let mut state = RenderRegionState::Init;
  let mut is_auftakt = false;
  let mut fine_tick_pos: Option<u32> = None;

  for bar in bars {
    if bar.repeats.contains(Repeat::Fine) {
      match fine_tick_pos {
        Some(prev) => {
          return Err(RenderRegionError::DuplicatedFine { prev_tick_pos: prev, next_tick_pos: bar.base_start_tick() });
        }
        None => fine_tick_pos = Some(bar.base_start_tick()),
      }
    }

    state = match &state {
      RenderRegionState::Init => {
        is_auftakt = bar.start_tick != start_rhythm.tick_len();
  
        if bar.repeats.contains(Repeat::Dc) {
          return Err(RenderRegionError::NoFine { dc_tick_pos: bar.base_start_tick() });
        }

        if bar.repeats.contains(Repeat::Start) && bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { start_tick: 0, end_tick: bar.base_start_tick() }}));
          RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
        } else if bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { start_tick: 0, end_tick: bar.base_start_tick() }}));
          RenderRegionState::Seq { start_tick: bar.base_start_tick() }
        } else if bar.repeats.contains(Repeat::Start) {
          regions.push(Box::new(SequenceRegion { start_tick: 0, end_tick: bar.base_start_tick() }));
          RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
        } else if let Some(idx) = bar.repeats.region_index() {
          if idx != VarIndex::VI1 {
            return Err(RenderRegionError::InvalidRegionIndex { tick_pos: bar.base_start_tick(), actual: idx, expected: VarIndex::VI1 });
          }
          RenderRegionState::Variation { start_tick: 0, region_start_ticks: vec![bar.base_start_tick()] }
        } else {
          RenderRegionState::Idle
        }
      },
      RenderRegionState::Idle => {
        if bar.repeats.contains(Repeat::Dc) {
          if bar.repeats.contains(Repeat::Start) || bar.repeats.has_end_or_region() {
            return Err(RenderRegionError::RepeatOrVariationOnDc { tick_pos: bar.base_start_tick() });
          }

          match fine_tick_pos {
            Some(fine_tick_pos) => {
              return Ok(
                Box::new(
                  create_dc(vec![Box::new(SequenceRegion { start_tick: 0, end_tick: bar.base_start_tick() })], fine_tick_pos)
                )
              );
            }
            None => {
              return Err(RenderRegionError::NoFine { dc_tick_pos: bar.base_start_tick() });
            }
          }
        }

        if bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { start_tick: 0, end_tick: bar.base_start_tick() }}));
          RenderRegionState::Seq { start_tick: bar.base_start_tick() }
        } else if let Some(idx) = bar.repeats.region_index() {
          if idx != VarIndex::VI1 {
            return Err(RenderRegionError::InvalidRegionIndex { tick_pos: bar.base_start_tick(), actual: idx, expected: VarIndex::VI1 });
          }
          RenderRegionState::Variation { start_tick: 0, region_start_ticks: vec![bar.base_start_tick()] }
        } else {
          state
        }
      },
      RenderRegionState::Seq { start_tick } => {
        if bar.repeats.contains(Repeat::End) {
          return Err(RenderRegionError::OrphanRepeatEnd{ tick_pos: bar.base_start_tick() });
        } else {
          state
        }
      },
      RenderRegionState::RepeatStart { start_tick } => {
        if bar.repeats.contains(Repeat::Start) && bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { start_tick: *start_tick, end_tick: bar.base_start_tick() }}));
          RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
        } else if bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { start_tick: *start_tick, end_tick: bar.base_start_tick() }}));
          RenderRegionState::Seq { start_tick: bar.base_start_tick() }
        } else if bar.repeats.contains(Repeat::Start) {
          return Err(RenderRegionError::DuplicatedRepeatStart { tick_pos: bar.base_start_tick() });
        } else {
          state
        }
      },
      RenderRegionState::Variation { start_tick, region_start_ticks } => {
        if bar.repeats.contains(Repeat::End) {
          return Err(RenderRegionError::RepeatInVariation { tick_pos: bar.base_start_tick() })
        } else if let Some(ri) = bar.repeats.region_index() {
          let current_idx = region_start_ticks.len() as u8;
          let idx = ri.value();
          if idx == current_idx {
            state
          } else if idx == current_idx + 1 {
            let mut rst = region_start_ticks.clone();
            rst.push(bar.base_start_tick());
            RenderRegionState::Variation { start_tick: *start_tick, region_start_ticks: rst }
          } else {
            return Err(RenderRegionError::InvalidRegionIndex {
              tick_pos: bar.base_start_tick(), actual: ri, expected: VarIndex::from_value(current_idx + 1).unwrap()
            });
          }
        } else {
          regions.push(create_variation(*start_tick, region_start_ticks.clone(), bar.base_start_tick()));

          if bar.repeats.contains(Repeat::Start) {
            RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
          } else {
            RenderRegionState::Seq { start_tick: bar.base_start_tick() }
          }
        }
      },
    }
  }

  match state {
    RenderRegionState::Init => Ok(Box::new(NullRegion)),
    RenderRegionState::Idle => Ok(Box::new(SequenceRegion { start_tick: 0, end_tick: u32::MAX })),
    RenderRegionState::Seq { start_tick } => if start_tick == 0 {
      Ok(Box::new(SequenceRegion { start_tick: 0, end_tick: u32::MAX }))
    } else {
      regions.push(Box::new(SequenceRegion { start_tick, end_tick: u32::MAX }));
      Ok(Box::new(CompoundRegion { regions }))
    }
    RenderRegionState::RepeatStart { start_tick } => Err(RenderRegionError::NoRepeatEnd { tick_pos: start_tick }),
    RenderRegionState::Variation { start_tick: _, region_start_ticks } => {
      Err(RenderRegionError::VariationNotClosed { tick_pos: *region_start_ticks.last().unwrap() })
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{repeat::{render_region, Chunk, RenderRegionError}, rhythm::Rhythm, bar::{Bar, RepeatSet, Repeat}};
  use crate::repeat_set;

  #[test]
  fn empty() {
    let bars: Vec<Bar> = vec![];
    let mut offset: u32 = 0;
    assert_eq!(render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunk(&mut offset).len(), 0);
  }

  // 0    100
  //   A  |
  #[test]
  fn single_bar() {
    let bar = Bar::new(100, None, None, crate::repeat_set!());
    let bars = vec![bar];
    let mut offset: u32 = 0;
    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunk(&mut offset);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk::new(0, u32::MAX, 0));
  }

  // 0    100
  //   A  :|  B
  //
  // 0    100   200
  //   A  |  A  |  B
  #[test]
  fn default_start_repeat() {
    let bars = vec![
      Bar::new(100, None, None, repeat_set!(Repeat::End))
    ];
    let mut offset: u32 = 0;
    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunk(&mut offset);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], Chunk::new(0, 100, 0));
    assert_eq!(chunks[1], Chunk::new(0, 100, 100));
    assert_eq!(chunks[2], Chunk::new(100, u32::MAX, 100));
  }

  // 0    100   200
  //   A  :|  B  :|
  //
  // Repeat end at 200 is invalid.
  #[test]
  fn invalid_repeat_end() {
    let bars = vec![
      Bar::new(100, None, None, repeat_set!(Repeat::End)),
      Bar::new(200, None, None, repeat_set!(Repeat::End)),
    ];
    let err = render_region(Rhythm::new(4, 4), bars.iter()).err().unwrap();
    assert_eq!(err, RenderRegionError::OrphanRepeatEnd { tick_pos: 200 });
  }

  // 0    100     200
  //   A  |:  B  :|  C
  //
  // 0    100   200   300
  //   A  |  B  |  B  |  C
  #[test]
  fn single_repeat() {
    let bars = vec![
      Bar::new(100, None, None, repeat_set!(Repeat::Start)),
      Bar::new(200, None, None, repeat_set!(Repeat::End)),
    ];
    let mut offset: u32 = 0;

    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunk(&mut offset);
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0], Chunk::new(0, 100, 0));
    assert_eq!(chunks[1], Chunk::new(100, 200, 0));
    assert_eq!(chunks[2], Chunk::new(100, 200, 100));
    assert_eq!(chunks[3], Chunk::new(200, u32::MAX, 100));
  }

  // 0    50          200
  //   A  |:    B    :|  C
  //
  // 0    50          200         350
  //   A  |     B     |     B     |  C
  #[test]
  fn single_repeat2() {
    let bars = vec![
      Bar::new(50, None, None, repeat_set!(Repeat::Start)),
      Bar::new(200, None, None, repeat_set!(Repeat::End)),
    ];
    let mut offset: u32 = 0;

    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunk(&mut offset);
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0], Chunk::new(0, 50, 0));
    assert_eq!(chunks[1], Chunk::new(50, 200, 0));
    assert_eq!(chunks[2], Chunk::new(50, 200, 150));
    assert_eq!(chunks[3], Chunk::new(200, u32::MAX, 150));
    assert_eq!(offset, 150);
  }

  // 0     50       100       150
  //   A   |[1]  B   |[2]  C   |    D
  //
  // 0     50       100      150    200
  //   A   |     B   |   A    |   C  |  D
  #[test]
  fn simple_variation() {
    let bars = vec![
      Bar::new(50, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(100, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(150, None, None, repeat_set!()),
    ];
    let mut offset: u32 = 0;

    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunk(&mut offset);
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 50, 0));
    assert_eq!(chunks[1], Chunk::new(50, 100, 0));
    assert_eq!(chunks[2], Chunk::new(0, 50, 100));
    assert_eq!(chunks[3], Chunk::new(100, 150, 50));
    assert_eq!(chunks[4], Chunk::new(150, u32::MAX, 50));
    assert_eq!(offset, 50);
  }

  // 0    100        200        350       500        650
  //   A   |[1]  B   |[1]   C   |[2]  D   |[2]   E   |    F
  //
  // 0    100        200        350    450      600     750
  //   A   |     B   |      C   |   A  |    D   |   E   |   F
  #[test]
  fn simple_variation2() {
    let bars = vec![
      Bar::new(100, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(200, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(350, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(500, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(650, None, None, repeat_set!()),
    ];
    let mut offset: u32 = 0;

    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunk(&mut offset);
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 100, 0));
    assert_eq!(chunks[1], Chunk::new(100, 350, 0));
    assert_eq!(chunks[2], Chunk::new(0, 100, 350));
    assert_eq!(chunks[3], Chunk::new(350, 650, 100));
    assert_eq!(chunks[4], Chunk::new(650, u32::MAX, 100));
    assert_eq!(offset, 100);
  }

  // Non auftakt.
  // 0    480         960        1440
  //   A   |Fine  B   |     C    |DC
  //
  // 0    480         960        1440   1920
  //   A   |     B    |      C   |   A  |
  #[test]
  fn simple_dc() {
    let bars = vec![
      Bar::new(480, None, None, repeat_set!(Repeat::Fine)),
      Bar::new(960, None, None, repeat_set!()),
      Bar::new(1440, None, None, repeat_set!(Repeat::Dc)),
    ];
    let mut offset: u32 = 0;

    let chunks = render_region(Rhythm::new(2, 4), bars.iter()).unwrap().to_chunk(&mut offset);
    assert_eq!(chunks.len(), 1);
  }



  // 0    100         200       350        500        650       800     900
  //   A   |[1]  B   |[1]   C   |[2]   D   |[2]   E   |Fine F   |   G   |DC
  //
  // 0    100         200       350    450     600     750     900    1000   1100    1250    1400
  //   A   |     B    |     C   |   A  |   D   |   E   |   F   |   G   |  A  |   D   |   E   |

  // 0 50    100         200   300 
  // A |: B :|:[Fine] C :|: D :|[D.C.]
  // 
  // 0  50 100 150 250 350 450 550 600 650
  // A  B  B   C   C   D   D   A   B

  // 0 50   100 150  200  250  300 350  400 450
  // A |: B | C |1 D |2 E |: F | G |1 H |2 I| J
  //
  // 0 50  100 150 200 250 300 350 400 450 500 550 600 650
  // A | B | C | D | B | C | E | F | G | H | F | G | I | J

  // 0 50   100 150  200  250  300  350  400 450 500 550 600
  // A |: B | C |1 D |2 E |  F |: G | H  |1 I|1 J |2 K| L |
  //
  // 0 50  100 150 200 250 300 350 400 450 500 550 600 650 700 750
  // A | B | C | D | B | C | E | F | G | H | I | J | G | H | K | L

  // 0 20   100 150  200  250  300  350  400 450     500 550 600  650 700 750
  // A |: B | C |1 D |2 E |  F |: G | H  |1 I|2 J Fine| K |: L| M | N :| O |DC
  //
  // 0 20  100 150 200 280 330 380 430 480 530 580 630 680 730 780 830 880 930 980 
  // A | B | C | D | B | C | E | F | G | H | I | G | H | J | K | L | M | N | L | M 
  //
  // 1030 1080 1130 1210 1260 1310 1360 1410 1460 1510
  // |  N | O  | B  | C  | E  | F  | G  | H  | J  |

  // 0    50          200      300
  //   A  |     B    :|:  C    :|
  //
  // 0    50          200  250         400    500   600
  //   A  |     B     |  A  |     B     |  C   |  C  |

  // 0    50          200          300
  //   A  |     B    :|:Fine  C    :| D.C
  //
  // 0    50          200  250         400    500   600    650    800
  //   A  |     B     |  A  |     B     |  C   |  C  |   A  |  B  |

  // 0 50    100         200   300      400
  // A |: B :|:[Fine] C :|: D :|[D.C.] E |
  // 
  // 0  50 100 150 250 350 450 550 600 650 750
  // A  B  B   C   C   D   D   A   B   E


}