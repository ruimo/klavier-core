use error_stack::{Report, IntoReport};
use interval::{IntervalSet, interval_set::ToIntervalSet};
use crate::{rhythm::Rhythm, repeat::RenderRegionError, bar::{Bar, Repeat}, have_start_tick::HaveBaseStartTick};

/// Coda marker positions in the score.
///
/// A coda is a concluding section of a piece, marked with the coda sign (⊕).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Coda {
  /// Single coda marker (orphan, will generate a warning).
  One(u32),
  /// Two coda markers (from and to positions).
  Two { from_tick: u32, to_tick: u32 },
}

/// D.C. (Da Capo) or D.S. (Dal Segno) repeat instruction.
///
/// These are navigation instructions that tell the performer to jump to
/// a different location in the score.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DsDc {
  /// Da Capo - return to the beginning.
  Dc { tick: u32, len: u32 },
  /// Dal Segno - return to the segno sign.
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

/// Warnings generated during repeat structure rendering.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RenderRegionWarning {
  /// Both Segno and D.C. were found (Segno will be used, D.C. ignored).
  SegnoAndDcFound { segno_tick: u32, dc_tick: u32 },
  /// A single Coda marker was found without a matching pair.
  OrphanCodaFound { coda_tick: u32 },
}

/// Global repeat structure for a musical piece.
///
/// Represents the overall repeat structure including D.C./D.S., Fine, Segno,
/// and Coda markers. This structure is used to determine the playback order
/// of musical sections.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GlobalRepeat {
  /// The D.C. or D.S. instruction.
  ds_dc: DsDc,
  /// Optional Fine (end) marker position.
  fine: Option<u32>,
  /// Segno (target for D.S.) marker position.
  segno: u32,
  /// Optional Coda jump positions [from, to].
  coda: Option<[u32; 2]>,
  /// Interval set for the first iteration.
  iter1_interval_set: IntervalSet<u32>,
}

impl GlobalRepeat {
  /// Returns the segno marker position.
  pub fn segno(&self) -> u32 {
    self.segno
  }

  /// Returns the D.C. or D.S. instruction.
  pub fn ds_dc(&self) -> DsDc {
    self.ds_dc
  }

  /// Returns the interval set for the first iteration.
  pub fn iter1_interval_set(&self) -> &IntervalSet<u32> {
    &self.iter1_interval_set
  }
}

/// Builder for constructing a `GlobalRepeat` structure.
///
/// This builder accumulates repeat markers as bars are processed and
/// validates the repeat structure before building the final `GlobalRepeat`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GlobalRepeatBuilder {
  /// The D.C. or D.S. instruction.
  pub ds_dc: Option<DsDc>,
  /// Optional Fine marker position.
  pub fine: Option<u32>,
  /// Optional Segno marker position.
  pub segno: Option<u32>,
  /// Optional Coda marker(s).
  pub coda: Option<Coda>,
  /// Length of the first bar (for auftakt/pickup measures).
  pub first_bar_len: Option<u32>,
  /// The top-level rhythm (time signature).
  pub top_rhythm: Rhythm,
  /// Accumulated warnings during building.
  pub warnings: Vec<RenderRegionWarning>,
  /// Previous bar's tick position.
  pub prev_bar_tick: Option<u32>,
}

impl GlobalRepeatBuilder {
  /// Creates a new builder with the given time signature.
  ///
  /// # Arguments
  ///
  /// * `tune_rhythm` - The initial time signature of the piece.
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

  pub fn adding_dc(mut self, dc_loc: u32, dc_bar_len: u32) -> Result<Self, Report<RenderRegionError>> {
    match self.ds_dc {
        None => {
          self.ds_dc = Some(DsDc::Dc { tick: dc_loc, len: dc_bar_len });
          Ok(self)
        }
        Some(DsDc::Dc { tick: prev_tick, len: _ }) =>
          Err(IntoReport::into_report(RenderRegionError::DuplicatedDsDc { tick: [prev_tick, dc_loc] })),
        Some(DsDc::Ds { tick: prev_tick }) =>
          Err(IntoReport::into_report(RenderRegionError::DuplicatedDsDc { tick: [prev_tick, dc_loc] })),
      }
  }

  fn adding_ds(mut self, tick: u32) -> Result<Self, Report<RenderRegionError>> {
    match self.ds_dc {
        None => {
          self.ds_dc = Some(DsDc::Ds { tick });
          Ok(self)
        }
        Some(DsDc::Dc { tick: prev_tick, len: _ }) =>
          Err(IntoReport::into_report(RenderRegionError::DuplicatedDsDc { tick: [prev_tick, tick] })),
        Some(DsDc::Ds{ tick: prev_tick } ) =>
          Err(IntoReport::into_report(RenderRegionError::DuplicatedDsDc { tick: [prev_tick, tick] })),
      }
  }

