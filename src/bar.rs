use super::{key::Key, rhythm::Rhythm, note::TickError, have_start_tick::{HaveBaseStartTick, HaveStartTick}};

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionIndex {
    RI1, RI2, RI3, RI4, RI5, RI6, RI7, RI8,
}

#[derive(Debug)]
pub enum RegionIndexError {
    InvalidIndex(u8),
}

impl RegionIndex {
    pub fn from_value(value: u8) -> Result<RegionIndex, RegionIndexError> {
        match value {
            1 => Ok(RegionIndex::RI1),
            2 => Ok(RegionIndex::RI2),
            3 => Ok(RegionIndex::RI3),
            4 => Ok(RegionIndex::RI4),
            5 => Ok(RegionIndex::RI5),
            6 => Ok(RegionIndex::RI6),
            7 => Ok(RegionIndex::RI7),
            8 => Ok(RegionIndex::RI8),
            _ => Err(RegionIndexError::InvalidIndex(value))
        }
    }

    pub fn value(self) -> u8 {
        match self {
            RegionIndex::RI1 => 1,
            RegionIndex::RI2 => 2,
            RegionIndex::RI3 => 3,
            RegionIndex::RI4 => 4,
            RegionIndex::RI5 => 5,
            RegionIndex::RI6 => 6,
            RegionIndex::RI7 => 7,
            RegionIndex::RI8 => 8,
        }
    }
}

