use std::{ops::Range, fmt::Display};
use error_stack::{Context, report};
use gcollections::ops::{Intersection, Union, Bounded};
use interval::{IntervalSet, interval_set::ToIntervalSet};
use error_stack::Result;
use klavier_helper::store::Store;
use crate::{bar::{Bar, VarIndex, Repeat}, rhythm::Rhythm, have_start_tick::HaveBaseStartTick, global_repeat::{GlobalRepeat, RenderRegionWarning, GlobalRepeatBuilder}};

// Accumulated tick after repeats are rendered.
pub type AccumTick = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
  start_tick: u32,
  end_tick: u32,
}

impl Chunk {
  pub fn new(start_tick: u32, end_tick: u32) -> Self {
    Self {
      start_tick, end_tick
    }
  }

  pub fn start_tick(self) -> u32 {
    self.start_tick
  }

  pub fn end_tick(self) -> u32 {
    self.end_tick
  }

  pub fn optimize(chunks: &[Chunk]) -> Vec<Chunk> {
    let mut ret: Vec<Chunk> = vec![];
    let mut z = chunks.iter();
    let mut cur = match z.next() {
      None => return ret,
      Some(c) => *c,
    };

    for next in z {
      if cur.end_tick == next.start_tick {
        cur.end_tick = next.end_tick;
      } else {
        ret.push(cur);
        cur = *next;
      }
    }

    ret.push(cur);
    ret
  }

  pub fn contains(self, tick: u32) -> bool {
    self.start_tick <= tick && tick < self.end_tick
  }

  pub fn len(self) -> u32 {
    self.end_tick - self.start_tick()
  }

  pub fn by_accum_tick(chunks: &[Chunk]) -> Store<AccumTick, Chunk, ()> {
    let mut offset: u32 = 0;
    let mut buf: Store<AccumTick, Chunk, ()> = Store::new(false);

    for c in chunks {
        buf.add(offset, c.clone(), ());
        if c.end_tick() != u32::MAX {
            offset += c.len();
        }
    }

    buf
  }
}

pub trait Region: std::fmt::Debug {
  fn to_chunks(&self) -> Vec<Chunk>;
  fn to_iter1_interval_set(&self) -> IntervalSet<u32>;
}

#[derive(Debug, Clone, PartialEq)]
enum RenderPhase {
  NonDcDs,
  DcDsIter0 { dc_ds_tick: u32 },
  DcDsIter1 { dc_ds_tick: u32, global_repeat: GlobalRepeat },
}

// SimpleRegion can be stored in a compound region.
trait SimpleRegion: Region {
  fn render_chunks(&self, phase: &RenderPhase) -> Vec<Chunk>;
  fn to_iter1_chunks(&self, global_repeat: &GlobalRepeat) -> Vec<Chunk> {
    let sections: IntervalSet<u32> = global_repeat.iter1_interval_set().clone().intersection(
      &self.to_iter1_interval_set()
    );

    sections.into_iter().map(|sec| {
      Chunk::new(sec.lower(), sec.upper() + 1)
    }).collect()
  }
}

#[derive(Debug, PartialEq, Eq)]
struct NullRegion;

impl Region for NullRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    vec![]
  }

  fn to_iter1_interval_set(&self) -> IntervalSet<u32> {
    vec![].to_interval_set()
  }
}

impl SimpleRegion for NullRegion {
  fn render_chunks(&self, _phase: &RenderPhase) -> Vec<Chunk> {
    vec![]
  }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SequenceRegion {
  tick_range: Range<u32>,
}

impl SequenceRegion {
  #[inline]
  fn tick_len(&self) -> u32 {
    self.tick_range.len() as u32
  }

  #[inline]
  fn start_tick(&self) -> u32 {
    self.tick_range.start
  }

  // Exclusive
  #[inline]
  fn end_tick(&self) -> u32 {
    self.tick_range.end
  }
}

impl Region for SequenceRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    self.render_chunks(&RenderPhase::NonDcDs)
  }

  fn to_iter1_interval_set(&self) -> IntervalSet<u32> {
    (self.tick_range.start, self.tick_range.end - 1).to_interval_set()
  }
}

impl SimpleRegion for SequenceRegion {
  fn render_chunks(&self, phase: &RenderPhase) -> Vec<Chunk> {
    match phase {
      RenderPhase::NonDcDs => vec![Chunk { start_tick: self.start_tick(), end_tick: self.end_tick() }],
      RenderPhase::DcDsIter0 { dc_ds_tick } => {
        if self.end_tick() <= *dc_ds_tick {
          vec![Chunk::new(self.start_tick(), self.end_tick())]
        } else if self.start_tick() < *dc_ds_tick && *dc_ds_tick < self.end_tick() {
          vec![Chunk::new(self.start_tick(), *dc_ds_tick)]
        } else {
          vec![]
        }
      }
      RenderPhase::DcDsIter1 { dc_ds_tick, global_repeat } => {
        self.to_iter1_chunks(global_repeat)
      }
    }
  }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RepeatRegion {
  region: SequenceRegion,
}

impl Region for RepeatRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    self.render_chunks(&RenderPhase::NonDcDs)
  }

  fn to_iter1_interval_set(&self) -> IntervalSet<u32> {
    (self.region.start_tick(), self.region.end_tick() - 1).to_interval_set()
  }
}

