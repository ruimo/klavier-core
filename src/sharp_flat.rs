/// Musical accidentals (sharp, flat, natural, etc.).
///
/// Represents the accidentals that modify a note's pitch:
/// - Sharp (♯): raises the pitch by one semitone
/// - Flat (♭): lowers the pitch by one semitone
/// - Double sharp (𝄪): raises the pitch by two semitones
/// - Double flat (𝄫): lowers the pitch by two semitones
/// - Natural (♮): cancels previous accidentals
/// - Null: no accidental specified (uses key signature)
///
/// # Examples
///
/// ```
/// # use klavier_core::sharp_flat::SharpFlat;
/// let sharp = SharpFlat::Sharp;      // Raises by 1 semitone
/// let flat = SharpFlat::Flat;        // Lowers by 1 semitone
/// let natural = SharpFlat::Natural;  // Cancels accidentals
/// ```
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum SharpFlat {
    /// Sharp (♯) - raises pitch by one semitone
    Sharp,
    /// Double sharp (𝄪) - raises pitch by two semitones
    DoubleSharp,
    /// Flat (♭) - lowers pitch by one semitone
    Flat,
    /// Double flat (𝄫) - lowers pitch by two semitones
    DoubleFlat,
    /// Natural (♮) - cancels previous accidentals
    Natural,
    /// No accidental specified (uses key signature)
    Null,
}

impl SharpFlat {
    /// Returns the pitch offset in semitones.
    ///
    /// - Sharp: +1
    /// - DoubleSharp: +2
    /// - Flat: -1
    /// - DoubleFlat: -2
    /// - Natural: 0
    /// - Null: 0
    pub const fn offset(self) -> i32 {
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
