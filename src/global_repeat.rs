use error_stack::report;
use interval::{IntervalSet, interval_set::ToIntervalSet};
use error_stack::Result;
use crate::{rhythm::Rhythm, repeat::RenderRegionError, bar::{Bar, Repeat}, have_start_tick::HaveBaseStartTick};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Coda {
  One(u32),
  Two { from_tick: u32, to_tick: u32 },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DsDc {
  Dc { tick: u32, len: u32 },
  Ds { tick: u32 },
}

impl DsDc {
  pub fn tick(self) -> u32 {
    match self {
      DsDc::Dc { tick, len: _ } => tick,
      DsDc::Ds { tick } => tick,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RenderRegionWarning {
  SegnoAndDcFound { segno_tick: u32, dc_tick: u32 },
  OrphanCodaFound { coda_tick: u32 },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GlobalRepeat {
  ds_dc: DsDc,
  fine: Option<u32>,
  segno: u32,
  coda: Option<[u32; 2]>,
  iter1_interval_set: IntervalSet<u32>,
}

impl GlobalRepeat {
  pub fn segno(&self) -> u32 {
    self.segno
  }

  pub fn ds_dc(&self) -> DsDc {
    self.ds_dc
  }

  pub fn iter1_interval_set(&self) -> &IntervalSet<u32> {
    &self.iter1_interval_set
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GlobalRepeatBuilder {
  pub ds_dc: Option<DsDc>,
  pub fine: Option<u32>,
  pub segno: Option<u32>,
  pub coda: Option<Coda>,
  pub first_bar_len: Option<u32>,
  pub top_rhythm: Rhythm,
  pub warnings: Vec<RenderRegionWarning>,
  pub prev_bar_tick: Option<u32>,
}

impl GlobalRepeatBuilder {
  pub fn new(tune_rhythm: Rhythm) -> Self {
    Self {
      ds_dc: None,
      fine: None,
      segno: None,
      coda: None,
      first_bar_len: None,
      top_rhythm: tune_rhythm,
      warnings: vec![],
      prev_bar_tick: None,
    }
  }

  pub fn adding_dc(mut self, dc_loc: u32, dc_bar_len: u32) -> Result<Self, RenderRegionError> {
    match self.ds_dc {
        None => {
          self.ds_dc = Some(DsDc::Dc { tick: dc_loc, len: dc_bar_len });
          Ok(self)
        }
        Some(DsDc::Dc { tick: prev_tick, len: _ }) =>
          Err(report!(RenderRegionError::DuplicatedDsDc { tick: [prev_tick, dc_loc] })),
        Some(DsDc::Ds { tick: prev_tick }) =>
          Err(report!(RenderRegionError::DuplicatedDsDc { tick: [prev_tick, dc_loc] })),
      }
  }

  fn adding_ds(mut self, tick: u32) -> Result<Self, RenderRegionError> {
    match self.ds_dc {
        None => {
          self.ds_dc = Some(DsDc::Ds { tick });
          Ok(self)
        }
        Some(DsDc::Dc { tick: prev_tick, len: _ }) =>
          Err(report!(RenderRegionError::DuplicatedDsDc { tick: [prev_tick, tick] })),
        Some(DsDc::Ds{ tick: prev_tick } ) =>
          Err(report!(RenderRegionError::DuplicatedDsDc { tick: [prev_tick, tick] })),
      }
  }

  fn adding_fine(mut self, tick: u32) -> Result<Self, RenderRegionError> {
    match self.fine {
        Some(prev_tick) =>
          Err(report!(RenderRegionError::DuplicatedFine { tick: [prev_tick, tick] })),
        None => {
          self.fine = Some(tick);
          Ok(self)
        }
    }
  }

  pub fn adding_segno(mut self, tick: u32) -> Result<Self, RenderRegionError> {
    match self.segno {
      Some(prev_tick) =>
        Err(report!(RenderRegionError::DuplicatedSegno { tick: [prev_tick, tick] })),
      None => {
        self.segno = Some(tick);
        Ok(self)
      }
    }
  }

  fn adding_coda(mut self, tick: u32) -> Result<Self, RenderRegionError> {
    match self.coda {
      None => {
        self.coda = Some(Coda::One(tick));
        Ok(self)
      }
      Some(Coda::One(prev_tick)) => {
        self.coda = Some(Coda::Two { from_tick: prev_tick, to_tick: tick });
        Ok(self)
      }
      Some(Coda::Two { from_tick, to_tick }) =>
        return Err(report!(RenderRegionError::MoreThanTwoCodas { tick: [from_tick, to_tick, tick] }))
    }
  }

  pub fn adding_first_bar_len(mut self, first_bar_len: u32) -> Result<Self, RenderRegionError> {
    self.first_bar_len = Some(first_bar_len);
    Ok(self)
  }

  pub fn on_bar(mut self, bar: &Bar) -> Result<Self, RenderRegionError> {
    let repeats = bar.repeats;
    let tick = bar.base_start_tick();

    if self.first_bar_len.is_none() {
      if bar.base_start_tick() == 0 {
        if let Some(rhythm) = bar.rhythm {
          self.top_rhythm = rhythm;
        }
      } else {
        self.first_bar_len = Some(bar.base_start_tick());
      }
    }

    if repeats.contains(Repeat::Dc) {
      match self.prev_bar_tick {
        Some(prev_bar_tick) => {
          let dc_bar_len = tick - prev_bar_tick;
          self = self.adding_dc(tick, dc_bar_len)?;
        }
        None => {
          self = self.adding_dc(tick, 0)?;
        }
      }
    }

    self = if repeats.contains(Repeat::Ds) { self.adding_ds(tick)? } else { self };
    self = if repeats.contains(Repeat::Fine) { self.adding_fine(tick)? } else { self };
    self = if repeats.contains(Repeat::Segno) { self.adding_segno(tick)? } else { self };
    self = if repeats.contains(Repeat::Coda) { self.adding_coda(tick)? } else { self };
    
    self.prev_bar_tick = Some(tick);
    Ok(self)
  }

  fn check_coda_pos(coda_from: u32, coda_to: u32, fine: Option<u32>) -> Result<(), RenderRegionError> {
    if let Some(fine) = fine {
      if fine < coda_to {
        Err(report!(RenderRegionError::CodaAfterFine { coda_from, coda_to, fine }))
      } else {
        Ok(())
      }
    } else {
      Ok(())
    }
  }

  fn to_interval_set(start_tick: u32, fine: Option<u32>, coda: Option<[u32; 2]>) -> IntervalSet<u32> {
    let end_tick = fine.unwrap_or(u32::MAX);
    match coda {
        Some([coda_from, coda_to]) => vec![
          (start_tick, coda_from - 1), (coda_to, end_tick - 1)
        ],
        None => {
          if end_tick <= start_tick {
            vec![]
          } else {
            vec![(start_tick, end_tick - 1)]
          }
        }
    }.to_interval_set()
  }

  pub fn build(self) -> Result<(Option<GlobalRepeat>, Vec<RenderRegionWarning>), RenderRegionError> {
    let mut warnings = self.warnings;

    match self.ds_dc {
      None => return Ok((None, warnings)),
      Some(ds_dc) => {
        match ds_dc {
          DsDc::Dc { tick, len } => {
            if let Some(first_bar_len) = self.first_bar_len {
              let segno = if let Some(segno_tick) = self.segno {
                warnings.push(RenderRegionWarning::SegnoAndDcFound { segno_tick, dc_tick: tick });
                segno_tick
              } else {
                let rhythm_tick_len = self.top_rhythm.tick_len();
                if len + first_bar_len == rhythm_tick_len {
                  0
                } else if len == rhythm_tick_len && first_bar_len == rhythm_tick_len {
                  0
                } else {
                  first_bar_len
                }
              };
              let coda: Option<[u32; 2]> = match self.coda {
                None => None,
                Some(Coda::One(orphan_coda)) => {
                  warnings.push(RenderRegionWarning::OrphanCodaFound { coda_tick: orphan_coda });
                  None
                },
                Some(Coda::Two { from_tick, to_tick }) => {
                  Self::check_coda_pos(from_tick, to_tick, self.fine)?;
                  Some([from_tick, to_tick])
                }
              };
  
              Ok((
                Some(
                  GlobalRepeat {
                    ds_dc, fine: self.fine, segno, coda,
                    iter1_interval_set: Self::to_interval_set(segno, self.fine, coda)
                  }),
                warnings
              ))
            } else {
              Ok((None, warnings))
            }
          }
          DsDc::Ds { tick } => {
            let segno_tick = match self.segno {
              Some(segno_tick) => segno_tick,
              None => return Err(report!(RenderRegionError::NoSegnoForDs { ds_tick: tick }))
            };
            let coda = match self.coda {
              None => None,
              Some(Coda::One(tick_tick)) => {
                return Err(report!(RenderRegionError::OnlyOneCoda { tick: tick_tick }))
              }
              Some(Coda::Two { from_tick, to_tick }) => {
                Self::check_coda_pos(from_tick, to_tick, self.fine)?;
                Some([from_tick, to_tick])
              }
            };
            match self.fine {
              None => Ok((
                Some(
                  GlobalRepeat {
                    ds_dc, segno: segno_tick, fine: None, coda,
                    iter1_interval_set: Self::to_interval_set(segno_tick, self.fine, coda)
                  }
                ),
                warnings
              )),
              Some(fine_tick) => Ok((
                Some(
                  GlobalRepeat {
                    ds_dc, segno: segno_tick, fine: Some(fine_tick), coda,
                    iter1_interval_set: Self::to_interval_set(segno_tick, self.fine, coda)
                  }
                ),
                warnings
              ))
            }
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
    use error_stack::Result;
    use interval::interval_set::*;
    use crate::{rhythm::Rhythm, repeat::RenderRegionError};
    use super::GlobalRepeatBuilder;

  #[test]
  fn dc_without_fine() -> Result<(), RenderRegionError> {
    let (gr, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_dc(4000, 240 * 4)?
      .adding_first_bar_len(240 * 4)?
      .build()?;

    let ranges = gr.as_ref().unwrap().iter1_interval_set();
    assert_eq!(ranges, &vec![(0, u32::MAX - 1)].to_interval_set());

    Ok(())
  }

  #[test]
  fn dc_with_fine() -> Result<(), RenderRegionError> {
    let (gr, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_dc(8000, 240 * 4)?
      .adding_fine(4000)?
      .adding_first_bar_len(240 * 4)?
      .build()?;

    let ranges = gr.as_ref().unwrap().iter1_interval_set();
    assert_eq!(ranges, &vec![(0, 4000 - 1)].to_interval_set());

    Ok(())
  }

  #[test]
  fn dc_with_fine_auftakt() -> Result<(), RenderRegionError> {
    let (gr, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_dc(4000, 240 * 4)?
      .adding_fine(2000)?
      .adding_first_bar_len(240)?
      .build()?;

    let ranges = gr.as_ref().unwrap().iter1_interval_set();
    assert_eq!(ranges, &vec![(240, 2000 - 1)].to_interval_set());

    Ok(())
  }

  #[test]
  fn dc_with_coda_fine_auftakt() -> Result<(), RenderRegionError> {
    let (gr, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_dc(4000, 240 * 4)?
      .adding_coda(1000)?
      .adding_coda(5000)?
      .adding_fine(8000)?
      .adding_first_bar_len(240)?
      .build()?;

    let ranges = gr.as_ref().unwrap().iter1_interval_set();
    assert_eq!(ranges, &vec![(240, 1000 - 1), (5000, 8000 - 1)].to_interval_set());

    Ok(())
  }

  #[test]
  fn ds_without_fine() -> Result<(), RenderRegionError> {
    let (gr, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_ds(4000)?
      .adding_segno(100)?
      .build()?;

    let ranges = gr.as_ref().unwrap().iter1_interval_set();
    assert_eq!(ranges, &vec![(100, u32::MAX - 1)].to_interval_set());

    Ok(())
  }

  #[test]
  fn ds_with_fine() -> Result<(), RenderRegionError> {
    let (gr, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_ds(8000)?
      .adding_fine(4000)?
      .adding_segno(100)?
      .build()?;

    let ranges = gr.as_ref().unwrap().iter1_interval_set();
    assert_eq!(ranges, &vec![(100, 4000 - 1)].to_interval_set());

    Ok(())
  }

  #[test]
  fn ds_with_coda_fine() -> Result<(), RenderRegionError> {
    let (gr, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_ds(4000)?
      .adding_coda(1000)?
      .adding_coda(5000)?
      .adding_fine(8000)?
      .adding_segno(240)?
      .build()?;

    let ranges = gr.as_ref().unwrap().iter1_interval_set();
    assert_eq!(ranges, &vec![(240, 1000 - 1), (5000, 8000 - 1)].to_interval_set());

    Ok(())
  }

}