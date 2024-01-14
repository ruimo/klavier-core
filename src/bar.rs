use std::fmt;

use enumset::{EnumSetType, EnumSet, enum_set};

use super::{key::Key, rhythm::Rhythm, note::TickError, have_start_tick::{HaveBaseStartTick, HaveStartTick}};

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarIndex {
    VI1, VI2, VI3, VI4,
}

#[derive(Debug)]
pub enum VarIndexError {
    InvalidIndex(u8),
}

impl VarIndex {
    pub fn from_value(value: u8) -> Result<VarIndex, VarIndexError> {
        match value {
            1 => Ok(VarIndex::VI1),
            2 => Ok(VarIndex::VI2),
            3 => Ok(VarIndex::VI3),
            4 => Ok(VarIndex::VI4),
            _ => Err(VarIndexError::InvalidIndex(value))
        }
    }

    pub fn value(self) -> u8 {
        match self {
            VarIndex::VI1 => 1,
            VarIndex::VI2 => 2,
            VarIndex::VI3 => 3,
            VarIndex::VI4 => 4,
        }
    }

    pub fn next(self) -> Result<VarIndex, VarIndexError> {
        Self::from_value(self.value() + 1)
    }

    pub fn prev(self) -> Result<VarIndex, VarIndexError> {
        Self::from_value(self.value() - 1)
    }

    pub fn to_repeat(self) -> Repeat {
        match self {
            Self::VI1 => Repeat::Var1,
            Self::VI2 => Repeat::Var2,
            Self::VI3 => Repeat::Var3,
            Self::VI4 => Repeat::Var4,
        }
    }
}

impl Into<VarIndex> for u8 {
    fn into(self) -> VarIndex {
        VarIndex::from_value(self).unwrap()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, EnumSetType)]
pub enum Repeat {
    Start,
    End,
    Dc,
    Fine,
    Ds,
    Segno,
    Coda,
    Var1,
    Var2,
    Var3,
    Var4,
}

impl fmt::Display for Repeat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Repeat::Start => "|:",
            Repeat::End => ":|",
            Repeat::Dc => "D.C.",
            Repeat::Fine => "Fine",
            Repeat::Ds => "D.S.",
            Repeat::Segno => "Segno",
            Repeat::Coda => "Coda",
            Repeat::Var1 => "Var1",
            Repeat::Var2 => "Var2",
            Repeat::Var3 => "Var3",
            Repeat::Var4 => "Var4",
        };

        write!(f, "{}", s)
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepeatSet {
    value: EnumSet<Repeat>,
}

impl RepeatSet {
    const ALL_REGION_BITS: EnumSet<Repeat> = enum_set!(Repeat::Var1 | Repeat::Var2 | Repeat::Var3 | Repeat::Var4);

