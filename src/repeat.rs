use std::slice::Iter;

use crate::{bar::{Bar, VarIndex, Repeat}, rhythm::Rhythm, have_start_tick::HaveBaseStartTick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
  start_tick: u32,
  end_tick: u32,
}

impl Chunk {
  fn new(start_tick: u32, end_tick: u32) -> Self {
    Self {
      start_tick, end_tick
    }
  }
}

trait Region: std::fmt::Debug {
  fn to_chunks(&self) -> Vec<Chunk>;
}

// SimpleRegion can be stored in a compound region.
// D.C. Region is not a SimpleRegion.
trait SimpleRegion: Region {
  fn render_chunks(&self, from_segno: Option<u32>, to_fine: Option<u32>) -> Vec<Chunk>;
}

#[derive(Debug, PartialEq, Eq)]
struct NullRegion;

impl Region for NullRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    vec![]
  }
}

impl SimpleRegion for NullRegion {
  fn render_chunks(&self, from_segno: Option<u32>, to_fine: Option<u32>) -> Vec<Chunk> {
    vec![]
  }
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
  fn to_chunks(&self) -> Vec<Chunk> {
    self.render_chunks(None, None)
  }
}

impl SimpleRegion for SequenceRegion {
  fn render_chunks(&self, from_segno: Option<u32>, to_fine: Option<u32>) -> Vec<Chunk> {
    let start_pos: Option<u32> = match from_segno {
      None =>
        Some(self.start_tick),
      Some(segno_pos) =>
        if segno_pos <= self.start_tick {
          Some(self.start_tick)
        } else if segno_pos < self.end_tick {
          Some(segno_pos)
        } else {
          None
        }
    };
    if start_pos.is_none() {
      return vec![]
    }

    let end_pos: Option<u32> = match to_fine {
      None =>
        Some(self.end_tick),
      Some(fine_pos) =>
        if fine_pos <= self.start_tick {
          None
        } else if fine_pos < self.end_tick {
          Some(fine_pos)
        } else {
          Some(self.end_tick)
        }
    };
    if end_pos.is_none() {
      return vec![]
    }

    vec![Chunk::new(start_pos.unwrap(), end_pos.unwrap())]
  }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RepeatRegion {
  region: SequenceRegion,
}

impl Region for RepeatRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    self.render_chunks(None, None)
  }
}

impl SimpleRegion for RepeatRegion {
  fn render_chunks(&self, from_segno: Option<u32>, to_fine: Option<u32>) -> Vec<Chunk> {
    let chunk = self.region.render_chunks(from_segno, to_fine);
    let mut ret: Vec<Chunk> = Vec::with_capacity(chunk.len());
    ret.extend(chunk.clone());
    if to_fine == None {
      ret.extend(chunk);
    }

    ret
  }
}

#[derive(Debug)]
pub struct VariationRegion {
  common: SequenceRegion,
  variations: Vec<SequenceRegion>,
}

impl Region for VariationRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    self.render_chunks(None, None)
  }
}

impl SimpleRegion for VariationRegion {
  fn render_chunks(&self, from_segno: Option<u32>, to_fine: Option<u32>) -> Vec<Chunk> {
    let mut ret: Vec<Chunk> = Vec::with_capacity(self.variations.len() * 2);

    match to_fine {
      None => {
        for v in self.variations.iter() {
          ret.extend(self.common.render_chunks(from_segno, to_fine));
          ret.extend(v.render_chunks(from_segno, to_fine));
        }
      }
      Some(_) => {
        ret.extend(self.common.render_chunks(from_segno, to_fine));
        for v in self.variations.last().iter() {
          ret.extend(v.render_chunks(from_segno, to_fine));
        }
      }
    }
    ret
  }
}

#[derive(Debug)]
pub struct CompoundRegion {
  regions: Vec<Box<dyn SimpleRegion>>,
}

impl Region for CompoundRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    self.render_chunks(None, None)
  }
}

