use delete_range::{TraversalDriver, TraversalDecision};
use base::Item;

pub struct DriverFromToRef<'a, T: Item+'a> {
    from: &'a T::Key,
    to: &'a T::Key
}

impl<'a, T: Item+'a> DriverFromToRef<'a, T> {
    pub fn new(from: &'a T::Key, to: &'a T::Key) -> DriverFromToRef<'a, T> {
        DriverFromToRef { from:from, to:to }
    }
}

impl<'a, T: Item+'a> TraversalDriver<T> for DriverFromToRef<'a, T> {
    #[inline(always)]
    fn decide(&self, x: &T::Key) -> TraversalDecision {
        let left = self.from <= x;
        let right = x <= self.to;

        TraversalDecision { left: left, right: right }
    }
}



pub struct DriverFromTo<T: Item> {
    from: T::Key,
    to: T::Key
}

impl<T: Item> DriverFromTo<T> {
    pub fn new(from: T::Key, to: T::Key) -> DriverFromTo<T> {
        DriverFromTo { from:from, to:to }
    }
}

impl<T: Item> TraversalDriver<T> for DriverFromTo<T> {
    #[inline(always)]
    fn decide(&self, x: &T::Key) -> TraversalDecision {
        let left = self.from <= *x;
        let right = *x <= self.to;

        TraversalDecision { left: left, right: right }
    }
}