    const START_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Dc | Repeat::Ds
        | Repeat::Var1 | Repeat::Var2 | Repeat::Var3 | Repeat::Var4
    );

    const END_DISLIKE: EnumSet<Repeat> = Self::ALL_REGION_BITS;

    const DC_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::Ds | Repeat::Segno
        | Repeat::Var1 | Repeat::Var2 | Repeat::Var3 | Repeat::Var4
    );

    const DS_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start
        | Repeat::Var1 | Repeat::Var2 | Repeat::Var3 | Repeat::Var4
    );

    const REGION1_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::End | Repeat::Dc | Repeat::Ds | Repeat::Segno | Repeat::Var2 | Repeat::Var3 | Repeat::Var4
    );

    const REGION2_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::End | Repeat::Dc | Repeat::Ds | Repeat::Segno | Repeat::Var1 | Repeat::Var3 | Repeat::Var4
    );

    const REGION3_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::End | Repeat::Dc | Repeat::Ds | Repeat::Segno | Repeat::Var1 | Repeat::Var2 | Repeat::Var4
    );

    const REGION4_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::End | Repeat::Dc | Repeat::Ds | Repeat::Segno | Repeat::Var1 | Repeat::Var2 | Repeat::Var3
    );

    pub const EMPTY: RepeatSet = Self { value: EnumSet::EMPTY };

    pub fn contains(self, r: Repeat) -> bool {
        self.value.contains(r)
    }

    const fn new(value: EnumSet<Repeat>) -> Self { Self { value } }

    #[inline]
    fn try_add_repeat(self, dislike: EnumSet<Repeat>, r: Repeat) -> Result<Self, EnumSet<Repeat>> {
        let dislike = dislike & self.value;
        if dislike.is_empty() {
            Ok(RepeatSet::new(self.value | r))
        } else {
            Err(dislike)
        }
    }

    pub fn try_add(self, r: Repeat) -> Result<Self, EnumSet<Repeat>> {
        match r {
            Repeat::Start => self.try_add_repeat(Self::START_DISLIKE, r),
            Repeat::End => self.try_add_repeat(Self::END_DISLIKE, r),
            Repeat::Dc => self.try_add_repeat(Self::DC_DISLIKE, r),
            Repeat::Fine => self.try_add_repeat(EnumSet::empty(), r),
            Repeat::Ds => self.try_add_repeat(Self::DS_DISLIKE, r),
            Repeat::Segno => self.try_add_repeat(EnumSet::empty(), r),
            Repeat::Coda => self.try_add_repeat(EnumSet::empty(), r),
            Repeat::Var1 => self.try_add_repeat(Self::REGION1_DISLIKE, r),
            Repeat::Var2 => self.try_add_repeat(Self::REGION2_DISLIKE, r),
            Repeat::Var3 => self.try_add_repeat(Self::REGION3_DISLIKE, r),
            Repeat::Var4 => self.try_add_repeat(Self::REGION4_DISLIKE, r),
        }
    }

    pub fn remove(self, r: Repeat) -> Self {
        let mut copy = self.clone();
        copy.value.remove(r);
        copy
    }

    pub fn region_index(self) -> Option<VarIndex> {
        if self.contains(Repeat::Var1) { Some(VarIndex::VI1) }
        else if self.contains(Repeat::Var2) { Some(VarIndex::VI2) }
        else if self.contains(Repeat::Var3) { Some(VarIndex::VI3) }
        else if self.contains(Repeat::Var4) { Some(VarIndex::VI4) }
        else { None }
    }

    pub fn remove_resions(self) -> Self {
        let mut copy = self.clone();
        copy.value.remove(Repeat::Var1);
        copy.value.remove(Repeat::Var2);
        copy.value.remove(Repeat::Var3);
        copy.value.remove(Repeat::Var4);
        copy
    }

    pub fn len(self) -> usize {
        self.value.len()
    }
}

impl Default for RepeatSet {
    fn default() -> Self {
        Self { value: Default::default() }
    }
}

#[macro_export]
macro_rules! repeat_set {
    () => {
        RepeatSet::EMPTY
    };

    ($e:expr) => {
        repeat_set!().try_add($e).unwrap()
    };

    ($e:expr, $($es:expr),+) => {
        repeat_set!($($es),+).try_add($e).unwrap()
    };
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    pub start_tick: u32,
    pub rhythm: Option<Rhythm>,
    pub key: Option<Key>,
    pub repeats: RepeatSet,
}

impl Bar {
    pub fn new(
        start_tick: u32,
        rhythm: Option<Rhythm>,
        key: Option<Key>,
        repeats: RepeatSet,
    ) -> Self {
        Self {
            start_tick, rhythm, key, repeats,
        }
    }

    pub fn drag(&self, tick_delta: i32) -> Self {
        Self {
             start_tick: ((self.start_tick as i64) + tick_delta as i64) as u32,
             ..*self
        }
    }

    pub fn with_tick_added(&self, tick_delta: i32) -> Result<Self, TickError> {
        let tick = (self.start_tick as i64) + tick_delta as i64;
        if tick < 0 {
            Err(TickError::Minus)
        } else {
            Ok(
                Self {
                    start_tick: tick as u32,
                    ..*self
                }
            )
        }
    }
}

impl HaveBaseStartTick for Bar {
    fn base_start_tick(&self) -> u32 {
        self.start_tick
    }
}

impl HaveStartTick for Bar {
    fn start_tick(&self) -> u32 {
        self.start_tick
    }
}

#[cfg(test)]
mod tests {
    use klavier_helper::nan_free_f32::NanFreeF32;
    use klavier_helper::store::Store;
    use klavier_helper::store::StoreEvent;
    use serde_json::Value;
    use serde_json::json;

    use crate::bar::Bar;
    use crate::rhythm::Rhythm;

    use super::Repeat;
    use super::RepeatSet;
    use enumset::enum_set;

