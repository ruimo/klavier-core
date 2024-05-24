use std::fmt::Display;

use regex::Regex;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Location {
    bar_no: usize,
    offset: usize,
}

impl Location {
    pub fn new(bar_no: usize, offset: usize) -> Self {
        Self { bar_no, offset }
    }

    pub fn bar_no(&self) -> usize { self.bar_no }
    pub fn offset(&self) -> usize { self.offset }
    pub fn parse(s: &str) -> Option<Location> { parse_location(s) }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.bar_no, self.offset)
    }
}


const LOCATION_PATTERN: once_cell::unsync::Lazy<Regex> = once_cell::unsync::Lazy::new(|| Regex::new(r"^(\d+):(\d+)$").unwrap());
pub fn parse_location(s: &str) -> Option<Location> {
    LOCATION_PATTERN.captures(s).map(|c| {
        Location {
            bar_no: c.get(1).unwrap().as_str().parse().unwrap(),
            offset: c.get(2).unwrap().as_str().parse().unwrap(),
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::location::{parse_location, Location};

    #[test]
    fn parse_fail() {
        assert_eq!(parse_location(""), None);
        assert_eq!(parse_location("012"), None);
        assert_eq!(parse_location("01:"), None);
        assert_eq!(parse_location(":01"), None);
    }

    #[test]
    fn parse_ok() {
        assert_eq!(parse_location("123:456"), Some(Location::new(123, 456)));
    }
}