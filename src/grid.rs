use std::fmt;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    ParseError(String),
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    value: u32,
}

impl Grid {
    pub fn value_of(s: &str) -> Result<Self, GridError> {
        s.parse::<u32>()
            .map_err(|_| {
                GridError::ParseError(s.to_owned())
            })
            .and_then(|i| {
                Self::from_u32(i)
            })
    }

    #[inline]
    pub fn from_u32(i: u32) -> Result<Self, GridError> {
        if i == 0 {
            Err(GridError::ParseError("0".to_owned()))
        } else {
            Ok(Self { value: i })
        }
    }

    #[inline]
    pub fn as_u32(self) -> u32 {
        self.value
    }

    #[inline]
    pub fn snap(self, tick: i64) -> i64 {
        let i = self.value as i64;
        if tick < 0 {
            i * ((tick - i / 2) / i)
        } else {
            i * ((tick + i / 2) / i)
        }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self { value: 60 }
    }
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[cfg(test)]
mod tests {
    use crate::grid::{Grid, GridError};

    #[test]
    fn empty_str() {
        assert_eq!(Grid::value_of(""), Err(GridError::ParseError("".to_owned())));
    }


    #[test]
    fn non_numerical_str() {
        assert_eq!(Grid::value_of("1a"), Err(GridError::ParseError("1a".to_owned())));
    }

    #[test]
    fn zero() {
        assert_eq!(Grid::value_of("0"), Err(GridError::ParseError("0".to_owned())));
    }

    #[test]
    fn ok() {
        assert_eq!(Grid::value_of("120"), Ok(Grid { value: 120 }));
    }

    #[test]
    fn snap() {
        assert_eq!(Grid::from_u32(100).unwrap().snap(49), 0);
        assert_eq!(Grid::from_u32(100).unwrap().snap(50), 100);
        assert_eq!(Grid::from_u32(100).unwrap().snap(99), 100);
        assert_eq!(Grid::from_u32(100).unwrap().snap(149), 100);
        assert_eq!(Grid::from_u32(100).unwrap().snap(150), 200);
        assert_eq!(Grid::from_u32(100).unwrap().snap(199), 200);
        assert_eq!(Grid::from_u32(100).unwrap().snap(249), 200);
 
        assert_eq!(Grid::from_u32(100).unwrap().snap(-49), 0);
        assert_eq!(Grid::from_u32(100).unwrap().snap(-50), -100);
    }
}