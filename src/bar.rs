use std::fmt;

use enumset::{EnumSetType, EnumSet, enum_set};

use super::{key::Key, rhythm::Rhythm, note::TickError, have_start_tick::{HaveBaseStartTick, HaveStartTick}};

/// Index for variation endings in repeat structures.
///
/// Represents the variation number (1-4) used in repeat endings like "1." and "2."
/// in musical notation.
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarIndex {
    /// First variation ending
    VI1,
    /// Second variation ending
    VI2,
    /// Third variation ending
    VI3,
    /// Fourth variation ending
    VI4,
}

/// Error type for invalid variation index values.
#[derive(Debug)]
pub enum VarIndexError {
    /// The provided index value is not in the valid range (1-4).
    InvalidIndex(u8),
}

impl VarIndex {
    /// Creates a `VarIndex` from a numeric value (1-4).
    ///
    /// # Arguments
    ///
    /// * `value` - The variation index number (must be 1, 2, 3, or 4).
    ///
    /// # Returns
    ///
    /// - `Ok(VarIndex)` - The corresponding variation index.
    /// - `Err(VarIndexError::InvalidIndex)` - If the value is not in the range 1-4.
    pub fn from_value(value: u8) -> Result<VarIndex, VarIndexError> {
        match value {
            1 => Ok(VarIndex::VI1),
            2 => Ok(VarIndex::VI2),
            3 => Ok(VarIndex::VI3),
            4 => Ok(VarIndex::VI4),
            _ => Err(VarIndexError::InvalidIndex(value))
        }
    }

    /// Returns the numeric value (1-4) of this variation index.
    pub fn value(self) -> u8 {
        match self {
            VarIndex::VI1 => 1,
            VarIndex::VI2 => 2,
            VarIndex::VI3 => 3,
            VarIndex::VI4 => 4,
        }
    }

    /// Returns the next variation index.
    ///
    /// # Returns
    ///
    /// - `Ok(VarIndex)` - The next variation index.
    /// - `Err(VarIndexError)` - If this is already the last variation (VI4).
    pub fn next(self) -> Result<VarIndex, VarIndexError> {
        Self::from_value(self.value() + 1)
    }

    /// Returns the previous variation index.
    ///
    /// # Returns
    ///
    /// - `Ok(VarIndex)` - The previous variation index.
    /// - `Err(VarIndexError)` - If this is already the first variation (VI1).
    pub fn prev(self) -> Result<VarIndex, VarIndexError> {
        Self::from_value(self.value() - 1)
    }

    /// Converts this variation index to the corresponding `Repeat` variant.
    pub fn to_repeat(self) -> Repeat {
        match self {
            Self::VI1 => Repeat::Var1,
            Self::VI2 => Repeat::Var2,
            Self::VI3 => Repeat::Var3,
            Self::VI4 => Repeat::Var4,
        }
    }
}

impl From<u8> for VarIndex {
    fn from(value: u8) -> Self {
        VarIndex::from_value(value).unwrap()
    }
}

/// Repeat and navigation symbols used in musical notation.
///
/// These symbols control the flow of music playback, including repeats,
/// jumps, and variation endings.
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, EnumSetType)]
pub enum Repeat {
    /// Repeat start marker (|:)
    Start,
    /// Repeat end marker (:|)
    End,
    /// Da Capo - return to the beginning
    Dc,
    /// Fine - end of the piece
    Fine,
    /// Dal Segno - return to the segno sign
    Ds,
    /// Segno sign (𝄋) - target for D.S.
    Segno,
    /// Coda sign (⊕) - jump target
    Coda,
    /// First variation ending
    Var1,
    /// Second variation ending
    Var2,
    /// Third variation ending
    Var3,
    /// Fourth variation ending
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

/// A set of repeat symbols that can be applied to a bar.
///
/// This structure ensures that only compatible repeat symbols can be
/// combined together, preventing invalid musical notation.
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RepeatSet {
    value: EnumSet<Repeat>,
}

impl RepeatSet {
    /// All variation ending bits combined.
    const ALL_REGION_BITS: EnumSet<Repeat> = enum_set!(Repeat::Var1 | Repeat::Var2 | Repeat::Var3 | Repeat::Var4);

    /// Repeat symbols that cannot coexist with Start.
    const START_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Dc | Repeat::Ds
        | Repeat::Var1 | Repeat::Var2 | Repeat::Var3 | Repeat::Var4
    );

    /// Repeat symbols that cannot coexist with End.
    const END_DISLIKE: EnumSet<Repeat> = Self::ALL_REGION_BITS;