impl Into<RegionIndex> for u8 {
    fn into(self) -> RegionIndex {
        RegionIndex::from_value(self).unwrap()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DcFine {
    Null,
    Dc,
    Fine,
}

impl Default for DcFine {
    fn default() -> Self {
        Self::Null
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndOrRegion {
    Null,
    RepeatEnd,
    Region(RegionIndex),
}

impl Into<EndOrRegion> for RegionIndex {
    fn into(self) -> EndOrRegion {
        EndOrRegion::Region(self)
    }
}

impl Default for EndOrRegion {
    fn default() -> Self {
        Self::Null
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatStart {
    Null,
    Start,
}

impl Default for RepeatStart {
    fn default() -> Self {
        Self::Null
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    pub start_tick: u32,
    pub rhythm: Option<Rhythm>,
    pub key: Option<Key>,
    pub dc_fine: DcFine,
    pub end_or_region: EndOrRegion,
    pub repeat_start: RepeatStart,
}

impl Bar {
    pub fn new(
        start_tick: u32,
        rhythm: Option<Rhythm>,
        key: Option<Key>,
        dc_fine: DcFine,
        end_or_region: EndOrRegion,
        repeat_start: RepeatStart,
    ) -> Self {
        Self {
            start_tick: start_tick,
            rhythm: rhythm,
            key: key,
            dc_fine: dc_fine,
            end_or_region: end_or_region,
            repeat_start: repeat_start,
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
    use crate::bar::DcFine;
    use crate::bar::EndOrRegion;
    use crate::bar::RegionIndex;
    use crate::bar::RepeatStart;
    use crate::rhythm::Rhythm;

    #[test]
    fn can_serialize_dc_fine() {
        let json_str = serde_json::to_string(&DcFine::Dc).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!("Dc")
        );
    }

    #[test]
    fn can_deserialize_dc_fine() {
        let dc_fine: DcFine = serde_json::from_str(r#""Dc""#).unwrap();
        assert_eq!(dc_fine, DcFine::Dc);
    }

    #[test]
    fn can_serialize_end_or_region() {
        let json_str = serde_json::to_string(&EndOrRegion::Region(RegionIndex::RI1)).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({
                "Region": "RI1"
            })
        );
    }

    #[test]
    fn can_deserialize_end_or_region() {
        let end_or_region: EndOrRegion = serde_json::from_str(r#"
        {
            "Region": "RI1"
        }
        "#).unwrap();
        assert_eq!(end_or_region, EndOrRegion::Region(RegionIndex::RI1));
    }

    #[test]
    fn can_serialize_repeat_start() {
        let json_str = serde_json::to_string(&RepeatStart::Null).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!("Null")
        );
    }

    #[test]
    fn can_deserialize_repeat_start() {
        let end_or_region: RepeatStart = serde_json::from_str(r#"
        "Null"
        "#).unwrap();
        assert_eq!(end_or_region, RepeatStart::Null);
    }

    #[test]
    fn can_serialize_bar() {
        let json_str = serde_json::to_string(&
            Bar {
              start_tick: 123,
              key: None,
              rhythm: Some(Rhythm::new(3, 4)),
              dc_fine: DcFine::Null,
              end_or_region: EndOrRegion::RepeatEnd,
              repeat_start: RepeatStart::Start,
            }).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!({
                "start_tick": 123,
                "dc_fine": "Null",
                "end_or_region": "RepeatEnd",
                "key": null,
                "repeat_start": "Start",
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
        assert_eq!(z.next(), None);
    }

    #[test]
    fn range_too_left() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, mut z) = store.range(NanFreeF32::from(0.0) .. NanFreeF32::from(1.0));
        assert_eq!(start, 0);
        assert_eq!(z.next(), None);
    }
    
    #[test]
    fn range_hit() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, mut z) = store.range(NanFreeF32::from(0.0) .. NanFreeF32::from(1.1));
        assert_eq!(start, 0);
        assert_eq!(z.next(), Some(&(NanFreeF32::from(1.0), bar0)));
        assert_eq!(z.next(), None);

        let (start, mut z) = store.range(NanFreeF32::from(1.0) .. NanFreeF32::from(1.1));
        assert_eq!(start, 0);
        assert_eq!(z.next(), Some(&(NanFreeF32::from(1.0), bar0)));
        assert_eq!(z.next(), None);
    }

    #[test]
    fn range_too_right() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, mut z) = store.range(NanFreeF32::from(1.1) .. NanFreeF32::from(2.0));
        assert_eq!(start, 0);
        assert_eq!(z.next(), None);
    }

    #[test]
    fn range_inclusive_end() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(1, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        store.add(NanFreeF32::from(1.0), bar0, 0);
        let (start, mut z) = store.range(NanFreeF32::from(0.1) ..= NanFreeF32::from(1.0));
        assert_eq!(start, 0);
        assert_eq!(z.next(), Some(&(NanFreeF32::from(1.0), bar0)));
    }

    #[test]
    fn should_sorted() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(2, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar1 = Bar::new(10, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar2 = Bar::new(0, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        store.add(2.0.into(), bar0, 0);
        store.add(10.0.into(), bar1, 0);
        store.add(0.0.into(), bar2, 0);

        let (start, mut z) = store.range(NanFreeF32::from(0.0) ..= NanFreeF32::from(10.0));
        assert_eq!(start, 0);
        assert_eq!(z.next(), Some(&(NanFreeF32::from(0.0), bar2)));
        assert_eq!(z.next(), Some(&(NanFreeF32::from(2.0), bar0)));
        assert_eq!(z.next(), Some(&(NanFreeF32::from(10.0), bar1)));
        assert_eq!(z.next(), None);
    }

    #[test]
    fn index() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(2, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar1 = Bar::new(10, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar2 = Bar::new(0, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
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
        let bar0 = Bar::new(2, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar1 = Bar::new(10, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar2 = Bar::new(0, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar3 = Bar::new(100, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        store.add(2.0.into(), bar0, 0);
        store.add(10.0.into(), bar1, 0);
        store.add(0.0.into(), bar2, 0);
        store.add(100.0.into(), bar3, 0);

        let (start, mut z) = store.range(NanFreeF32::from(1.0) .. NanFreeF32::from(20.0));
        assert_eq!(start, 1);
        assert_eq!(z.next(), Some(&(NanFreeF32::from(2.0), bar0)));
        assert_eq!(z.next(), Some(&(NanFreeF32::from(10.0), bar1)));
        assert_eq!(z.next(), None);

        let (start, mut z) = store.range(NanFreeF32::from(10.0) .. NanFreeF32::from(200.0));
        assert_eq!(start, 2);
        assert_eq!(z.next(), Some(&(NanFreeF32::from(10.0), bar1)));
        assert_eq!(z.next(), Some(&(NanFreeF32::from(100.0), bar3)));
        assert_eq!(z.next(), None);

        let (start, mut z) = store.range(NanFreeF32::from(10.0) ..);
        assert_eq!(start, 2);
        assert_eq!(z.next(), Some(&(NanFreeF32::from(10.0), bar1)));
        assert_eq!(z.next(), Some(&(NanFreeF32::from(100.0), bar3)));
        assert_eq!(z.next(), None);
    }

    #[test]
    fn observe() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(true);
        let bar0 = Bar::new(0, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        store.add(NanFreeF32::from(0.0), bar0, 999);

        let events = store.events();
        assert_eq!(events.len(), 1);
        let e = &events[0];
        match e {
            StoreEvent::Add { added: s, metadata } => {
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
        let bar0 = Bar::new(2, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar1 = Bar::new(10, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar2 = Bar::new(0, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar3 = Bar::new(100, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
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
        let bar0 = Bar::new(2, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar1 = Bar::new(10, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        let bar2 = Bar::new(0, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        store.add(2.0.into(), bar0, 0);
        store.add(10.0.into(), bar1, 0);
        store.add(0.0.into(), bar2, 0);

        let bar3 = Bar::new(0, None, None, DcFine::Dc, EndOrRegion::Null, RepeatStart::Null);
        store.update(1, bar3);
        assert_eq!(store[1], (2.0.into(), bar3));
    }

    #[test]
    fn as_ref() {
        let mut store: Store<NanFreeF32, Bar, i32> = Store::new(false);
        let bar0 = Bar::new(2, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
        store.add(2.0.into(), bar0, 0);

        let as_ref = store.as_ref();
        assert_eq!(as_ref.len(), 1);
        assert_eq!(as_ref[0].0, 2.0.into());
        assert_eq!(as_ref[0].1, bar0);
    }
}