impl SimpleRegion for CompoundRegion {
  fn render_chunks(&self, from_segno: Option<u32>, to_fine: Option<u32>) -> Vec<Chunk> {
    let mut buf: Vec<Chunk> = vec![];
    for r in self.regions.iter() {
      buf.extend(r.render_chunks(from_segno, to_fine));
    }
    buf
  }
}

#[derive(Debug)]
pub struct DcRegion {
  body: Box<dyn SimpleRegion>,
  fine_tick_pos: u32,
}

impl Region for DcRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    let mut ret = vec![];
    ret.extend(self.body.render_chunks(None, None));
    ret.extend(self.body.render_chunks(None, Some(self.fine_tick_pos)));

    ret
  }
}

#[derive(Debug)]
pub struct DsRegion {
  body: Box<dyn SimpleRegion>,
  segno_tick_pos: u32, 
  fine_tick_pos: u32,
}

impl Region for DsRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    let mut ret = vec![];
    ret.extend(self.body.render_chunks(None, None));
    ret.extend(self.body.render_chunks(Some(self.segno_tick_pos), Some(self.fine_tick_pos)));

    ret
  }
}


#[derive(Debug)]
enum RenderRegionState {
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
  DuplicatedFine { tick_pos: [u32; 2] },
  NoFine { dc_or_ds_tick_pos: u32 },
  RepeatOrVariationOnDc { tick_pos: u32 },
  RepeatOrVariationOnDs { tick_pos: u32 },
  DuplicatedSegno { tick_pos: [u32; 2] },
  FineNotAfterSegno { segno_pos: u32, fine_pos: u32 },
  SegnoAndDcFound { segno_pos: u32, dc_pos: u32 },
  NoSegnoForDs { ds_pos: u32 },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum FineSegno {
  Nothing,
  Fine(u32),
  Segno(u32),
  Both { fine_pos: u32, segno_pos: u32 },
}

impl FineSegno {
  fn on_bar(self, bar: &Bar) -> Result<Self, RenderRegionError> {
    let repeats = bar.repeats;
    let tick = bar.base_start_tick();
    if repeats.contains(Repeat::Segno) && repeats.contains(Repeat::Fine) {
      return Err(RenderRegionError::FineNotAfterSegno { segno_pos: tick, fine_pos: tick })
    }

    match self {
      FineSegno::Nothing =>
        if repeats.contains(Repeat::Segno) {
          Ok(FineSegno::Segno(tick))
        } else if repeats.contains(Repeat::Fine) {
          Ok(FineSegno::Fine(tick))
        } else {
          Ok(self)
        }
      FineSegno::Fine(fine_pos) =>
        if repeats.contains(Repeat::Segno) {
          if fine_pos <= tick {
            Err(RenderRegionError::FineNotAfterSegno { segno_pos: tick, fine_pos })
          } else {
            Ok(FineSegno::Both { fine_pos, segno_pos: tick })
          }
        } else if repeats.contains(Repeat::Fine) {
          Err(RenderRegionError::DuplicatedFine { tick_pos: [fine_pos, tick] })
        } else {
          Ok(self)
        }
      FineSegno::Segno(segno_pos) =>
        if repeats.contains(Repeat::Segno) {
          Err(RenderRegionError::DuplicatedSegno { tick_pos: [segno_pos, tick] })
        } else if repeats.contains(Repeat::Fine) {
          if tick <= segno_pos {
            Err(RenderRegionError::FineNotAfterSegno { segno_pos, fine_pos: tick })
          } else {
            Ok(FineSegno::Both { fine_pos: tick, segno_pos })
          }
        } else {
          Ok(self)
        }
      FineSegno::Both { fine_pos, segno_pos } =>
        if repeats.contains(Repeat::Segno) {
          Err(RenderRegionError::DuplicatedSegno { tick_pos: [segno_pos, tick] })
        } else if repeats.contains(Repeat::Fine) {
          Err(RenderRegionError::DuplicatedFine { tick_pos: [fine_pos, tick] })
        } else {
          Ok(self)
        }
    }
  }
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

