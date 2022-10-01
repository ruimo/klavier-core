#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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

    pub fn offset(self) -> i8 {
        self.0
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