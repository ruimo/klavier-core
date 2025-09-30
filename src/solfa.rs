use std::{fmt, ops::{AddAssign, SubAssign}};

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Solfa {
    C, D, E, F, G, A, B,
}

impl AddAssign<i32> for Solfa {
    fn add_assign(&mut self, rhs: i32) {
        let so = self.score_offset() + rhs;
        if Solfa::B.score_offset() < so {
            panic!("Solfa overflow");
        }
        *self = Solfa::from_score_offset(so);
    }
}

impl SubAssign<i32> for Solfa {
    fn sub_assign(&mut self, rhs: i32) {
        let so = self.score_offset() - rhs;
        if so < Solfa::C.score_offset() {
            panic!("Solfa overflow");
        }
        *self = Solfa::from_score_offset(so);
    }
}

impl Solfa {
    pub const ALL: &'static [Solfa] = &[Self::C, Self::D, Self::E, Self::F, Self::G, Self::A, Self::B];

    pub const fn score_offset(self) -> i32 {
        match self {
            Self::C => 0,
            Self::D => 1,
            Self::E => 2,
            Self::F => 3,
            Self::G => 4,
            Self::A => 5,
            Self::B => 6,
        }
    }

    pub const fn pitch_offset(self) -> i32 {
        match self {
            Self::C => 0,
            Self::D => 2,
            Self::E => 4,
            Self::F => 5,
            Self::G => 7,
            Self::A => 9,
            Self::B => 11,
        }
    }

    pub fn from_score_offset(offset: i32) -> Solfa {
        if offset < Self::C.score_offset() {
            Self::C
        } else if offset > Self::B.score_offset() {
            Self::B
        } else {
            Self::ALL[offset as usize]
        }
    }
}

impl fmt::Display for Solfa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Solfa::C => write!(f, "C"),
            Solfa::D => write!(f, "D"),
            Solfa::E => write!(f, "E"),
            Solfa::F => write!(f, "F"),
            Solfa::G => write!(f, "G"),
            Solfa::A => write!(f, "A"),
            Solfa::B => write!(f, "B"),
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::solfa::Solfa;

    #[test]
    fn score_offset_is_valid() {
        assert_eq!(Solfa::C.score_offset(), 0);
        assert_eq!(Solfa::D.score_offset(), 1);
        assert_eq!(Solfa::E.score_offset(), 2);
        assert_eq!(Solfa::F.score_offset(), 3);
        assert_eq!(Solfa::G.score_offset(), 4);
        assert_eq!(Solfa::A.score_offset(), 5);
        assert_eq!(Solfa::B.score_offset(), 6);
    }

    #[test]
    fn pitch_offset_is_valid() {
        assert_eq!(Solfa::C.pitch_offset(), 0);
        assert_eq!(Solfa::D.pitch_offset(), 2);
        assert_eq!(Solfa::E.pitch_offset(), 4);
        assert_eq!(Solfa::F.pitch_offset(), 5);
        assert_eq!(Solfa::G.pitch_offset(), 7);
        assert_eq!(Solfa::A.pitch_offset(), 9);
        assert_eq!(Solfa::B.pitch_offset(), 11);
    }

    #[test]
    fn from_score_offset() {
        assert_eq!(Solfa::from_score_offset(-1), Solfa::C);
        assert_eq!(Solfa::from_score_offset(0), Solfa::C);
        assert_eq!(Solfa::from_score_offset(1), Solfa::D);
        assert_eq!(Solfa::from_score_offset(2), Solfa::E);
        assert_eq!(Solfa::from_score_offset(3), Solfa::F);
        assert_eq!(Solfa::from_score_offset(4), Solfa::G);
        assert_eq!(Solfa::from_score_offset(5), Solfa::A);
        assert_eq!(Solfa::from_score_offset(6), Solfa::B);
        assert_eq!(Solfa::from_score_offset(7), Solfa::B);
    }

    #[test]
    fn all() {
        assert_eq!(Solfa::ALL[0], Solfa::C);
        assert_eq!(Solfa::ALL[1], Solfa::D);
        assert_eq!(Solfa::ALL[2], Solfa::E);
        assert_eq!(Solfa::ALL[3], Solfa::F);
        assert_eq!(Solfa::ALL[4], Solfa::G);
        assert_eq!(Solfa::ALL[5], Solfa::A);
        assert_eq!(Solfa::ALL[6], Solfa::B);
        assert_eq!(Solfa::ALL.len(), 7);
    }

    #[test]
    fn add_assign() {
        let mut solfa = Solfa::C;
        solfa += 1;
        assert_eq!(solfa, Solfa::D);
        solfa += 2;
        assert_eq!(solfa, Solfa::F);
    }

    #[test]
    #[should_panic]
    fn add_assign_error() {
        let mut solfa = Solfa::B;
        solfa += 1;
    }

    #[test]
    fn sub_assign() {
        let mut solfa = Solfa::B;
        solfa -= 1;
        assert_eq!(solfa, Solfa::A);
        solfa -= 2;
        assert_eq!(solfa, Solfa::F);
    }

    #[test]
    #[should_panic]
    fn sub_assign_error() {
        let mut solfa = Solfa::C;
        solfa -= 1;
    }
}
