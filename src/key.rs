use std::collections::{HashMap, HashSet};

use once_cell::unsync::Lazy;

use crate::solfa::Solfa;

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct Key(i8);

impl Key {
    pub const NONE: Key = Key(0);
    pub const SHARP_1: Key = Key(1);
    pub const SHARP_2: Key = Key(2);
    pub const SHARP_3: Key = Key(3);
    pub const SHARP_4: Key = Key(4);
    pub const SHARP_5: Key = Key(5);
    pub const SHARP_6: Key = Key(6);
    pub const SHARP_7: Key = Key(7);
    pub const FLAT_1: Key = Key(-1);
    pub const FLAT_2: Key = Key(-2);
    pub const FLAT_3: Key = Key(-3);
    pub const FLAT_4: Key = Key(-4);
    pub const FLAT_5: Key = Key(-5);
    pub const FLAT_6: Key = Key(-6);
    pub const FLAT_7: Key = Key(-7);
    pub const ALL: [Key; 15] = [
        Self::NONE, Self::SHARP_1, Self::SHARP_2, Self::SHARP_3, Self::SHARP_4, Self::SHARP_5, Self::SHARP_6, Self::SHARP_7,
        Self::FLAT_1, Self::FLAT_2, Self::FLAT_3, Self::FLAT_4, Self::FLAT_5, Self::FLAT_6, Self::FLAT_7
    ];

    pub const SOLFAS: Lazy<HashMap<Key, HashSet<Solfa>>> = Lazy::new(||
        HashMap::from([
            (Self::NONE, HashSet::from([])),
            (Self::SHARP_1, HashSet::from([Solfa::F])),
            (Self::SHARP_2, HashSet::from([Solfa::F, Solfa::C])),
            (Self::SHARP_3, HashSet::from([Solfa::F, Solfa::C, Solfa::G])),
            (Self::SHARP_4, HashSet::from([Solfa::F, Solfa::C, Solfa::G, Solfa::D])),
            (Self::SHARP_5, HashSet::from([Solfa::F, Solfa::C, Solfa::G, Solfa::D, Solfa::A])),
            (Self::SHARP_6, HashSet::from([Solfa::F, Solfa::C, Solfa::G, Solfa::D, Solfa::A, Solfa::E])),
            (Self::SHARP_7, HashSet::from([Solfa::F, Solfa::C, Solfa::G, Solfa::D, Solfa::A, Solfa::E, Solfa::B])),
            (Self::FLAT_1, HashSet::from([Solfa::B])),
            (Self::FLAT_2, HashSet::from([Solfa::B, Solfa::E])),
            (Self::FLAT_3, HashSet::from([Solfa::B, Solfa::E, Solfa::A])),
            (Self::FLAT_4, HashSet::from([Solfa::B, Solfa::E, Solfa::A, Solfa::D])),
            (Self::FLAT_5, HashSet::from([Solfa::B, Solfa::E, Solfa::A, Solfa::D, Solfa::G])),
            (Self::FLAT_6, HashSet::from([Solfa::B, Solfa::E, Solfa::A, Solfa::D, Solfa::G, Solfa::C])),
            (Self::FLAT_7, HashSet::from([Solfa::B, Solfa::E, Solfa::A, Solfa::D, Solfa::G, Solfa::C, Solfa::F])),
        ])
    );

    pub fn offset(self) -> i8 {
        self.0
    }

    pub fn is_flat(self) -> bool {
        self.0 < 0
    }

    pub fn is_sharp(self) -> bool {
        0 < self.0
    }
}

impl Default for Key {
    fn default() -> Self {
        Self::NONE
    }
}

#[cfg(test)]
mod tests {
    use crate::key;
    use serde_json::Value;
    use serde_json::json;
    use super::Key;

    #[test]
    fn can_match() {
        assert_eq!(key::Key::NONE.offset(), 0);
        assert_eq!(key::Key::SHARP_3.offset(), 3);
        assert_eq!(key::Key::FLAT_3.offset(), -3);
    }

    #[test]    
    fn can_serialize_to_json() {
        let json_str = serde_json::to_string(&Key::FLAT_2).unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            json,
            json!(-2)
        );
    }

    #[test]
    fn can_deserialize_from_json() {
        let key: Key = serde_json::from_str("-2").unwrap();
        assert_eq!(key, Key::FLAT_2);
    }
}