impl SimpleRegion for RepeatRegion {
  fn render_chunks(&self, phase: &RenderPhase) -> Vec<Chunk> {
    fn full(sr: &RepeatRegion) -> Vec<Chunk> {
      let mut chunks = Vec::with_capacity(2);
      chunks.extend(sr.region.to_chunks());
      chunks.extend(sr.region.to_chunks());
      chunks
    }

    match phase {
        RenderPhase::NonDcDs => full(&self),
        RenderPhase::DcDsIter0 { dc_ds_tick } => {
          if self.region.end_tick() <= *dc_ds_tick {
            full(&self)
          } else if self.region.end_tick() < *dc_ds_tick && *dc_ds_tick < self.region.end_tick() {
            // This condition should not occur.
            panic!("Logic error.");
          } else {
            vec![]
          }
        },
        RenderPhase::DcDsIter1 { dc_ds_tick, global_repeat } => {
          self.to_iter1_chunks(global_repeat)
        }
    }
  }
}

#[derive(Debug)]
pub struct VariationRegion {
  common: SequenceRegion,
  variations: Vec<SequenceRegion>,
}

impl VariationRegion {
  fn end_tick(&self) -> u32 {
    self.variations.last().unwrap().end_tick()
  }
}

impl VariationRegion {
  fn last_variation(&self) -> &SequenceRegion {
    &self.variations[self.variations.len() - 1]
  }
}

impl Region for VariationRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    self.render_chunks(&RenderPhase::NonDcDs)
  }

  fn to_iter1_interval_set(&self) -> IntervalSet<u32> {
    self.common.to_iter1_interval_set().union(
      &self.last_variation().to_iter1_interval_set()
    )
  }
}

impl SimpleRegion for VariationRegion {
  fn render_chunks(&self, phase: &RenderPhase) -> Vec<Chunk> {
    fn full(vr: &VariationRegion) -> Vec<Chunk> {
      let mut chunks = vec![];
      let common = vr.common.to_chunks();
      for v in vr.variations.iter() {
        chunks.extend(common.clone());
        chunks.extend(v.to_chunks());
      }

      chunks
    }

    match phase {
        RenderPhase::NonDcDs => full(&self),
        RenderPhase::DcDsIter0 { dc_ds_tick } => {
          if *dc_ds_tick <= self.common.start_tick() {
            vec![]
          } else if *dc_ds_tick < self.end_tick() {
            // This condition should not occur.
            panic!("Logic error");
          } else {
            full(&self)
          }
        },
        RenderPhase::DcDsIter1 { dc_ds_tick, global_repeat } => {
          self.to_iter1_chunks(global_repeat)
        }
    }
  }
}

#[derive(Debug)]
pub struct CompoundRegion {
  global_repeat: Option<GlobalRepeat>,
  regions: Vec<Box<dyn SimpleRegion>>,
}

impl Region for CompoundRegion {
  fn to_chunks(&self) -> Vec<Chunk> {
    match self.global_repeat.as_ref() {
        Some(gr) => {
          let mut chunks = vec![];
          for r in self.regions.iter() {
            chunks.extend(r.render_chunks(&RenderPhase::DcDsIter0 { dc_ds_tick: gr.ds_dc().tick() }));
          }
          for r in self.regions.iter() {
            chunks.extend(r.render_chunks(&RenderPhase::DcDsIter1 { dc_ds_tick: gr.ds_dc().tick(), global_repeat: gr.clone() } ));
          }

          chunks
        }
        None => {
          let mut chunks = vec![];
          for r in self.regions.iter() {
            chunks.extend(r.render_chunks(&RenderPhase::NonDcDs));
          }

          chunks
        }
    }
  }