    /// Repeat symbols that cannot coexist with D.C.
    const DC_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::Ds | Repeat::Segno
        | Repeat::Var1 | Repeat::Var2 | Repeat::Var3 | Repeat::Var4
    );

    /// Repeat symbols that cannot coexist with D.S.
    const DS_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start
        | Repeat::Var1 | Repeat::Var2 | Repeat::Var3 | Repeat::Var4
    );

    /// Repeat symbols that cannot coexist with Var1.
    const REGION1_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::End | Repeat::Dc | Repeat::Ds | Repeat::Segno | Repeat::Var2 | Repeat::Var3 | Repeat::Var4
    );

    /// Repeat symbols that cannot coexist with Var2.
    const REGION2_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::End | Repeat::Dc | Repeat::Ds | Repeat::Segno | Repeat::Var1 | Repeat::Var3 | Repeat::Var4
    );

    /// Repeat symbols that cannot coexist with Var3.
    const REGION3_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::End | Repeat::Dc | Repeat::Ds | Repeat::Segno | Repeat::Var1 | Repeat::Var2 | Repeat::Var4
    );

    /// Repeat symbols that cannot coexist with Var4.
    const REGION4_DISLIKE: EnumSet<Repeat> = enum_set!(
        Repeat::Start | Repeat::End | Repeat::Dc | Repeat::Ds | Repeat::Segno | Repeat::Var1 | Repeat::Var2 | Repeat::Var3
    );

    /// An empty repeat set with no symbols.
    pub const EMPTY: RepeatSet = Self { value: EnumSet::empty() };

    /// Checks if this set contains the specified repeat symbol.
    pub fn contains(self, r: Repeat) -> bool {
        self.value.contains(r)
    }

    /// Creates a new repeat set from an `EnumSet`.
    const fn new(value: EnumSet<Repeat>) -> Self { Self { value } }

    /// Internal helper to try adding a repeat symbol with conflict checking.
    #[inline]
    fn try_add_repeat(self, dislike: EnumSet<Repeat>, r: Repeat) -> Result<Self, EnumSet<Repeat>> {
        let dislike = dislike & self.value;
        if dislike.is_empty() {
            Ok(RepeatSet::new(self.value | r))
        } else {
            Err(dislike)
        }
    }

    /// Attempts to add a repeat symbol to this set.
    ///
    /// # Arguments
    ///
    /// * `r` - The repeat symbol to add.
    ///
    /// # Returns
    ///
    /// - `Ok(RepeatSet)` - A new set with the symbol added.
    /// - `Err(EnumSet<Repeat>)` - The set of conflicting symbols that prevent adding.
    ///
    /// # Examples
    ///
    /// ```
    /// # use klavier_core::bar::{RepeatSet, Repeat};
    /// let set = RepeatSet::EMPTY;
    /// let set = set.try_add(Repeat::Start).unwrap();
    /// assert!(set.contains(Repeat::Start));
    /// ```
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

    /// Removes a repeat symbol from this set.
    pub fn remove(self, r: Repeat) -> Self {
        let mut copy = self;
        copy.value.remove(r);
        copy
    }

    /// Returns the variation index if this set contains a variation ending.
    ///
    /// # Returns
    ///
    /// - `Some(VarIndex)` - The variation index (1-4).
    /// - `None` - If no variation ending is present.
    pub fn region_index(self) -> Option<VarIndex> {
        if self.contains(Repeat::Var1) { Some(VarIndex::VI1) }
        else if self.contains(Repeat::Var2) { Some(VarIndex::VI2) }
        else if self.contains(Repeat::Var3) { Some(VarIndex::VI3) }
        else if self.contains(Repeat::Var4) { Some(VarIndex::VI4) }
        else { None }
    }

    /// Removes all variation ending symbols from this set.
    pub fn remove_resions(self) -> Self {
        let mut copy = self;
        copy.value.remove(Repeat::Var1);
        copy.value.remove(Repeat::Var2);
        copy.value.remove(Repeat::Var3);
        copy.value.remove(Repeat::Var4);
        copy
    }

    /// Returns the number of repeat symbols in this set.
    pub fn len(self) -> usize {
        self.value.len()
    }

    /// Returns `true` if this set contains no repeat symbols.
    pub fn is_empty(self) -> bool {
        self.value.is_empty()
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

/// Represents a measure (bar) in musical notation.
///
/// A bar marks a division in music and can contain rhythm changes, key changes,
/// and repeat symbols.
///
/// # Examples
///
/// ```
/// # use klavier_core::bar::{Bar, RepeatSet};
/// # use klavier_core::rhythm::Rhythm;
/// # use klavier_core::key::Key;
/// let bar = Bar::new(
///     0,
///     Some(Rhythm::new(4, 4)),
///     Some(Key::NONE),
///     RepeatSet::EMPTY
/// );
/// ```
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    /// The tick position where this bar starts.
    pub start_tick: u32,
    
    /// Optional rhythm (time signature) change at this bar.
    pub rhythm: Option<Rhythm>,
    
    /// Optional key signature change at this bar.
    pub key: Option<Key>,
    
    /// Set of repeat symbols applied to this bar.
    pub repeats: RepeatSet,
}

