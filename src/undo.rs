use std::{collections::{VecDeque, vec_deque::Iter}, rc::Rc};

use crate::{note::Note, bar::Bar, tempo::Tempo, ctrl_chg::CtrlChg, models::Models};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, PartialEq, Debug)]
pub struct ModelChanges {
    pub notes: Vec<(Rc<Note>, Rc<Note>)>,
    pub bars: Vec<(Bar, Bar)>,
    pub tempos: Vec<(Tempo, Tempo)>,
    pub dumpers: Vec<(CtrlChg, CtrlChg)>,
    pub softs: Vec<(CtrlChg, CtrlChg)>,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, PartialEq, Debug)]
pub enum Undo {
    Added { added: Models, removed: Models },
    Changed { changed: ModelChanges, removed: Models },
    Removed(Models),
}

#[derive(Debug)]
pub struct UndoStore {
    store: VecDeque<Undo>,
    capacity: usize,
    index: usize,
    is_freezed: bool,
}

impl UndoStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            store: VecDeque::new(),
            capacity,
            index: 0,
            is_freezed: false,
        }
    }
    
    #[allow(dead_code)]
    pub fn iter(&self) -> Iter<'_, Undo> {
        self.store.iter()
    }
    
    pub fn add(&mut self, undo: Undo) {
        if self.is_freezed { return; }
        if self.index != 0 {
            self.store = self.store.split_off(self.index);
            self.index = 0;
        }
        
        if self.capacity <= self.store.len() {
            self.store.pop_back();
        }
        self.store.push_front(undo);
    }
    
    pub fn undo(&mut self) -> Option<&Undo> {
        let ret = self.store.get(self.index);
        if ret.is_some() {
            self.index += 1;
        }
        ret
    }
    
    #[allow(dead_code)]
    pub fn redo(&mut self) -> Option<&Undo> {
        if self.index != 0 {
            self.index -= 1;
            self.store.get(self.index)
        } else {
            None
        }
    }
    
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.store.len()
    }
    
    pub fn can_undo(&self) -> bool {
        self.index < self.store.len()
    }
    
    #[allow(dead_code)]
    pub fn can_redo(&self) -> bool {
        self.index != 0
    }
    
    pub fn freeze(&mut self, is_freezed: bool){
        self.is_freezed = is_freezed;
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc};

    use crate::{models::Models, note::Note, pitch::Pitch, solfa::Solfa, octave::Octave, sharp_flat::SharpFlat, duration::{Duration, Numerator, Denominator, Dots}, velocity::Velocity, trimmer::{Trimmer, RateTrimmer}, bar::{Bar, DcFine, EndOrRegion, RepeatStart}, undo::{UndoStore, Undo}};
    
    fn test_models() -> [Models; 5] {
        let note0 = Rc::new(
            Note::new(
                123, // base_start_tick
                Pitch::new(Solfa::C, Octave::Oct1, SharpFlat::Null),
                Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
                false, // tie
                false, // tied
                Velocity::new(10), // base_velocity
                Trimmer::ZERO, // start_tick_trimmer
                RateTrimmer::new(1.0, 0.5, 2.0, 1.5), // duration_trimmer
                Trimmer::ZERO, // velocity_trimmer
            )
        );
        let note1 = Rc::new(
            Note::new(
                123, // base_start_tick
                Pitch::new(Solfa::D, Octave::Oct1, SharpFlat::Null),
                Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
                false, // tie
                false, // tied
                Velocity::new(10), // base_velocity
                Trimmer::ZERO, // start_tick_trimmer
                RateTrimmer::new(1.0, 0.5, 2.0, 1.5), // duration_trimmer
                Trimmer::ZERO, // velocity_trimmer
            )
        );
        let note2 = Rc::new(
            Note::new(
                234, // base_start_tick
                Pitch::new(Solfa::E, Octave::Oct1, SharpFlat::Null),
                Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
                false, // tie
                false, // tied
                Velocity::new(10), // base_velocity
                Trimmer::ZERO, // start_tick_trimmer
                RateTrimmer::new(1.0, 0.5, 2.0, 1.5), // duration_trimmer
                Trimmer::ZERO, // velocity_trimmer
            )
        );
        let note3 = Rc::new(
            Note::new(
                345, // base_start_tick
                Pitch::new(Solfa::F, Octave::Oct1, SharpFlat::Null),
                Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
                false, // tie
                false, // tied
                Velocity::new(10), // base_velocity
                Trimmer::ZERO, // start_tick_trimmer
                RateTrimmer::new(1.0, 0.5, 2.0, 1.5), // duration_trimmer
                Trimmer::ZERO, // velocity_trimmer
            )
        );

        let bar0 = Bar::new(
            123,
            None,
            None,
            DcFine::Null,
            EndOrRegion::Null,
            RepeatStart::Null
        );
        let bar1 = Bar::new(
            234,
            None,
            None,
            DcFine::Dc,
            EndOrRegion::Null,
            RepeatStart::Null
        );
        let bar2 = Bar::new(
            345,
            None,
            None,
            DcFine::Null,
            EndOrRegion::RepeatEnd,
            RepeatStart::Null
        );
        let bar3 = Bar::new(
            456,
            None,
            None,
            DcFine::Null,
            EndOrRegion::Null,
            RepeatStart::Start
        );
        let bar4 = Bar::new(
            567,
            None,
            None,
            DcFine::Null,
            EndOrRegion::Null,
            RepeatStart::Start
        );

        [
          Models {
            notes: vec![note0], bars: vec![], tempos: vec![], dumpers: vec![], softs: vec![],
          },
          Models {
            notes: vec![], bars: vec![bar0], tempos: vec![], dumpers: vec![], softs: vec![],
          },
          Models {
            notes: vec![note1], bars: vec![bar1, bar2], tempos: vec![], dumpers: vec![], softs: vec![],
          },
          Models {
            notes: vec![note2], bars: vec![bar3], tempos: vec![], dumpers: vec![], softs: vec![],
          },
          Models {
            notes: vec![note3], bars: vec![bar4], tempos: vec![], dumpers: vec![], softs: vec![],
          },
        ]
    }

    #[test]
    fn should_be_dropped_when_capacity_reached() {
        let mut store = UndoStore::new(3);
        let models = test_models();
        assert!(! store.can_undo());
        assert_eq!(store.undo(), None);
        store.add(Undo::Added { added: models[0].clone(), removed: Models::empty() });
        assert_eq!(store.len(), 1);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Added { added: models[0].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
        
        store.add(Undo::Added { added: models[1].clone(), removed: Models::empty() });
        assert_eq!(store.len(), 2);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Added { added: models[1].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), Some(&Undo::Added { added: models[0].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
        
        store.add(Undo::Added { added: models[2].clone(), removed: Models::empty() });
        assert_eq!(store.len(), 3);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Added { added: models[2].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), Some(&Undo::Added { added: models[1].clone(), removed: Models::empty() } ));
        assert_eq!(z.next(), Some(&Undo::Added { added: models[0].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
        
        store.add(Undo::Added { added: models[3].clone(), removed: Models::empty() });
        assert_eq!(store.len(), 3);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Added { added: models[3].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), Some(&Undo::Added { added: models[2].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), Some(&Undo::Added { added: models[1].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
        
        store.add(Undo::Added { added: models[4].clone(), removed: Models::empty() });
        assert_eq!(store.len(), 3);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Added { added: models[4].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), Some(&Undo::Added { added: models[3].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), Some(&Undo::Added { added: models[2].clone(), removed: Models::empty() }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
    }

    #[test]
    fn can_undo() {
        let mut store = UndoStore::new(3);
        let models = test_models();
        assert!(! store.can_undo());
        store.add(Undo::Added { added: models[0].clone(), removed: Models::empty() });
        store.add(Undo::Added { added: models[1].clone(), removed: Models::empty() });

        assert!(store.can_undo());
        assert_eq!(store.undo(), Some(&Undo::Added { added: models[1].clone(), removed: Models::empty() }));
        assert_eq!(store.undo(), Some(&Undo::Added { added: models[0].clone(), removed: Models::empty() }));
        assert_eq!(store.undo(), None);
        assert!(! store.can_undo());

        store.add(Undo::Added { added: models[0].clone(), removed: Models::empty() });
        store.add(Undo::Added { added: models[1].clone(), removed: Models::empty() });
        store.add(Undo::Added { added: models[2].clone(), removed: Models::empty() });
        assert!(store.can_undo());
        assert_eq!(store.undo(), Some(&Undo::Added { added: models[2].clone(), removed: Models::empty() }));
        store.add(Undo::Added { added: models[3].clone(), removed: Models::empty() });
        assert_eq!(store.undo(), Some(&Undo::Added { added: models[3].clone(), removed: Models::empty() }));
        assert_eq!(store.undo(), Some(&Undo::Added { added: models[1].clone(), removed: Models::empty() }));
        assert_eq!(store.undo(), Some(&Undo::Added { added: models[0].clone(), removed: Models::empty() }));
        assert_eq!(store.undo(), None);
        assert!(! store.can_undo());
    }
}