  fn to_iter1_interval_set(&self) -> IntervalSet<u32> {
    let union = self.regions.iter().fold(
      vec![].to_interval_set(),
      |u, e| u.union(&e.to_iter1_interval_set())
    );
    
    self.global_repeat.as_ref().map(|gr| union.intersection(gr.iter1_interval_set())).unwrap_or(union)
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RenderRegionError {
  DuplicatedRepeatStart { tick: u32 },
  DuplicatedSegno { tick: [u32; 2] },
  DuplicatedDsDc { tick: [u32; 2] },
  DuplicatedFine { tick: [u32; 2] },
  OrphanRepeatEnd { tick: u32 },
  FineWithoutDsDc { tick: u32 },
  SegnoWithoutDs { tick: u32 },
  CodaWithoutDsDc { tick: [u32; 2]},
  NoRepeatEnd { tick: u32 },
  InvalidRegionIndex { tick: u32, actual: VarIndex, expected: VarIndex },
  RepeatInVariation { tick: u32 },
  VariationNotClosed { tick: u32 },
  RepeatOrVariationOnDc { tick: u32 },
  RepeatOrVariationOnDs { tick: u32 },
  FineNotAfterSegno { segno_tick: u32, fine_tick: u32 },
  NoSegnoForDs { ds_tick: u32 },
  MoreThanTwoCodas { tick: [u32; 3] },
  OnlyOneCoda { tick: u32 },
  DcDsWhileRepeat { tick: u32 },
  DcDsWhileVariation { tick: u32 },
  SegnoWhildVariation { tick: u32 },
  CodaAfterFine { coda_from: u32, coda_to: u32, fine: u32 },
}

impl Context for RenderRegionError {}

impl Display for RenderRegionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn render_region<'a>(tune_rhythm: Rhythm, bars: impl Iterator<Item = &'a Bar>) -> Result<(Box<dyn Region>, Vec<RenderRegionWarning>), RenderRegionError> {
  fn create_variation(start_tick: u32, region_start_ticks: Vec<u32>, end_tick: u32) -> Box<dyn SimpleRegion> {
    let mut variations: Vec<SequenceRegion> = vec![];
    let mut iter = region_start_ticks.iter();
    let mut tick = *iter.next().unwrap();
    for t in iter {
      variations.push(SequenceRegion { tick_range: tick..*t });
      tick = *t;
    }

    variations.push(SequenceRegion { tick_range: tick..end_tick });

    Box::new(VariationRegion {
      common: SequenceRegion { tick_range: start_tick..region_start_ticks[0] }, variations
    })
  }

  let mut regions: Vec<Box<dyn SimpleRegion>> = vec![];
  let mut state = RenderRegionState::Idle;
  let mut is_auftakt: Option<bool> = None;
  let mut global_repeat: GlobalRepeatBuilder = GlobalRepeatBuilder::new(tune_rhythm);

