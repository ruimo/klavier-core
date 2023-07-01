use std::{rc::Rc, ops::Index, collections::HashSet};

use super::{note::{Note}, have_start_tick::HaveBaseStartTick, duration::{Duration, Denominator}};
use gcd::Gcd;

pub fn tuplize(mut notes: Vec<Rc<Note>>) -> Vec<Rc<Note>> {
    if notes.is_empty() { return vec![]; }
    let notes = &mut notes;
    let (min_unit, sorted) = sort_by_start_tick(notes);

    if let Some(min_unit) = min_unit {
        let start_tick = notes[0].base_start_tick();
        let (total_tick, total_unit) = total_tick_unit(&sorted);
        let denominator = total_unit / min_unit;
        let total_tick = total_tick / denominator * 2;
        let denominator = Denominator::from_value(denominator as u8).unwrap();
        let mut u = 0;
        let mut ret = Vec::with_capacity(notes.len());

        for e in sorted.iter() {
            match e {
                TupleElem::None => {},
                TupleElem::Some { start_tick: _, notes, min_duration: _ } => {
                    for n in notes.iter() {
                        let mut note = (**n).clone();
                        note.base_start_tick = (start_tick + (total_tick * u / total_unit)) as u32;
                        note.duration = n.duration.with_denominator(denominator);
                        u += numerator_unit(note.duration);
                        ret.push(Rc::new(note));
                    }
                },
            }
        }

        ret
    } else {
        vec![]
    }
}
enum TupleElem {
    None,
    Some {
        start_tick: u32,
        notes: Vec<Rc<Note>>,
        min_duration: Duration,
    }
}
enum SingleOrDouble<T> {
    Single(T),
    Double(T, T),
}

impl Index<usize> for TupleElem {
    type Output = Rc<Note>;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            TupleElem::None => panic!("Index(={}) out of bounds.", index),
            TupleElem::Some { start_tick: _, notes, min_duration: _ } => &notes[index],
        }
    }
}

impl TupleElem {
    #[cfg(test)]
    fn len(&self) -> usize {
        match self {
            TupleElem::None => 0,
            TupleElem::Some { start_tick: _, notes, min_duration: _ } => notes.len(),
        }
    }

    #[cfg(test)]
    fn contains(&self, note: &Rc<Note>) -> bool {
        match self {
            TupleElem::None => false,
            TupleElem::Some { start_tick: _, notes, min_duration: _ } => notes.contains(note),
        }
    }

    fn tick_length(&self) -> u32 {
        match self {
            TupleElem::None => 0,
            TupleElem::Some { start_tick: _, notes: _, min_duration } => min_duration.tick_length(),
        }
    }

    fn unit(&self) -> Option<u32> {
        match self {
            TupleElem::None => None,
            TupleElem::Some { start_tick: _, notes: _, min_duration } => Some(numerator_unit(*min_duration)),
        }
    }

    fn add(self, note: Rc<Note>) -> SingleOrDouble<Self> {
        let duration = note.duration.with_denominator(Denominator::from_value(2).unwrap());
        match self {
            TupleElem::None =>
                SingleOrDouble::Single(TupleElem::Some {
                    start_tick: note.base_start_tick(),
                    min_duration: duration,
                    notes: vec![note],
                }),
            TupleElem::Some { start_tick, mut notes, min_duration } => {
                if start_tick != note.base_start_tick() {
                    SingleOrDouble::Double(
                        TupleElem::Some {
                            start_tick, notes, min_duration
                        },
                        TupleElem::Some {
                            start_tick: note.base_start_tick(),
                            min_duration: duration,
                            notes: vec![note],
                        }
                    )
                } else {
                    notes.push(note.clone());
                    SingleOrDouble::Single(
                        TupleElem::Some {
                            start_tick,
                            min_duration: min_duration.min(duration),
                            notes,
                        }
                    )
                }
            }
        }
    }
}

// 256th note = 1
// 128th = 2
// 64th = 4
// 32th = 8
// 16th = 16
// 8th = 32
// Quarter = 64
// Half = 128
// Whole = 256
#[inline]
fn numerator_unit(dur: Duration) -> u32 {
    let len: u32 = 1 << (8 - dur.numerator.ord());
    len + (len - (len >> dur.dots.value()))
}

#[inline]
fn gcd_units(units: HashSet<u32>) -> Option<u32> {
    units.iter().map(|p| *p).reduce(|u0, u1| u0.gcd(u1))
}

