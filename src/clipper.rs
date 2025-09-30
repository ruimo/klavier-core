#[derive(Debug, Clone)]
pub struct Clipper<T> where T: PartialOrd + Copy {
    pub min: T,
    pub max: T,
}

pub const fn for_i32(min: i32, max: i32) -> Clipper<i32> {
    Clipper::<i32> { min, max, }
}

pub const fn for_i16(min: i16, max: i16) -> Clipper<i16> {
    Clipper::<i16> { min, max, }
}

pub const fn for_f32(min: f32, max: f32) -> Clipper<f32> {
    Clipper::<f32> { min, max, }
}

impl <T: PartialOrd + Copy> Clipper<T> {
    pub fn clip(&self, value: T) -> T {
        if value < self.min {
            self.min
        } else if self.max < value {
            self.max
        } else {
            value
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::clipper;

    #[test]
    fn can_i32_clipped() {
        let clipper = clipper::for_i32(5, 10);
        assert_eq!(clipper.clip(4), 5);
        assert_eq!(clipper.clip(5), 5);
        assert_eq!(clipper.clip(6), 6);
        assert_eq!(clipper.clip(9), 9);
        assert_eq!(clipper.clip(10), 10);
        assert_eq!(clipper.clip(11), 10);
    }

    #[test]
    fn can_f32_clipped() {
        let clipper = clipper::for_f32(5f32, 10f32);
        assert_eq!(clipper.clip(4f32), 5f32);
        assert_eq!(clipper.clip(5f32), 5f32);
        assert_eq!(clipper.clip(6f32), 6f32);
        assert_eq!(clipper.clip(9f32), 9f32);
        assert_eq!(clipper.clip(10f32), 10f32);
        assert_eq!(clipper.clip(11f32), 10f32);
    }
}
