pub trait CanApply<T> {
    fn apply(self, value: T) -> T;
}