    #[test]
    fn can_serialize_dc_fine() {
        let repeats = repeat_set!(Repeat::Dc);
        let json_str = serde_json::to_string(&repeats).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({ "value": 4 })
        );
    }

    #[test]
    fn can_deserialize_dc_fine() {
        let repeats: RepeatSet = serde_json::from_str(r#"{ "value": 4 }"#).unwrap();
        assert_eq!(repeats.len(), 1);
        assert_eq!(repeats.contains(Repeat::Dc), true);
    }

    #[test]
    fn can_serialize_end_or_region() {
        let repeats = repeat_set!(Repeat::Var1);
        let json_str = serde_json::to_string(&repeats).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({ "value": 128 })
        );
    }

    #[test]
    fn can_deserialize_end_or_region() {
        let repeats: RepeatSet = serde_json::from_str(r#"{ "value": 64 } "#).unwrap();
        assert_eq!(repeats, repeat_set!(Repeat::Coda));
    }

    #[test]
    fn can_serialize_repeat_start() {
        let repeats = repeat_set!();
        let json_str = serde_json::to_string(&repeats).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({ "value": 0})
        );
    }

    #[test]
    fn can_deserialize_repeat_start() {
        let repeats: RepeatSet = serde_json::from_str(r#"{ "value": 0 }"#).unwrap();
        assert_eq!(repeats, repeat_set!());
    }

    #[test]
    fn can_serialize_bar() {
        let json_str = serde_json::to_string(&
            Bar {
              start_tick: 123,
              key: None,
              rhythm: Some(Rhythm::new(3, 4)),
              repeats: repeat_set!(Repeat::End, Repeat::Start)
            }).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({
                "start_tick": 123,
                "repeats": { "value": 3},
                "key": null,
                "rhythm": {
                    "numerator": 3,
                    "denominator": "D4"
                }
            })
        );

    }

    #[test]
    fn range_empty() {
        let store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let (start, mut z) = store.range(NanFreeF32::from(0.0) .. NanFreeF32::from(100.0));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 0);
    }

    #[test]
    fn range_too_left() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, repeat_set!());
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, mut z) = store.range(NanFreeF32::from(0.0) .. NanFreeF32::from(1.0));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 0);
    }
    
    #[test]
    fn range_hit() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, repeat_set!());
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, mut z) = store.range(NanFreeF32::from(0.0) .. NanFreeF32::from(1.1));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 1);
        assert_eq!(z[0], (NanFreeF32::from(1.0), bar0));

        let (start, mut z) = store.range(NanFreeF32::from(1.0) .. NanFreeF32::from(1.1));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 1);
        assert_eq!(z[0], (NanFreeF32::from(1.0), bar0));
    }

    #[test]
    fn range_too_right() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, repeat_set!());
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, mut z) = store.range(NanFreeF32::from(1.1) .. NanFreeF32::from(2.0));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 0);
    }

    #[test]
    fn range_inclusive_end() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, repeat_set!());
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, mut z) = store.range(NanFreeF32::from(0.1) ..= NanFreeF32::from(1.0));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 1);
        assert_eq!(z[0], (NanFreeF32::from(1.0), bar0));
    }

    #[test]
    fn should_sorted() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(2, None, None, repeat_set!());
        let bar1 = Bar::new(10, None, None, repeat_set!());
        let bar2 = Bar::new(0, None, None, repeat_set!());
        store.add(2.0.into(), bar0, 0);
        store.add(10.0.into(), bar1, 0);
        store.add(0.0.into(), bar2, 0);

        let (start, mut z) = store.range(NanFreeF32::from(0.0) ..= NanFreeF32::from(10.0));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 3);
        assert_eq!(z[0], (NanFreeF32::from(0.0), bar2));
        assert_eq!(z[1], (NanFreeF32::from(2.0), bar0));
        assert_eq!(z[2], (NanFreeF32::from(10.0), bar1));
    }

    #[test]
    fn index() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(2, None, None, repeat_set!());
        let bar1 = Bar::new(10, None, None, repeat_set!());
        let bar2 = Bar::new(0, None, None, repeat_set!());
        store.add(2.0.into(), bar0, 0);
        store.add(10.0.into(), bar1, 0);
        store.add(0.0.into(), bar2, 0);

        assert_eq!(store.index(2.0.into()), Ok(1));
        assert_eq!(store.index(0.0.into()), Ok(0));
        assert_eq!(store.index(10.0.into()), Ok(2));
        
        assert_eq!(store.index(1.0.into()), Err(1));
    }

    #[test]
    fn range() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(2, None, None, repeat_set!());
        let bar1 = Bar::new(10, None, None, repeat_set!());
        let bar2 = Bar::new(0, None, None, repeat_set!());
        let bar3 = Bar::new(100, None, None, repeat_set!());
        store.add(2.0.into(), bar0, 0);
        store.add(10.0.into(), bar1, 0);
        store.add(0.0.into(), bar2, 0);
        store.add(100.0.into(), bar3, 0);

        let (start, z) = store.range(NanFreeF32::from(1.0) .. NanFreeF32::from(20.0));
        assert_eq!(start, 1);
        assert_eq!(z.len(), 2);
        assert_eq!(z[0], (NanFreeF32::from(2.0), bar0));
        assert_eq!(z[1], (NanFreeF32::from(10.0), bar1));

        let (start, z) = store.range(NanFreeF32::from(10.0) .. NanFreeF32::from(200.0));
        assert_eq!(start, 2);
        assert_eq!(z.len(), 2);
        assert_eq!(z[0], (NanFreeF32::from(10.0), bar1));
        assert_eq!(z[1], (NanFreeF32::from(100.0), bar3));

        let (start, z) = store.range(NanFreeF32::from(10.0) ..);
        assert_eq!(start, 2);
        assert_eq!(z.len(), 2);
        assert_eq!(z[0], (NanFreeF32::from(10.0), bar1));
        assert_eq!(z[1], (NanFreeF32::from(100.0), bar3));
    }

    #[test]
    fn observe() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(true);
        let bar0 = Bar::new(0, None, None, repeat_set!());
        store.add(NanFreeF32::from(0.0), bar0, 999);

        let events = store.events();
        assert_eq!(events.len(), 1);
        let e = &events[0];
        match e {
            StoreEvent::Added { added: s, metadata } => {
                assert_eq!(*s, bar0);
                assert_eq!(*metadata, 999);
            },
            _ => {
                panic!("Test failed.");
            }   
        }
    }

    #[test]
    fn find() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(2, None, None, repeat_set!());
        let bar1 = Bar::new(10, None, None, repeat_set!());
        let bar2 = Bar::new(0, None, None, repeat_set!());
        let bar3 = Bar::new(100, None, None, repeat_set!());
        store.add(2.0.into(), bar0, 0);
        store.add(10.0.into(), bar1, 0);
        store.add(0.0.into(), bar2, 0);
        store.add(100.0.into(), bar3, 0);
        assert_eq!(store.find(&2.0.into()).ok(), Some(1));
        assert_eq!(store.find(&1.0.into()).err(), Some(1));
    }

    #[test]
    fn update() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(2, None, None, repeat_set!());
        let bar1 = Bar::new(10, None, None, repeat_set!());
        let bar2 = Bar::new(0, None, None, repeat_set!());
        store.add(2.0.into(), bar0, 0);
        store.add(10.0.into(), bar1, 0);
        store.add(0.0.into(), bar2, 0);

        let bar3 = Bar::new(0, None, None, repeat_set!());
        store.update_at_idx(1, bar3, 0);
        assert_eq!(store[1], (2.0.into(), bar3));
    }

    #[test]
    fn as_ref() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(2, None, None, repeat_set!());
        store.add(2.0.into(), bar0, 0);

        let as_ref = store.as_ref();
        assert_eq!(as_ref.len(), 1);
        assert_eq!(as_ref[0].0, 2.0.into());
        assert_eq!(as_ref[0].1, bar0);
    }

    #[test]
    fn empty_repeat_set() {
        let set = repeat_set!();
        assert_eq!(set.contains(Repeat::Start), false);
        assert_eq!(set.contains(Repeat::End), false);
        assert_eq!(set.contains(Repeat::Dc), false);
        assert_eq!(set.contains(Repeat::Fine), false);
        assert_eq!(set.contains(Repeat::Ds), false);
        assert_eq!(set.contains(Repeat::Segno), false);
        assert_eq!(set.contains(Repeat::Var1), false);
        assert_eq!(set.contains(Repeat::Var2), false);
        assert_eq!(set.contains(Repeat::Var3), false);
        assert_eq!(set.contains(Repeat::Var4), false);
    }

    #[test]
    fn repeat_set_contains() {
        let set = repeat_set!(Repeat::Start, Repeat::End);
        assert_eq!(set.contains(Repeat::Start), true);
        assert_eq!(set.contains(Repeat::End), true);
        assert_eq!(set.contains(Repeat::Dc), false);
        assert_eq!(set.contains(Repeat::Fine), false);
        assert_eq!(set.contains(Repeat::Ds), false);
        assert_eq!(set.contains(Repeat::Segno), false);
        assert_eq!(set.contains(Repeat::Var1), false);
        assert_eq!(set.contains(Repeat::Var2), false);
        assert_eq!(set.contains(Repeat::Var3), false);
        assert_eq!(set.contains(Repeat::Var4), false);
    }

    #[test]
    fn cannot_dc_and_start() {
        let result = repeat_set!(Repeat::Dc).try_add(Repeat::Start);
        assert_eq!(result, Err(enum_set!(Repeat::Dc)));
    }
}
