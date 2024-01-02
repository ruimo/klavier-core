#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum SharpFlat {
    Sharp,
    DoubleSharp,
    Flat,
    DoubleFlat,
    Natural,
    Null,
}

impl SharpFlat {
    pub const fn offset(self: Self) -> i32 {
        match self {
            Self::Sharp => 1,
            Self::DoubleSharp => 2,
            Self::Flat => -1,
            Self::DoubleFlat => -2,
            Self::Natural => 0,
            Self::Null => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sharp_flat::SharpFlat;

    #[test]
    fn offset_is_valid() {
        assert_eq!(SharpFlat::Sharp.offset(), 1);
        assert_eq!(SharpFlat::DoubleSharp.offset(), 2);
        assert_eq!(SharpFlat::Flat.offset(), -1);
        assert_eq!(SharpFlat::DoubleFlat.offset(), -2);
        assert_eq!(SharpFlat::Natural.offset(), 0);
        assert_eq!(SharpFlat::Null.offset(), 0);
    }
}