  fn adding_fine(mut self, tick: u32) -> Result<Self, Report<RenderRegionError>> {
    match self.fine {
        Some(prev_tick) =>
          Err(IntoReport::into_report(RenderRegionError::DuplicatedFine { tick: [prev_tick, tick] })),
        None => {
          self.fine = Some(tick);
          Ok(self)
        }
    }
  }

  pub fn adding_segno(mut self, tick: u32) -> Result<Self, Report<RenderRegionError>> {
    match self.segno {
      Some(prev_tick) =>
        Err(IntoReport::into_report(RenderRegionError::DuplicatedSegno { tick: [prev_tick, tick] })),
      None => {
        self.segno = Some(tick);
        Ok(self)
      }
    }
  }

  fn adding_coda(mut self, tick: u32) -> Result<Self, Report<RenderRegionError>> {
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
        Err(IntoReport::into_report(RenderRegionError::MoreThanTwoCodas { tick: [from_tick, to_tick, tick] }))
    }
  }

  pub fn adding_first_bar_len(mut self, first_bar_len: u32) -> Result<Self, RenderRegionError> {
    self.first_bar_len = Some(first_bar_len);
    Ok(self)
  }

  pub fn on_bar(mut self, bar: &Bar) -> Result<Self, Report<RenderRegionError>> {
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

  fn check_coda_pos(coda_from: u32, coda_to: u32, fine: Option<u32>) -> Result<(), Report<RenderRegionError>> {
    if let Some(fine) = fine {
      if fine < coda_to {
        Err(IntoReport::into_report(RenderRegionError::CodaAfterFine { coda_from, coda_to, fine }))
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

  pub fn build(self) -> Result<(Option<GlobalRepeat>, Vec<RenderRegionWarning>), Report<RenderRegionError>> {
    let mut warnings = self.warnings;

    match self.ds_dc {
      None => Ok((None, warnings)),
      Some(ds_dc) => {
        match ds_dc {
          DsDc::Dc { tick, len } => {
            if let Some(first_bar_len) = self.first_bar_len {
              let segno = if let Some(segno_tick) = self.segno {
                warnings.push(RenderRegionWarning::SegnoAndDcFound { segno_tick, dc_tick: tick });
                segno_tick
              } else {
                let rhythm_tick_len = self.top_rhythm.tick_len();
                if len + first_bar_len == rhythm_tick_len || len == rhythm_tick_len && first_bar_len == rhythm_tick_len{
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
              None => return Err(IntoReport::into_report(RenderRegionError::NoSegnoForDs { ds_tick: tick }))
            };
            let coda = match self.coda {
              None => None,
              Some(Coda::One(tick_tick)) => {
                return Err(IntoReport::into_report(RenderRegionError::OnlyOneCoda { tick: tick_tick }))
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
    use error_stack::Report;
    use interval::interval_set::*;
    use crate::{rhythm::Rhythm, repeat::RenderRegionError};
    use super::GlobalRepeatBuilder;

  #[test]
  fn dc_without_fine() -> Result<(), Report<RenderRegionError>> {
    let (gr, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_dc(4000, 240 * 4)?
      .adding_first_bar_len(240 * 4)?
      .build()?;

    let ranges = gr.as_ref().unwrap().iter1_interval_set();
    assert_eq!(ranges, &vec![(0, u32::MAX - 1)].to_interval_set());

    Ok(())
  }

  #[test]
  fn dc_with_fine() -> Result<(), Report<RenderRegionError>> {
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
  fn dc_with_fine_auftakt() -> Result<(), Report<RenderRegionError>> {
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
  fn dc_with_coda_fine_auftakt() -> Result<(), Report<RenderRegionError>> {
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
  fn ds_without_fine() -> Result<(), Report<RenderRegionError>> {
    let (gr, _warn) = GlobalRepeatBuilder::new(Rhythm::new(4, 4))
      .adding_ds(4000)?
      .adding_segno(100)?
      .build()?;

    let ranges = gr.as_ref().unwrap().iter1_interval_set();
    assert_eq!(ranges, &vec![(100, u32::MAX - 1)].to_interval_set());

    Ok(())
  }

  #[test]
  fn ds_with_fine() -> Result<(), Report<RenderRegionError>> {
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
  fn ds_with_coda_fine() -> Result<(), Report<RenderRegionError>> {
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