  for bar in bars {
    global_repeat = global_repeat.on_bar(&bar)?;
    if is_auftakt.is_none() {
      if bar.base_start_tick() != 0 {
        is_auftakt = Some(bar.base_start_tick() < tune_rhythm.tick_len())
      }
    }

    state = match &state {
      RenderRegionState::Idle => {
        if bar.repeats.contains(Repeat::Start) && bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { tick_range: 0..bar.base_start_tick() }}));
          RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
        } else if bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { tick_range: 0..bar.base_start_tick() }}));
          RenderRegionState::Seq { start_tick: bar.base_start_tick() }
        } else if bar.repeats.contains(Repeat::Start) {
          regions.push(Box::new(SequenceRegion { tick_range: 0..bar.base_start_tick() }));
          RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
        } else if let Some(idx) = bar.repeats.region_index() {
          if idx != VarIndex::VI1 {
            return Err(report!(RenderRegionError::InvalidRegionIndex { tick: bar.base_start_tick(), actual: idx, expected: VarIndex::VI1 }));
          }
          RenderRegionState::Variation { start_tick: 0, region_start_ticks: vec![bar.base_start_tick()] }
        } else {
          state
        }
      },
      RenderRegionState::Seq { start_tick } => {
        if bar.repeats.contains(Repeat::End) {
          return Err(report!(RenderRegionError::OrphanRepeatEnd{ tick: bar.base_start_tick() }));
        } else if bar.repeats.contains(Repeat::Start) {
          regions.push(Box::new(SequenceRegion { tick_range: *start_tick..bar.base_start_tick() }));
          RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
        } else {
          state
        }
      },
      RenderRegionState::RepeatStart { start_tick } => {
        if bar.repeats.contains(Repeat::Start) && bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { tick_range: *start_tick..bar.base_start_tick() }}));
          RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
        } else if bar.repeats.contains(Repeat::End) {
          regions.push(Box::new(RepeatRegion { region: SequenceRegion { tick_range: *start_tick..bar.base_start_tick() }}));
          RenderRegionState::Seq { start_tick: bar.base_start_tick() }
        } else if bar.repeats.contains(Repeat::Start) {
          return Err(report!(RenderRegionError::DuplicatedRepeatStart { tick: bar.base_start_tick() }));
        } else if bar.repeats.contains(Repeat::Dc) || bar.repeats.contains(Repeat::Ds) {
          return Err(report!(RenderRegionError::DcDsWhileRepeat { tick: bar.base_start_tick() }));
        } else if let Some(idx) = bar.repeats.region_index() {
          if idx != VarIndex::VI1 {
            return Err(report!(RenderRegionError::InvalidRegionIndex { tick: bar.base_start_tick(), actual: idx, expected: VarIndex::VI1 }));
          }
          RenderRegionState::Variation { start_tick: *start_tick, region_start_ticks: vec![bar.base_start_tick()] }
        } else {
          state
        }
      },
      RenderRegionState::Variation { start_tick, region_start_ticks } => {
        if bar.repeats.contains(Repeat::End) {
          return Err(report!(RenderRegionError::RepeatInVariation { tick: bar.base_start_tick() }))
        } else if let Some(ri) = bar.repeats.region_index() {
          if bar.repeats.contains(Repeat::Dc) || bar.repeats.contains(Repeat::Ds) {
            return Err(report!(RenderRegionError::DcDsWhileVariation { tick: bar.base_start_tick() }));
          }
          let current_idx = region_start_ticks.len() as u8;
          let idx = ri.value();
          if idx == current_idx {
            state
          } else if idx == current_idx + 1 {
            let mut rst = region_start_ticks.clone();
            rst.push(bar.base_start_tick());
            RenderRegionState::Variation { start_tick: *start_tick, region_start_ticks: rst }
          } else {
            return Err(report!(RenderRegionError::InvalidRegionIndex {
              tick: bar.base_start_tick(), actual: ri, expected: VarIndex::from_value(current_idx + 1).unwrap()
            }));
          }
        } else {
          if let Some(segno) = global_repeat.segno {
            if region_start_ticks[0] <= segno && segno < *region_start_ticks.last().unwrap() {
              return Err(report!(RenderRegionError::SegnoWhildVariation { tick: bar.base_start_tick() }));
            }
          }
          regions.push(create_variation(*start_tick, region_start_ticks.clone(), bar.base_start_tick()));

          if bar.repeats.contains(Repeat::Start) {
            RenderRegionState::RepeatStart { start_tick: bar.base_start_tick() }
          } else {
            RenderRegionState::Seq { start_tick: bar.base_start_tick() }
          }
        }
      }
    }
  }

  match state {
    RenderRegionState::Idle => {
      regions.push(Box::new(SequenceRegion { tick_range: 0..u32::MAX }));
      let (gr, w) = global_repeat.build()?;
      Ok((Box::new(CompoundRegion { regions, global_repeat: gr }), w))
    },
    RenderRegionState::Seq { start_tick } => {
      regions.push(Box::new(SequenceRegion { tick_range: start_tick..u32::MAX }));
      let (gr, w) = global_repeat.build()?;
      Ok((Box::new(CompoundRegion { regions, global_repeat: gr }), w))
    }
    RenderRegionState::RepeatStart { start_tick } => Err(report!(RenderRegionError::NoRepeatEnd { tick: start_tick })),
    RenderRegionState::Variation { start_tick: _, region_start_ticks } => {
      Err(report!(RenderRegionError::VariationNotClosed { tick: *region_start_ticks.last().unwrap() }))
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{bar::{Bar, Repeat}, play_iter::PlayIter, play_start_tick::{PlayStartTick, ToAccumTickError}, repeat::{render_region, Chunk, GlobalRepeatBuilder, RenderRegionError, SimpleRegion}, rhythm::Rhythm};
  use crate::repeat_set;
  use super::{AccumTick, RenderPhase, SequenceRegion};
  use crate::bar::RepeatSet;
  use error_stack::Result;

  fn to_accum_tick(tick: u32, iter: u8, chunks: &[(AccumTick, Chunk)]) -> std::result::Result<AccumTick, ToAccumTickError> {
    PlayStartTick::new(tick, iter).to_accum_tick(chunks)
  }

  #[test]
  fn empty() {
    let bars: Vec<Bar> = vec![];
    let (region, warnings) = render_region(Rhythm::new(4, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk::new(0, u32::MAX));

    let by_accum_tick = Chunk::by_accum_tick(&chunks);
    assert_eq!(to_accum_tick(0, 1, &by_accum_tick).unwrap(), 0);
    assert_eq!(to_accum_tick(100, 1, &by_accum_tick).unwrap(), 100);
    assert_eq!(to_accum_tick(100, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
  }

  // 0    100
  //   A  |
  #[test]
  fn single_bar() {
    let bar = Bar::new(100, None, None, crate::repeat_set!());
    let bars = vec![bar];
    let (region, warnings) = render_region(Rhythm::new(4, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk::new(0, u32::MAX));

    let by_accum_tick = Chunk::by_accum_tick(&chunks);
    assert_eq!(to_accum_tick(0, 1, &by_accum_tick).unwrap(), 0);
    assert_eq!(to_accum_tick(100, 1, &by_accum_tick).unwrap(), 100);
    assert_eq!(to_accum_tick(100, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
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
    let (region, warnings) = render_region(Rhythm::new(4, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], Chunk::new(0, 100));
    assert_eq!(chunks[1], Chunk::new(0, 100));
    assert_eq!(chunks[2], Chunk::new(100, u32::MAX));

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], Chunk::new(0, 100));
    assert_eq!(chunks[1], Chunk::new(0, u32::MAX));

    let by_accum_tick = Chunk::by_accum_tick(&chunks);
    assert_eq!(to_accum_tick(0, 1, &by_accum_tick).unwrap(), 0);
    assert_eq!(to_accum_tick(50, 1, &by_accum_tick).unwrap(), 50);
    assert_eq!(to_accum_tick(0, 2, &by_accum_tick).unwrap(), 100);
    assert_eq!(to_accum_tick(50, 2, &by_accum_tick).unwrap(), 150);
    assert_eq!(to_accum_tick(100, 1, &by_accum_tick).unwrap(), 200);
    assert_eq!(to_accum_tick(150, 1, &by_accum_tick).unwrap(), 250);
    assert_eq!(to_accum_tick(100, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
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

    let e =  render_region(Rhythm::new(4, 4), bars.iter()).unwrap_err();
    let err: &RenderRegionError = e.current_context();
    assert_eq!(*err, RenderRegionError::OrphanRepeatEnd { tick: 200 });
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

    let (region, warnings) = render_region(Rhythm::new(4, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0], Chunk::new(0, 100));
    assert_eq!(chunks[1], Chunk::new(100, 200));
    assert_eq!(chunks[2], Chunk::new(100, 200));
    assert_eq!(chunks[3], Chunk::new(200, u32::MAX));

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], Chunk::new(0, 200));
    assert_eq!(chunks[1], Chunk::new(100, u32::MAX));

    let by_accum_tick = Chunk::by_accum_tick(&chunks);
    assert_eq!(to_accum_tick(0, 1, &by_accum_tick).unwrap(), 0);
    assert_eq!(to_accum_tick(0, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
    assert_eq!(to_accum_tick(50, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
    assert_eq!(to_accum_tick(100, 1, &by_accum_tick).unwrap(), 100);
    assert_eq!(to_accum_tick(150, 1, &by_accum_tick).unwrap(), 150);
    assert_eq!(to_accum_tick(100, 2, &by_accum_tick).unwrap(), 200);
    assert_eq!(to_accum_tick(150, 2, &by_accum_tick).unwrap(), 250);
    assert_eq!(to_accum_tick(200, 1, &by_accum_tick).unwrap(), 300);
    assert_eq!(to_accum_tick(250, 1, &by_accum_tick).unwrap(), 350);
    assert_eq!(to_accum_tick(200, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
    assert_eq!(to_accum_tick(250, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
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

    let (region, warnings) = render_region(Rhythm::new(4, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0], Chunk::new(0, 50));
    assert_eq!(chunks[1], Chunk::new(50, 200));
    assert_eq!(chunks[2], Chunk::new(50, 200));
    assert_eq!(chunks[3], Chunk::new(200, u32::MAX));

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], Chunk::new(0, 200));
    assert_eq!(chunks[1], Chunk::new(50, u32::MAX));

    let by_accum_tick = Chunk::by_accum_tick(&chunks);
    assert_eq!(to_accum_tick(0, 1, &by_accum_tick).unwrap(), 0);
    assert_eq!(to_accum_tick(50, 1, &by_accum_tick).unwrap(), 50);
    assert_eq!(to_accum_tick(50, 2, &by_accum_tick).unwrap(), 200);
    assert_eq!(to_accum_tick(199, 2, &by_accum_tick).unwrap(), 349);
    assert_eq!(to_accum_tick(200, 1, &by_accum_tick).unwrap(), 350);
    assert_eq!(to_accum_tick(200, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
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

    let (region, warnings) = render_region(Rhythm::new(4, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 50));
    assert_eq!(chunks[1], Chunk::new(50, 100));
    assert_eq!(chunks[2], Chunk::new(0, 50));
    assert_eq!(chunks[3], Chunk::new(100, 150));
    assert_eq!(chunks[4], Chunk::new(150, u32::MAX));

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], Chunk::new(0, 100));
    assert_eq!(chunks[1], Chunk::new(0, 50));
    assert_eq!(chunks[2], Chunk::new(100, u32::MAX));

    let by_accum_tick = Chunk::by_accum_tick(&chunks);
    assert_eq!(to_accum_tick(0, 1, &by_accum_tick).unwrap(), 0);
    assert_eq!(to_accum_tick(0, 2, &by_accum_tick).unwrap(), 100);
    assert_eq!(to_accum_tick(20, 2, &by_accum_tick).unwrap(), 120);
    assert_eq!(to_accum_tick(100, 1, &by_accum_tick).unwrap(), 150);
    assert_eq!(to_accum_tick(100, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
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

    let (region, warnings) = render_region(Rhythm::new(4, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 100));
    assert_eq!(chunks[1], Chunk::new(100, 350));
    assert_eq!(chunks[2], Chunk::new(0, 100));
    assert_eq!(chunks[3], Chunk::new(350, 650));
    assert_eq!(chunks[4], Chunk::new(650, u32::MAX));

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], Chunk::new(0, 350));
    assert_eq!(chunks[1], Chunk::new(0, 100));
    assert_eq!(chunks[2], Chunk::new(350, u32::MAX));

    let by_accum_tick = Chunk::by_accum_tick(&chunks);
    assert_eq!(to_accum_tick(0, 1, &by_accum_tick).unwrap(), 0);
    assert_eq!(to_accum_tick(200, 1, &by_accum_tick).unwrap(), 200);
    assert_eq!(to_accum_tick(0, 2, &by_accum_tick).unwrap(), 350);
    assert_eq!(to_accum_tick(350, 1, &by_accum_tick).unwrap(), 450);
    assert_eq!(to_accum_tick(650, 1, &by_accum_tick).unwrap(), 750);
    assert_eq!(to_accum_tick(650, 2, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(2), max_iter: 1 }));
    assert_eq!(to_accum_tick(0, 3, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(3), max_iter: 2 }));
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

    let (region, warnings) = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], Chunk::new(0, 1440));
    assert_eq!(chunks[1], Chunk::new(0, 480));

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], Chunk::new(0, 1440));
    assert_eq!(chunks[1], Chunk::new(0, 480));

    let by_accum_tick = Chunk::by_accum_tick(&chunks);
    assert_eq!(to_accum_tick(0, 1, &by_accum_tick).unwrap(), 0);
    assert_eq!(to_accum_tick(0, 2, &by_accum_tick).unwrap(), 1440);
    assert_eq!(to_accum_tick(0, 3, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(3), max_iter: 2 }));
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

    let (region, warnings) = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 7);
    assert_eq!(chunks[0], Chunk::new(0, 480));
    assert_eq!(chunks[1], Chunk::new(480, 730));
    assert_eq!(chunks[2], Chunk::new(0, 480));
    assert_eq!(chunks[3], Chunk::new(730, 1030));
    assert_eq!(chunks[4], Chunk::new(1030, 1280));
    assert_eq!(chunks[5], Chunk::new(0, 480));
    assert_eq!(chunks[6], Chunk::new(730, 1030));

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 730));
    assert_eq!(chunks[1], Chunk::new(0, 480));
    assert_eq!(chunks[2], Chunk::new(730, 1280));
    assert_eq!(chunks[3], Chunk::new(0, 480));
    assert_eq!(chunks[4], Chunk::new(730, 1030));

    let by_accum_tick = Chunk::by_accum_tick(&chunks);
    assert_eq!(to_accum_tick(0, 1, &by_accum_tick).unwrap(), 0);
    assert_eq!(to_accum_tick(480, 1, &by_accum_tick).unwrap(), 480);
    assert_eq!(to_accum_tick(400, 2, &by_accum_tick).unwrap(), 730 + 400);
    assert_eq!(to_accum_tick(800, 1, &by_accum_tick).unwrap(), 730 + 480 + (800 - 730));
    assert_eq!(to_accum_tick(1180, 1, &by_accum_tick).unwrap(), 730 + 480 + (1180 - 730));
    assert_eq!(to_accum_tick(0, 3, &by_accum_tick).unwrap(), 730 + 480 + (1280 - 730));
    assert_eq!(to_accum_tick(0, 4, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(4), max_iter: 3 }));
    assert_eq!(to_accum_tick(730, 2, &by_accum_tick).unwrap(), 730 + 480 + (1280 - 730) + 480);
    assert_eq!(to_accum_tick(730, 3, &by_accum_tick), Err(ToAccumTickError::CannotFind { specified_iter: PlayIter::new(3), max_iter: 2 }));
  }

  // 0 480    530        600   1080 
  // A |: B :|:[Fine] C :|: D :|[D.C.]
  //
  // A B B C C D D A B
  #[test]
  fn dc_and_repeat_end() {
    let bars = vec![
      Bar::new(480, None, None, repeat_set!(Repeat::Start)),
      Bar::new(530, None, None, repeat_set!(Repeat::Start, Repeat::End, Repeat::Fine)),
      Bar::new(600, None, None, repeat_set!(Repeat::Start, Repeat::End)),
      Bar::new(1080, None, None, repeat_set!(Repeat::Dc, Repeat::End)),
    ];

    let (region, warnings) = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();

    assert_eq!(chunks.len(), 9);
    assert_eq!(chunks[0], Chunk::new(0, 480));
    assert_eq!(chunks[1], Chunk::new(480, 530));
    assert_eq!(chunks[2], Chunk::new(480, 530));
    assert_eq!(chunks[3], Chunk::new(530, 600));
    assert_eq!(chunks[4], Chunk::new(530, 600));
    assert_eq!(chunks[5], Chunk::new(600, 1080));
    assert_eq!(chunks[6], Chunk::new(600, 1080));
    assert_eq!(chunks[7], Chunk::new(0, 480));
    assert_eq!(chunks[8], Chunk::new(480, 530));

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 530));
    assert_eq!(chunks[1], Chunk::new(480, 600));
    assert_eq!(chunks[2], Chunk::new(530, 1080));
    assert_eq!(chunks[3], Chunk::new(600, 1080));
    assert_eq!(chunks[4], Chunk::new(0, 530));
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

    let (region, warnings) = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
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

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 200));
    assert_eq!(chunks[1], Chunk::new(50, 150));
    assert_eq!(chunks[2], Chunk::new(200, 400));
    assert_eq!(chunks[3], Chunk::new(250, 350));
    assert_eq!(chunks[4], Chunk::new(400, u32::MAX));
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

    let (region, warnings) = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
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

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0], Chunk::new(0, 200));
    assert_eq!(chunks[1], Chunk::new(50, 150));
    assert_eq!(chunks[2], Chunk::new(200, 500));
    assert_eq!(chunks[3], Chunk::new(300, 400));
    assert_eq!(chunks[4], Chunk::new(500, u32::MAX));

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

    let (region, warnings) = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
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

    let chunks = Chunk::optimize(&chunks);
    assert_eq!(chunks.len(), 9);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 420));
    assert_eq!(*z.next().unwrap(), Chunk::new(240, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(420, 670));
    assert_eq!(*z.next().unwrap(), Chunk::new(520, 620));
    assert_eq!(*z.next().unwrap(), Chunk::new(670, 920));
    assert_eq!(*z.next().unwrap(), Chunk::new(770, 970));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(420, 620));
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

    let (region, warnings) = render_region(Rhythm::new(2, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 5);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 200));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 200));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 300));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 300));
    assert_eq!(*z.next().unwrap(), Chunk::new(300, u32::MAX));
    assert_eq!(z.next(), None);

    let chunks = Chunk::optimize(&chunks);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 200));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 300));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, u32::MAX));
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

    let (region, warnings) = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 5);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(z.next(), None);

    let chunks = Chunk::optimize(&chunks);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 370));
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

    let (region, warnings) = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
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

    let chunks = Chunk::optimize(&chunks);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 170));
    assert_eq!(*z.next().unwrap(), Chunk::new(120, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(170, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 170));
    assert_eq!(z.next(), None);
  }

  // 0 120 170  270  370    470
  // A | B |: C | D :|D.C. E |
  // 
  // A B C D C D A B C D E
  #[test]
  fn dc_without_fine() {
    let bars = vec![
      Bar::new(120, None, None, repeat_set!()),
      Bar::new(170, None, None, repeat_set!(Repeat::Start)),
      Bar::new(270, None, None, repeat_set!()),
      Bar::new(370, None, None, repeat_set!(Repeat::Dc, Repeat::End)),
      Bar::new(470, None, None, repeat_set!()),
    ];

    let (region, warnings) = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 6);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 170));
    assert_eq!(*z.next().unwrap(), Chunk::new(170, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(170, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 170));
    assert_eq!(*z.next().unwrap(), Chunk::new(170, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(370, u32::MAX));
    assert_eq!(z.next(), None);

    let chunks = Chunk::optimize(&chunks);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(170, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, u32::MAX));
    assert_eq!(z.next(), None);
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

    let (region, warnings) = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 5);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 270));
    assert_eq!(z.next(), None);

    let chunks = Chunk::optimize(&chunks);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 370));
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

    let (region, warnings) = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
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

    let chunks = Chunk::optimize(&chunks);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 470));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(470, 570));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 270));
    assert_eq!(z.next(), None);
  }    

  // 0   120   200         270  370     470  570     670
  //   A  |  B  |Segno C   :| D |Coda E | F |Coda G  | D.S
  //
  //   A  |  B  |  C  |  A  |  B  |  C  |  D | E | F | G | C | D | G
  #[test]
  fn ds_and_coda() {
    let bars = vec![
      Bar::new(120, None, None, repeat_set!()),
      Bar::new(200, None, None, repeat_set!(Repeat::Segno)),
      Bar::new(270, None, None, repeat_set!(Repeat::End)),
      Bar::new(370, None, None, repeat_set!(Repeat::Coda)),
      Bar::new(470, None, None, repeat_set!()),
      Bar::new(570, None, None, repeat_set!(Repeat::Coda)),
      Bar::new(670, None, None, repeat_set!(Repeat::Ds)),
    ];

    let (region, warnings) = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 6);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 670));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(570, u32::MAX));
    assert_eq!(z.next(), None);

    let chunks = Chunk::optimize(&chunks);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 670));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(570, u32::MAX));
    assert_eq!(z.next(), None);
  }    

  // 0   120   200         270  370     470  570     670   770
  //   A  |  B  |Segno C   :| D |Coda E | F |Coda G  |Fine |D.S
  //
  //   A  |  B  |  C  |  A  |  B  |  C  |  D | E | G 
  #[test]
  fn ds_and_coda_fine() {
    let bars = vec![
      Bar::new(120, None, None, repeat_set!()),
      Bar::new(200, None, None, repeat_set!(Repeat::Segno)),
      Bar::new(270, None, None, repeat_set!(Repeat::End)),
      Bar::new(370, None, None, repeat_set!(Repeat::Coda)),
      Bar::new(470, None, None, repeat_set!()),
      Bar::new(570, None, None, repeat_set!(Repeat::Coda)),
      Bar::new(670, None, None, repeat_set!(Repeat::Fine)),
      Bar::new(770, None, None, repeat_set!(Repeat::Ds)),
    ];

    let (region, warnings) = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 6);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 770));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(570, 670));
    assert_eq!(z.next(), None);

    let chunks = Chunk::optimize(&chunks);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 770));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(570, 670));
    assert_eq!(z.next(), None);
  }    

  // 0   120   200         270  370     470 570  670
  //   A  |  B  |Segno C   :| D |Coda E | F |D.S |Coda H
  //
  //   A  |  B  |  C  |  A  |  B  |  C  |  D | E | F | C | D | H
  #[test]
  fn ds_and_coda_skip_ds() {
    let bars = vec![
      Bar::new(120, None, None, repeat_set!()),
      Bar::new(200, None, None, repeat_set!(Repeat::Segno)),
      Bar::new(270, None, None, repeat_set!(Repeat::End)),
      Bar::new(370, None, None, repeat_set!(Repeat::Coda)),
      Bar::new(470, None, None, repeat_set!()),
      Bar::new(570, None, None, repeat_set!(Repeat::Ds)),
      Bar::new(670, None, None, repeat_set!(Repeat::Coda)),
    ];

    let (region, warnings) = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 6);

    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 570));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(270, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(670, u32::MAX));
    assert_eq!(z.next(), None);

    let chunks = Chunk::optimize(&chunks);
    let mut z = chunks.iter();
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 270));
    assert_eq!(*z.next().unwrap(), Chunk::new(0, 570));
    assert_eq!(*z.next().unwrap(), Chunk::new(200, 370));
    assert_eq!(*z.next().unwrap(), Chunk::new(670, u32::MAX));
    assert_eq!(z.next(), None);
  }    

  #[test]  
  fn render_sequence_region_when_non_dcds() {
    let seq_region = SequenceRegion {
      tick_range: 100..200
    };
    let chunks: Vec<Chunk> = seq_region.render_chunks(&RenderPhase::NonDcDs);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk { start_tick: 100, end_tick: 200 });
  }

  #[test]  
  fn render_sequence_region_when_dcds_iter0() {
    let seq_region = SequenceRegion {
      tick_range: 100..200
    };
    let chunks: Vec<Chunk> = seq_region.render_chunks(&RenderPhase::DcDsIter0 { dc_ds_tick: 99 });
    assert_eq!(chunks.len(), 0);

    let chunks: Vec<Chunk> = seq_region.render_chunks(&RenderPhase::DcDsIter0 { dc_ds_tick: 100 });
    assert_eq!(chunks.len(), 0);

    let chunks: Vec<Chunk> = seq_region.render_chunks(&RenderPhase::DcDsIter0 { dc_ds_tick: 101 });
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk { start_tick: 100, end_tick: 101 });

    let chunks: Vec<Chunk> = seq_region.render_chunks(&RenderPhase::DcDsIter0 { dc_ds_tick: 199 });
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk { start_tick: 100, end_tick: 199 });

    let chunks: Vec<Chunk> = seq_region.render_chunks(&RenderPhase::DcDsIter0 { dc_ds_tick: 200 });
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk { start_tick: 100, end_tick: 200 });

    let chunks: Vec<Chunk> = seq_region.render_chunks(&RenderPhase::DcDsIter0 { dc_ds_tick: 201 });
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk { start_tick: 100, end_tick: 200 });
  }

  #[test]  
  fn render_sequence_region_when_dcds_iter1() -> Result<(), RenderRegionError> {
    let seq_region = SequenceRegion {
      tick_range: 100..200
    };
    let (global_repeat, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_segno(99)?
      .adding_dc(400, 100)?
      .adding_first_bar_len(30)?
      .build()?;
      
    let gr: crate::global_repeat::GlobalRepeat = global_repeat.unwrap();
    let rp = RenderPhase::DcDsIter1 { dc_ds_tick: gr.ds_dc().tick(), global_repeat: gr };
    let chunks = seq_region.render_chunks(&rp);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Chunk { start_tick: 100, end_tick: 200 });
    Ok(())
  }

  //   200      400
  // A | Fine B | D.C.
  // This is assumed auftakt.
  // A B
  #[test]
  fn dc_goes_to_fine() {
    let bars = vec![
      Bar::new(200, None, None, repeat_set!(Repeat::Fine)),
      Bar::new(400, None, None, repeat_set!(Repeat::Dc)),
    ];

    let (region, warnings) = render_region(Rhythm::new(1, 4), bars.iter()).unwrap();
    let chunks = region.to_chunks();
    assert_eq!(chunks.len(), 1);
  }
}