fn sort_by_start_tick(notes: &mut [Rc<Note>]) -> (Option<u32>, Vec<TupleElem>) {
    if notes.is_empty() { return (None, vec![]) }

    notes.sort_by(|note0, note1| note0.base_start_tick().cmp(&note1.base_start_tick()));
    let mut ret: Vec<TupleElem> = vec![];
    let mut cur = TupleElem::None;
    let mut numerator_units = HashSet::new();

    for note in notes.iter() {
        numerator_units.insert(numerator_unit(note.duration) as u32);
        match cur.add(note.clone()) {
            SingleOrDouble::Single(e) => {
                cur = e;
            },
            SingleOrDouble::Double(e0, e1) => {
                ret.push(e0);
                cur = e1;
            },
        }
    }

    ret.push(cur);
    (
        gcd_units(numerator_units),
        ret
    )
}

fn total_tick_unit(elements: &Vec<TupleElem>) -> (u32, u32) {
    let mut tick = 0;
    let mut unit: u32 = 0;

    for e in elements.iter() {
        tick += e.tick_length();
        unit += e.unit().unwrap_or(0);
    }

    (tick, unit)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use crate::{note::Note, pitch::Pitch, solfa::Solfa, octave::Octave, sharp_flat::SharpFlat, duration::{Duration, Numerator, Denominator, Dots}, trimmer::{Trimmer, RateTrimmer}, velocity::Velocity};
    use super::{numerator_unit, tuplize};

    #[test]
    fn sort_by_start_tick() {
        let note0 = Rc::new(Note::new(
            0,
            Pitch::new(Solfa::A, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false, Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        ));

        let note1 = Rc::new(Note::new(
            0,
            Pitch::new(Solfa::A, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false, Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        ));

        let note2 = Rc::new(Note::new(
            100,
            Pitch::new(Solfa::A, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false, Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        ));

        let mut notes: [Rc<Note>; 3] = [note2.clone(), note1.clone(), note0.clone()];
        let (min_unit, sorted) = super::sort_by_start_tick(&mut notes);

        assert_eq!(min_unit.unwrap(), 64);
        assert_eq!(sorted.len(), 2);
        
        let g0 = &sorted[0];
        assert_eq!(g0.len(), 2);
        assert!(g0.contains(&note0));
        assert!(g0.contains(&note1));

        assert_eq!(sorted[1][0], note2);
    }

    #[test]
    fn unit() {
        assert_eq!(
            2,
            numerator_unit(
                Duration::new(Numerator::N128th, Denominator::from_value(2).unwrap(), Dots::ZERO)
            )
        );
        assert_eq!(
            2,
            numerator_unit(
                Duration::new(Numerator::N128th, Denominator::from_value(5).unwrap(), Dots::ZERO)
            )
        );
        assert_eq!(
            3,
            numerator_unit(
                Duration::new(Numerator::N128th, Denominator::from_value(2).unwrap(), Dots::ONE)
            )
        );
        assert_eq!(
            6,
            numerator_unit(
                Duration::new(Numerator::N64th, Denominator::from_value(2).unwrap(), Dots::ONE)
            )
        );
    }

    fn note(tick: u32, pitch: Pitch, duration: Duration) -> Rc<Note> {
        Rc::new(Note::new(
            tick, pitch, duration,
            false, false, Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        ))
    }

    #[test]
    fn tuplize_3_eigth() {
        let note0 = note(
            10,
            Pitch::new(Solfa::A, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO)
        );
        let note1 = note(
            20,
            Pitch::new(Solfa::A, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO)
        );
        let note2 = note(
            50,
            Pitch::new(Solfa::A, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO)
        );

        let result = tuplize(vec![note0, note1, note2]);
        assert_eq!(result[0].base_start_tick, 10);
        assert_eq!(result[0].duration, Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));

        assert_eq!(result[1].base_start_tick, 10 + 80);
        assert_eq!(result[1].duration, Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));

        assert_eq!(result[2].base_start_tick, 10 + 80 * 2);
        assert_eq!(result[2].duration, Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));
    }

    #[test]
    fn tuplize_3_eigth_again() {
        let note0 = note(
            0,
            Pitch::new(Solfa::A, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO)
        );
        let note1 = note(
            80,
            Pitch::new(Solfa::A, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO)
        );
        let note2 = note(
            160,
            Pitch::new(Solfa::A, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO)
        );

        let result = tuplize(vec![note0, note1, note2]);
        assert_eq!(result[0].base_start_tick, 0);
        assert_eq!(result[0].duration, Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));

        assert_eq!(result[1].base_start_tick, 80);
        assert_eq!(result[1].duration, Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));

        assert_eq!(result[2].base_start_tick, 160);
        assert_eq!(result[2].duration, Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));
    }
}