impl Bar {
    /// Creates a new bar with the specified properties.
    ///
    /// # Arguments
    ///
    /// * `start_tick` - The tick position where this bar starts.
    /// * `rhythm` - Optional rhythm (time signature) change.
    /// * `key` - Optional key signature change.
    /// * `repeats` - Set of repeat symbols for this bar.
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

    /// Creates a new bar with the start tick adjusted by dragging.
    ///
    /// This method allows negative deltas and will cast the result to u32,
    /// potentially wrapping around for very large negative values.
    ///
    /// # Arguments
    ///
    /// * `tick_delta` - The amount to add to the start tick.
    pub fn drag(&self, tick_delta: i32) -> Self {
        Self {
             start_tick: ((self.start_tick as i64) + tick_delta as i64) as u32,
             ..*self
        }
    }

    /// Creates a new bar with the start tick adjusted by the specified delta.
    ///
    /// # Arguments
    ///
    /// * `tick_delta` - The amount to add to the start tick.
    ///
    /// # Returns
    ///
    /// - `Ok(Bar)` - A new bar with the adjusted start tick.
    /// - `Err(TickError::Minus)` - If the resulting tick would be negative.
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
        assert!(repeats.contains(Repeat::Dc));
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
        let (start, z) = store.range(NanFreeF32::from(0.0) .. NanFreeF32::from(100.0));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 0);
    }

    #[test]
    fn range_too_left() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, repeat_set!());
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, z) = store.range(NanFreeF32::from(0.0) .. NanFreeF32::from(1.0));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 0);
    }
    
    #[test]
    fn range_hit() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, repeat_set!());
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, z) = store.range(NanFreeF32::from(0.0) .. NanFreeF32::from(1.1));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 1);
        assert_eq!(z[0], (NanFreeF32::from(1.0), bar0));

        let (start, z) = store.range(NanFreeF32::from(1.0) .. NanFreeF32::from(1.1));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 1);
        assert_eq!(z[0], (NanFreeF32::from(1.0), bar0));
    }

    #[test]
    fn range_too_right() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, repeat_set!());
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, z) = store.range(NanFreeF32::from(1.1) .. NanFreeF32::from(2.0));
        assert_eq!(start, 0);
        assert_eq!(z.len(), 0);
    }

    #[test]
    fn range_inclusive_end() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, repeat_set!());
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, z) = store.range(NanFreeF32::from(0.1) ..= NanFreeF32::from(1.0));
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

        let (start, z) = store.range(NanFreeF32::from(0.0) ..= NanFreeF32::from(10.0));
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
        assert!(!set.contains(Repeat::Start));
        assert!(!set.contains(Repeat::End));
        assert!(!set.contains(Repeat::Dc));
        assert!(!set.contains(Repeat::Fine));
        assert!(!set.contains(Repeat::Ds));
        assert!(!set.contains(Repeat::Segno));
        assert!(!set.contains(Repeat::Var1));
        assert!(!set.contains(Repeat::Var2));
        assert!(!set.contains(Repeat::Var3));
        assert!(!set.contains(Repeat::Var4));
    }

    #[test]
    fn repeat_set_contains() {
        let set = repeat_set!(Repeat::Start, Repeat::End);
        assert!(set.contains(Repeat::Start));
        assert!(set.contains(Repeat::End));
        assert!(!set.contains(Repeat::Dc));
        assert!(!set.contains(Repeat::Fine));
        assert!(!set.contains(Repeat::Ds));
        assert!(!set.contains(Repeat::Segno));
        assert!(!set.contains(Repeat::Var1));
        assert!(!set.contains(Repeat::Var2));
        assert!(!set.contains(Repeat::Var3));
        assert!(!set.contains(Repeat::Var4));
    }

    #[test]
    fn cannot_dc_and_start() {
        let result = repeat_set!(Repeat::Dc).try_add(Repeat::Start);
        assert_eq!(result, Err(enum_set!(Repeat::Dc)));
    }
}
