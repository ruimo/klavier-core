use std::{collections::VecDeque};
use crate::{models::{Models}, project::ModelChangeMetadata};

#[derive(Clone, PartialEq, Debug, serde::Deserialize, serde::Serialize)]
pub enum Undo {
    Changed { added: Models, removed: Models, metadata: ModelChangeMetadata },
}

#[derive(Debug)]
pub struct UndoStore {
    store: VecDeque<Undo>,
    capacity: usize,
    index: usize,
    is_freezed: bool,
}

impl Default for UndoStore {
    fn default() -> Self {
        Self::new(100)
    }
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
    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, Undo> {
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

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
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

    use crate::{models::Models, note::Note, pitch::Pitch, solfa::Solfa, octave::Octave, sharp_flat::SharpFlat, duration::{Duration, Numerator, Denominator, Dots}, velocity::Velocity, trimmer::RateTrimmer, bar::{Bar, RepeatSet, Repeat}, undo::{UndoStore, Undo}, project::ModelChangeMetadata};
    
    fn test_models() -> [Models; 5] {
        let note0 = Rc::new(
            Note {
                base_start_tick: 123,
                pitch: Pitch::new(Solfa::C, Octave::Oct1, SharpFlat::Null),
                duration: Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
                base_velocity: Velocity::new(10),
                duration_trimmer: RateTrimmer::new(1.0, 0.5, 2.0, 1.5),
                ..Default::default()
            }
        );
        let note1 = Rc::new(
            Note {
                base_start_tick: 123,
                pitch: Pitch::new(Solfa::D, Octave::Oct1, SharpFlat::Null),
                duration: Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
                base_velocity: Velocity::new(10),
                duration_trimmer: RateTrimmer::new(1.0, 0.5, 2.0, 1.5),
                ..Default::default()
            }
        );
        let note2 = Rc::new(
            Note {
                base_start_tick: 234,
                pitch: Pitch::new(Solfa::E, Octave::Oct1, SharpFlat::Null),
                duration: Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
                base_velocity: Velocity::new(10),
                duration_trimmer: RateTrimmer::new(1.0, 0.5, 2.0, 1.5),
                ..Default::default()
            }
        );
        let note3 = Rc::new(
            Note {
                base_start_tick: 345,
                pitch: Pitch::new(Solfa::F, Octave::Oct1, SharpFlat::Null),
                duration: Duration::new(Numerator::Half, Denominator::from_value(2).unwrap(), Dots::ZERO),
                base_velocity: Velocity::new(10),
                duration_trimmer: RateTrimmer::new(1.0, 0.5, 2.0, 1.5),
                ..Default::default()
            }
        );

        let bar0 = Bar::new(123, None, None, RepeatSet::EMPTY);
        let bar1 = Bar::new(
            234, None, None, RepeatSet::EMPTY.try_add(Repeat::Dc).unwrap()
        );
        let bar2 = Bar::new(
            345, None, None, RepeatSet::EMPTY.try_add(Repeat::End).unwrap()
        );
        let bar3 = Bar::new(
            456, None, None, RepeatSet::EMPTY.try_add(Repeat::Start).unwrap()
        );
        let bar4 = Bar::new(
            567, None, None, RepeatSet::EMPTY.try_add(Repeat::Start).unwrap(),
        );

        [
          Models {
            notes: vec![(*note0).clone()], bars: vec![], tempos: vec![], dumpers: vec![], softs: vec![],
          },
          Models {
            notes: vec![], bars: vec![bar0], tempos: vec![], dumpers: vec![], softs: vec![],
          },
          Models {
            notes: vec![(*note1).clone()], bars: vec![bar1, bar2], tempos: vec![], dumpers: vec![], softs: vec![],
          },
          Models {
            notes: vec![(*note2).clone()], bars: vec![bar3], tempos: vec![], dumpers: vec![], softs: vec![],
          },
          Models {
            notes: vec![(*note3).clone()], bars: vec![bar4], tempos: vec![], dumpers: vec![], softs: vec![],
          },
        ]
    }

    #[test]
    fn should_be_dropped_when_capacity_reached() {
        let mut store = UndoStore::new(3);
        let models = test_models();
        assert!(! store.can_undo());
        assert_eq!(store.undo(), None);
        let metadata = ModelChangeMetadata::default();
        store.add(Undo::Changed { added: models[0].clone(), removed: Models::empty(), metadata });
        assert_eq!(store.len(), 1);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[0].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
        
        store.add(Undo::Changed { added: models[1].clone(), removed: Models::empty(), metadata });
        assert_eq!(store.len(), 2);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[1].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[0].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
        
        store.add(Undo::Changed { added: models[2].clone(), removed: Models::empty(), metadata });
        assert_eq!(store.len(), 3);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[2].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[1].clone(), removed: Models::empty(), metadata } ));
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[0].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
        
        store.add(Undo::Changed { added: models[3].clone(), removed: Models::empty(), metadata });
        assert_eq!(store.len(), 3);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[3].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[2].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[1].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
        
        store.add(Undo::Changed { added: models[4].clone(), removed: Models::empty(), metadata });
        assert_eq!(store.len(), 3);
        let mut z = store.iter();
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[4].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[3].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), Some(&Undo::Changed { added: models[2].clone(), removed: Models::empty(), metadata }));
        assert_eq!(z.next(), None);
        assert!(store.can_undo());
    }

    #[test]
    fn can_undo() {
        let mut store = UndoStore::new(3);
        let models = test_models();
        let metadata = ModelChangeMetadata::default();
        assert!(! store.can_undo());
        store.add(Undo::Changed { added: models[0].clone(), removed: Models::empty(), metadata });
        store.add(Undo::Changed { added: models[1].clone(), removed: Models::empty(), metadata });

        assert!(store.can_undo());
        assert_eq!(store.undo(), Some(&Undo::Changed { added: models[1].clone(), removed: Models::empty(), metadata }));
        assert_eq!(store.undo(), Some(&Undo::Changed { added: models[0].clone(), removed: Models::empty(), metadata }));
        assert_eq!(store.undo(), None);
        assert!(! store.can_undo());

        store.add(Undo::Changed { added: models[0].clone(), removed: Models::empty(), metadata });
        store.add(Undo::Changed { added: models[1].clone(), removed: Models::empty(), metadata });
        store.add(Undo::Changed { added: models[2].clone(), removed: Models::empty(), metadata });
        assert!(store.can_undo());
        assert_eq!(store.undo(), Some(&Undo::Changed { added: models[2].clone(), removed: Models::empty(), metadata }));
        store.add(Undo::Changed { added: models[3].clone(), removed: Models::empty(), metadata });
        assert_eq!(store.undo(), Some(&Undo::Changed { added: models[3].clone(), removed: Models::empty(), metadata }));
        assert_eq!(store.undo(), Some(&Undo::Changed { added: models[1].clone(), removed: Models::empty(), metadata }));
        assert_eq!(store.undo(), Some(&Undo::Changed { added: models[0].clone(), removed: Models::empty(), metadata }));
        assert_eq!(store.undo(), None);
        assert!(! store.can_undo());
    }
}