  fn create_ds(regions: Vec<Box<dyn SimpleRegion>>, segno_tick_pos: u32, fine_tick_pos: u32) -> DsRegion {
    let len = regions.len();
    if len == 0 {
      panic!("Logic error.");
    } else if len == 1 {
      DsRegion {
        body: regions.into_iter().next().unwrap(),
        segno_tick_pos, fine_tick_pos
      }
    } else {
      DsRegion {
        body: Box::new(CompoundRegion { regions }),
        segno_tick_pos, fine_tick_pos
      }
    }
  }

  let mut regions: Vec<Box<dyn SimpleRegion>> = vec![];
  let mut state = RenderRegionState::Idle;
  let mut is_auftakt: Option<bool> = None;
  let mut fine_segno: FineSegno = FineSegno::Nothing;

  for bar in bars {
    fn on_dc(bar: &Bar, regions: Vec<Box<dyn SimpleRegion>>, fine_segno: FineSegno) -> Result<Box<dyn Region>, RenderRegionError> {
      if bar.repeats.contains(Repeat::Start) || bar.repeats.region_index() != None {
        return Err(RenderRegionError::RepeatOrVariationOnDc { tick_pos: bar.base_start_tick() });
      }

      match fine_segno {
        FineSegno::Nothing =>
          Err(RenderRegionError::NoFine { dc_or_ds_tick_pos: bar.base_start_tick() }),
        FineSegno::Segno(segno_pos) =>
          Err(RenderRegionError::SegnoAndDcFound { segno_pos, dc_pos: bar.base_start_tick() }),
        FineSegno::Both { fine_pos, segno_pos } =>
          Err(RenderRegionError::SegnoAndDcFound { segno_pos, dc_pos: bar.base_start_tick() }),
        FineSegno::Fine(fine_pos) => Ok(
          if regions.is_empty() {
            Box::new(
              create_dc(vec![Box::new(SequenceRegion { start_tick: 0, end_tick: bar.base_start_tick() })], fine_pos)
            )
          } else {
            Box::new(create_dc(regions, fine_pos))
          }
        )
      }
    }

    fn on_ds(bar: &Bar, regions: Vec<Box<dyn SimpleRegion>>, fine_segno: FineSegno) -> Result<Box<dyn Region>, RenderRegionError> {
      if bar.repeats.contains(Repeat::Start) || bar.repeats.region_index() != None {
        return Err(RenderRegionError::RepeatOrVariationOnDs { tick_pos: bar.base_start_tick() });
      }

      match fine_segno {
        FineSegno::Nothing =>
          Err(RenderRegionError::NoFine { dc_or_ds_tick_pos: bar.base_start_tick() }),
        FineSegno::Fine(_) => 
          Err(RenderRegionError::NoSegnoForDs { ds_pos: bar.base_start_tick() }),
        FineSegno::Segno(_) =>
          Err(RenderRegionError::NoFine { dc_or_ds_tick_pos: bar.base_start_tick() }),
        FineSegno::Both { fine_pos, segno_pos } => Ok (
          if regions.is_empty() {
            Box::new(
              create_ds(vec![Box::new(SequenceRegion { start_tick: 0, end_tick: bar.base_start_tick() })], segno_pos, fine_pos)
            )
          } else {
            Box::new(create_ds(regions, segno_pos, fine_pos))
          }
        )
      }
    }

    fine_segno = fine_segno.on_bar(&bar)?;

    state = match &state {
      RenderRegionState::Idle => {
        if bar.repeats.contains(Repeat::Dc) {
          return on_dc(bar, regions, fine_segno);
        } else if bar.repeats.contains(Repeat::Ds) {
          return on_ds(bar, regions, fine_segno);
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
          state
        }
      },
      RenderRegionState::Seq { start_tick } => {
        if bar.repeats.contains(Repeat::End) {
          return Err(RenderRegionError::OrphanRepeatEnd{ tick_pos: bar.base_start_tick() });
        } else if bar.repeats.contains(Repeat::Dc) {
          regions.push(Box::new(SequenceRegion { start_tick: *start_tick, end_tick: bar.base_start_tick() }));
          return on_dc(bar, regions, fine_segno);
        } else if bar.repeats.contains(Repeat::Ds) {
          regions.push(Box::new(SequenceRegion { start_tick: *start_tick, end_tick: bar.base_start_tick() }));
          return on_ds(bar, regions, fine_segno);
        } else if bar.repeats.contains(Repeat::Start) {
          regions.push(Box::new(SequenceRegion { start_tick: *start_tick, end_tick: bar.base_start_tick() }));
          RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
        } else {
          state
        }
      },
      RenderRegionState::RepeatStart { start_tick } => {
        if bar.repeats.contains(Repeat::Start) && bar.repeats.contains(Repeat::End) {
          if bar.repeats.contains(Repeat::Dc) {
            return Err(RenderRegionError::RepeatOrVariationOnDc { tick_pos: bar.base_start_tick() });
          } else if bar.repeats.contains(Repeat::Ds) {
            return Err(RenderRegionError::RepeatOrVariationOnDs { tick_pos: bar.base_start_tick() });
          }
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { start_tick: *start_tick, end_tick: bar.base_start_tick() }}));
          RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
        } else if bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { start_tick: *start_tick, end_tick: bar.base_start_tick() }}));
          if bar.repeats.contains(Repeat::Dc) {
            return on_dc(bar, regions, fine_segno);
          } else if bar.repeats.contains(Repeat::Ds) {
            return on_ds(bar, regions, fine_segno);
          } else {
            RenderRegionState::Seq { start_tick: bar.base_start_tick() }
          }
        } else if bar.repeats.contains(Repeat::Start) {
          if bar.repeats.contains(Repeat::Dc) {
            return Err(RenderRegionError::RepeatOrVariationOnDc { tick_pos: bar.base_start_tick() });
          } else if bar.repeats.contains(Repeat::Ds) {
            return Err(RenderRegionError::RepeatOrVariationOnDs { tick_pos: bar.base_start_tick() });
          }
          return Err(RenderRegionError::DuplicatedRepeatStart { tick_pos: bar.base_start_tick() });
        } else if let Some(idx) = bar.repeats.region_index() {
          if idx != VarIndex::VI1 {
            return Err(RenderRegionError::InvalidRegionIndex { tick_pos: bar.base_start_tick(), actual: idx, expected: VarIndex::VI1 });
          }
          RenderRegionState::Variation { start_tick: *start_tick, region_start_ticks: vec![bar.base_start_tick()] }
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
          } else if bar.repeats.contains(Repeat::Ds) {
            return on_ds(&bar, regions, fine_segno);
          } else if bar.repeats.contains(Repeat::Dc) {
            return on_dc(&bar, regions, fine_segno);
          } else {
            RenderRegionState::Seq { start_tick: bar.base_start_tick() }
          }
        }
      },
    }
  }

  match state {
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

use super::FineSegno;

  #[test]
  fn empty() {
    let bars: Vec<Bar> = vec![];
    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunks();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk::new(0, u32::MAX));
  }

  // 0    100
  //   A  |
  #[test]
  fn single_bar() {
    let bar = Bar::new(100, None, None, crate::repeat_set!());
    let bars = vec![bar];
    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunks();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk::new(0, u32::MAX));
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
    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunks();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], Chunk::new(0, 100));
    assert_eq!(chunks[1], Chunk::new(0, 100));
    assert_eq!(chunks[2], Chunk::new(100, u32::MAX));
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

    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunks();
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0], Chunk::new(0, 100));
    assert_eq!(chunks[1], Chunk::new(100, 200));
    assert_eq!(chunks[2], Chunk::new(100, 200));
    assert_eq!(chunks[3], Chunk::new(200, u32::MAX));
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

    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunks();
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0], Chunk::new(0, 50));
    assert_eq!(chunks[1], Chunk::new(50, 200));
    assert_eq!(chunks[2], Chunk::new(50, 200));
    assert_eq!(chunks[3], Chunk::new(200, u32::MAX));
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

    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunks();
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 50));
    assert_eq!(chunks[1], Chunk::new(50, 100));
    assert_eq!(chunks[2], Chunk::new(0, 50));
    assert_eq!(chunks[3], Chunk::new(100, 150));
    assert_eq!(chunks[4], Chunk::new(150, u32::MAX));
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

    let chunks = render_region(Rhythm::new(4, 4), bars.iter()).unwrap().to_chunks();
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 100));
    assert_eq!(chunks[1], Chunk::new(100, 350));
    assert_eq!(chunks[2], Chunk::new(0, 100));
    assert_eq!(chunks[3], Chunk::new(350, 650));
    assert_eq!(chunks[4], Chunk::new(650, u32::MAX));
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

    let chunks = render_region(Rhythm::new(2, 4), bars.iter()).unwrap().to_chunks();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], Chunk::new(0, 1440));
    assert_eq!(chunks[1], Chunk::new(0, 480));
  }

  // Non auftakt.
  // 0    480         580       730        880       1030      1180    1280
  //   A   |[1]  B   |[1]   C   |[2]   D   |[2]   E   |Fine F   |   G   |DC
  //
  //   A   |     B    |     C   |   A  |   D   |   E   |   F   |   G   |  A  |   D   |   E   |
  #[test]
  fn simple_var_dc() {
    let bars = vec![
      Bar::new(480, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(580, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(730, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(880, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(1030, None, None, repeat_set!(Repeat::Fine)),
      Bar::new(1180, None, None, repeat_set!()),
      Bar::new(1280, None, None, repeat_set!(Repeat::Dc)),
    ];

    let region = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 7);
    assert_eq!(chunks[0], Chunk::new(0, 480));
    assert_eq!(chunks[1], Chunk::new(480, 730));
    assert_eq!(chunks[2], Chunk::new(0, 480));
    assert_eq!(chunks[3], Chunk::new(730, 1030));
    assert_eq!(chunks[4], Chunk::new(1030, 1280));
    assert_eq!(chunks[5], Chunk::new(0, 480));
    assert_eq!(chunks[6], Chunk::new(730, 1030));
  }

  // 0 480    530        630   730 
  // A |: B :|:[Fine] C :|: D :|[D.C.]
  #[test]
  fn dc_and_repeat_end() {
    let bars = vec![
      Bar::new(480, None, None, repeat_set!(Repeat::Start)),
      Bar::new(530, None, None, repeat_set!(Repeat::Start, Repeat::End, Repeat::Fine)),
      Bar::new(630, None, None, repeat_set!(Repeat::Start, Repeat::End)),
      Bar::new(730, None, None, repeat_set!(Repeat::Dc, Repeat::End)),
    ];

    let region = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 9);

    assert_eq!(chunks[0], Chunk::new(0, 480));
    assert_eq!(chunks[1], Chunk::new(480, 530));
    assert_eq!(chunks[2], Chunk::new(480, 530));
    assert_eq!(chunks[3], Chunk::new(530, 630));
    assert_eq!(chunks[4], Chunk::new(530, 630));
    assert_eq!(chunks[5], Chunk::new(630, 730));
    assert_eq!(chunks[6], Chunk::new(630, 730));
    assert_eq!(chunks[7], Chunk::new(0, 480));
    assert_eq!(chunks[8], Chunk::new(480, 530));
  }

  // 0 50   100 150  200  250  300 350  400 450
  // A |: B | C |1 D |2 E |: F | G |1 H |2 I| J
  //
  // 0 50  100 150 200 250 300 350 400 450 500 550 600 650
  // A | B | C | D | B | C | E | F | G | H | F | G | I | J
  #[test]
  fn two_vars() {
    let bars = vec![
      Bar::new(50, None, None, repeat_set!(Repeat::Start)),
      Bar::new(100, None, None, repeat_set!()),
      Bar::new(150, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(200, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(250, None, None, repeat_set!(Repeat::Start)),
      Bar::new(300, None, None, repeat_set!()),
      Bar::new(350, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(400, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(450, None, None, repeat_set!()),
    ];

    let region = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 10);

    assert_eq!(chunks[0], Chunk::new(0, 50));
    assert_eq!(chunks[1], Chunk::new(50, 150));
    assert_eq!(chunks[2], Chunk::new(150, 200));
    assert_eq!(chunks[3], Chunk::new(50, 150));
    assert_eq!(chunks[4], Chunk::new(200, 250));
    assert_eq!(chunks[5], Chunk::new(250, 350));
    assert_eq!(chunks[6], Chunk::new(350, 400));
    assert_eq!(chunks[7], Chunk::new(250, 350));
    assert_eq!(chunks[8], Chunk::new(400, 450));
    assert_eq!(chunks[9], Chunk::new(450, u32::MAX));
  }

  // 0 50   100 150  200  250  300  350  400 450 500 550 600
  // A |: B | C |1 D |2 E |  F |: G | H  |1 I|1 J |2 K| L |
  //
  // 0 50  100 150 200 250 300 350 400 450 500 550 600 650 700 750
  // A | B | C | D | B | C | E | F | G | H | I | J | G | H | K | L
  #[test]
  fn two_vars_with_end_bar() {
    let bars = vec![
      Bar::new(50, None, None, repeat_set!(Repeat::Start)),
      Bar::new(100, None, None, repeat_set!()),
      Bar::new(150, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(200, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(250, None, None, repeat_set!()),
      Bar::new(300, None, None, repeat_set!(Repeat::Start)),
      Bar::new(350, None, None, repeat_set!()),
      Bar::new(400, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(450, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(500, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(550, None, None, repeat_set!()),
      Bar::new(600, None, None, repeat_set!()),
    ];

    let region = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 11);

    assert_eq!(chunks[0], Chunk::new(0, 50));
    assert_eq!(chunks[1], Chunk::new(50, 150));
    assert_eq!(chunks[2], Chunk::new(150, 200));
    assert_eq!(chunks[3], Chunk::new(50, 150));
    assert_eq!(chunks[4], Chunk::new(200, 250));
    assert_eq!(chunks[5], Chunk::new(250, 300));
    assert_eq!(chunks[6], Chunk::new(300, 400));
    assert_eq!(chunks[7], Chunk::new(400, 500));
    assert_eq!(chunks[8], Chunk::new(300, 400));
    assert_eq!(chunks[9], Chunk::new(500, 550));
    assert_eq!(chunks[10], Chunk::new(550, u32::MAX));
  }

  // 0 240  320 370  420  470  520  570  620 670     720 770 820  870 920 970
  // A |: B | C |1 D |2 E |  F |: G | H  |1 I|2 J Fine| K |: L| M | N :| O |DC
  //
  // A | B | C | D | B | C | E | F | G | H | I | G | H | J | K | L | M | N | L | M 
  // |  N | O  | B  | C  | E  | F  | G  | H  | J  |
  #[test]
  fn repeat_and_dc() {
    let bars = vec![
      Bar::new(240, None, None, repeat_set!(Repeat::Start)),
      Bar::new(320, None, None, repeat_set!()),
      Bar::new(370, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(420, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(470, None, None, repeat_set!()),
      Bar::new(520, None, None, repeat_set!(Repeat::Start)),
      Bar::new(570, None, None, repeat_set!()),
      Bar::new(620, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(670, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(720, None, None, repeat_set!(Repeat::Fine)),
      Bar::new(770, None, None, repeat_set!(Repeat::Start)),
      Bar::new(820, None, None, repeat_set!()),
      Bar::new(870, None, None, repeat_set!()),
      Bar::new(920, None, None, repeat_set!(Repeat::End)),
      Bar::new(970, None, None, repeat_set!(Repeat::Dc)),
    ];

    let region = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 20);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 240));
    assert_eq!(*z.next().unwrap(), Chunk::new(240, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(370, 420));
    assert_eq!(*z.next().unwrap(), Chunk::new(240, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(420, 470));

    assert_eq!(*z.next().unwrap(), Chunk::new(470, 520));
    assert_eq!(*z.next().unwrap(), Chunk::new(520, 620));
    assert_eq!(*z.next().unwrap(), Chunk::new(620, 670));
    assert_eq!(*z.next().unwrap(), Chunk::new(520, 620));
    assert_eq!(*z.next().unwrap(), Chunk::new(670, 720));

    assert_eq!(*z.next().unwrap(), Chunk::new(720, 770));
    assert_eq!(*z.next().unwrap(), Chunk::new(770, 920));
    assert_eq!(*z.next().unwrap(), Chunk::new(770, 920));
    assert_eq!(*z.next().unwrap(), Chunk::new(920, 970));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 240));

    assert_eq!(*z.next().unwrap(), Chunk::new(240, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(420, 470));
    assert_eq!(*z.next().unwrap(), Chunk::new(470, 520));
    assert_eq!(*z.next().unwrap(), Chunk::new(520, 620));
    assert_eq!(*z.next().unwrap(), Chunk::new(670, 720));
    assert_eq!(z.next(), None);
  }

  // 0    50          200      300
  //   A  |     B    :|:  C    :|
  //
  // 0    50          200  250         400    500   600
  //   A  |     B     |  A  |     B     |  C   |  C  |
  #[test]
  fn consecutive_repeat() {
    let bars = vec![
      Bar::new(50, None, None, repeat_set!()),
      Bar::new(200, None, None, repeat_set!(Repeat::Start, Repeat::End)),
      Bar::new(300, None, None, repeat_set!(Repeat::End)),
    ];

    let region = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 5);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 200));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 200));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 300));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 300));
    assert_eq!(*z.next().unwrap(), Chunk::new(300, u32::MAX));
    assert_eq!(z.next(), None);
  }

  // 0   120          270          370
  //   A  |     B    :|:Fine  C    :| D.C
  //
  //   A  |     B     |  A  |     B     |  C   |  C  |   A  |  B  |
  #[test]
  fn fine_and_repeat() {
    let bars = vec![
      Bar::new(120, None, None, repeat_set!()),
      Bar::new(270, None, None, repeat_set!(Repeat::Start, Repeat::End, Repeat::Fine)),
      Bar::new(370, None, None, repeat_set!(Repeat::Dc, Repeat::End)),
    ];

    let region = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 5);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(z.next(), None);
  }

  // 0 120   170       270   370
  // A |: B :|:Fine C :|: D :|D.C.
  // 
  // A  B  B   C   C   D   D   A   B 
  #[test]
  fn dc_and_repeat() {
    let bars = vec![
      Bar::new(120, None, None, repeat_set!(Repeat::Start)),
      Bar::new(170, None, None, repeat_set!(Repeat::Start, Repeat::End, Repeat::Fine)),
      Bar::new(270, None, None, repeat_set!(Repeat::Start, Repeat::End)),
      Bar::new(370, None, None, repeat_set!(Repeat::Dc, Repeat::End)),
    ];

    let region = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 9);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 120));
    assert_eq!(*z.next().unwrap(), Chunk::new(120, 170));
    assert_eq!(*z.next().unwrap(), Chunk::new(120, 170));
    assert_eq!(*z.next().unwrap(), Chunk::new(170, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(170, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 120));
    assert_eq!(*z.next().unwrap(), Chunk::new(120, 170));
    assert_eq!(z.next(), None);
  }

  #[test]
  fn fine_segno() {
    assert_eq!(
      Ok(FineSegno::Fine(120)),
      FineSegno::Nothing.on_bar(&Bar::new(120, None, None, repeat_set!(Repeat::Fine)))
    );

    assert_eq!(
      Ok(FineSegno::Segno(120)),
      FineSegno::Nothing.on_bar(&Bar::new(120, None, None, repeat_set!(Repeat::Segno)))
    );

    assert_eq!(
      Err(RenderRegionError::FineNotAfterSegno { segno_pos: 120, fine_pos: 120 }),
      FineSegno::Nothing.on_bar(&Bar::new(120, None, None, repeat_set!(Repeat::Segno, Repeat::Fine)))
    );

    assert_eq!(
      Err(RenderRegionError::DuplicatedFine { tick_pos: [120, 240] }),
      FineSegno::Fine(120).on_bar(&Bar::new(240, None, None, repeat_set!(Repeat::Fine)))
    );

    assert_eq!(
      Err(RenderRegionError::DuplicatedSegno { tick_pos: [120, 240] }),
      FineSegno::Segno(120).on_bar(&Bar::new(240, None, None, repeat_set!(Repeat::Segno)))
    );

    assert_eq!(
      Err(RenderRegionError::FineNotAfterSegno { segno_pos: 120, fine_pos: 120 }),
      FineSegno::Segno(120).on_bar(&Bar::new(120, None, None, repeat_set!(Repeat::Fine)))
    );

    assert_eq!(
      Err(RenderRegionError::FineNotAfterSegno { segno_pos: 121, fine_pos: 120 }),
      FineSegno::Fine(120).on_bar(&Bar::new(121, None, None, repeat_set!(Repeat::Segno)))
    );

    assert_eq!(
      Ok(FineSegno::Both { segno_pos: 120, fine_pos: 121 }),
      FineSegno::Segno(120).on_bar(&Bar::new(121, None, None, repeat_set!(Repeat::Fine)))
    );

    assert_eq!(
      Err(RenderRegionError::DuplicatedFine { tick_pos: [120, 121] }),
      FineSegno::Both { segno_pos: 100, fine_pos: 120 }.on_bar(&Bar::new(121, None, None, repeat_set!(Repeat::Fine)))
    );
    
    assert_eq!(
      Err(RenderRegionError::DuplicatedSegno { tick_pos: [120, 121] }),
      FineSegno::Both { segno_pos: 120, fine_pos: 220 }.on_bar(&Bar::new(121, None, None, repeat_set!(Repeat::Segno)))
    );
  }

  // 0   120   200         270          370
  //   A  |  B  |Segno C   :|:Fine  D    :| D.S
  //
  //   A  |     B     |  C  |     A     |  B   |  C  |   D  |  D  | A | B |
  #[test]
  fn simple_ds() {
    let bars = vec![
      Bar::new(120, None, None, repeat_set!()),
      Bar::new(200, None, None, repeat_set!(Repeat::Segno)),
      Bar::new(270, None, None, repeat_set!(Repeat::Start, Repeat::End, Repeat::Fine)),
      Bar::new(370, None, None, repeat_set!(Repeat::Ds, Repeat::End)),
    ];

    let region = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 5);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 270));
    assert_eq!(z.next(), None);
  }

  // 0   120   200         270          370    470     570
  //   A  |  B  |Segno C   :|:Fine  D    |[1] E |[2] F  |D.S
  //
  //   A  |  B  |  C  |  A  |  B  |  C  |   D  |  E  | D | F | C
  #[test]
  fn var_and_ds() {
    let bars = vec![
      Bar::new(120, None, None, repeat_set!()),
      Bar::new(200, None, None, repeat_set!(Repeat::Segno)),
      Bar::new(270, None, None, repeat_set!(Repeat::Start, Repeat::End, Repeat::Fine)),
      Bar::new(370, None, None, repeat_set!(Repeat::Var1)),
      Bar::new(470, None, None, repeat_set!(Repeat::Var2)),
      Bar::new(570, None, None, repeat_set!(Repeat::Ds)),
    ];

    let region = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 7);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(370, 470));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(470, 570));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 270));
    assert_eq!(z.next(), None);
  }    
}
