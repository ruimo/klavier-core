use std::fmt::Display;

use once_cell::sync::Lazy;
use regex::Regex;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Location {
    bar_no: usize,
    offset: usize,
}

impl Location {
    const PARSER: Lazy<LocationParser> = Lazy::new(|| LocationParser::default());

    pub fn new(bar_no: usize, offset: usize) -> Self {
        Self { bar_no, offset }
    }

    pub fn bar_no(&self) -> usize { self.bar_no }
    pub fn offset(&self) -> usize { self.offset }
    pub fn parse(s: &str) -> Option<Location> { Self::PARSER.parse(s) }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.bar_no, self.offset)
    }
}

pub struct LocationParser {
    pattern: Regex,
}

impl Default for LocationParser {
    fn default() -> Self {
        Self {
            pattern: Regex::new(r"^(\d+):(\d+)$").unwrap()
        }
    }
}

impl LocationParser {
    pub fn parse(&self, s: &str) -> Option<Location> {
        self.pattern.captures(s).map(|c| {
            Location {
                bar_no: c.get(1).unwrap().as_str().parse().unwrap(),
                offset: c.get(2).unwrap().as_str().parse().unwrap(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::location::Location;
    use super::LocationParser;

    #[test]
    fn parse_fail() {
        let parser = LocationParser::default();
        assert_eq!(parser.parse(""), None);
        assert_eq!(parser.parse("012"), None);
        assert_eq!(parser.parse("01:"), None);
        assert_eq!(parser.parse(":01"), None);
    }

    #[test]
    fn parse_ok() {
        let parser = LocationParser::default();
        assert_eq!(parser.parse("123:456"), Some(Location::new(123, 456)));
    }
}