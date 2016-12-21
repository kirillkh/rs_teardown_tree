use base::Item;
use std::ptr;

pub trait Sink<T: Item> {
    fn consume(&mut self, item: T);
    fn consume_unchecked(&mut self, item: T);
    fn consume_ptr(&mut self, src: *const T);
}

pub trait TraversalDriver<T: Item>: Sink<T> {
    type Decision: TraversalDecision;

    #[inline(always)]
    fn decide(&self, key: &T::Key) -> Self::Decision;
}


pub trait TraversalDecision {
    #[inline] fn left(&self) -> bool;
    #[inline] fn right(&self) -> bool;
    #[inline] fn consume(&self) -> bool;
}




#[derive(Clone, Copy, Debug)]
pub struct RangeDecision {
    pub left: bool,
    pub right: bool,
}

impl TraversalDecision for RangeDecision {
    #[inline] fn left(&self) -> bool {
        self.left
    }

    #[inline] fn right(&self) -> bool {
        self.right
    }

    #[inline] fn consume(&self) -> bool {
        self.left && self.right
    }
}




pub struct RangeRefDriver<'a, T: Item+'a> {
    from: &'a T::Key,
    to: &'a T::Key,
    output: &'a mut Vec<T>
}

impl<'a, T: Item+'a> RangeRefDriver<'a, T> {
    pub fn new(from: &'a T::Key, to: &'a T::Key, output: &'a mut Vec<T>) -> RangeRefDriver<'a, T> {
        RangeRefDriver { from:from, to:to, output:output }
    }
}

impl<'a, T: Item+'a> TraversalDriver<T> for RangeRefDriver<'a, T> {
    type Decision = RangeDecision;

    #[inline(always)]
    fn decide(&self, x: &T::Key) -> Self::Decision {
        let left = self.from <= x;
        let right = x <= self.to;

        RangeDecision { left: left, right: right }
    }
}

impl<'a, T: Item+'a> Sink<T> for RangeRefDriver<'a, T> {
    #[inline(always)]
    fn consume(&mut self, item: T) {
        self.output.push(item)
    }

    #[inline(always)]
    fn consume_unchecked(&mut self, item: T) {
        consume_unchecked(&mut self.output, item);
    }

    #[inline(always)]
    fn consume_ptr(&mut self, src: *const T) {
        consume_ptr(&mut self.output, src);
    }
}



pub struct RangeDriver<'a, T: Item+'a> {
    from: T::Key,
    to: T::Key,
    output: &'a mut Vec<T>
}

impl<'a, T: Item+'a> RangeDriver<'a, T> {
    pub fn new(from: T::Key, to: T::Key, output: &'a mut Vec<T>) -> RangeDriver<T> {
        RangeDriver { from:from, to:to, output: output }
    }
}

impl<'a, T: Item+'a> TraversalDriver<T> for RangeDriver<'a, T> {
    type Decision = RangeDecision;

    #[inline(always)]
    fn decide(&self, x: &T::Key) -> Self::Decision {
        let left = self.from <= *x;
        let right = *x <= self.to;

        RangeDecision { left: left, right: right }
    }
}

impl<'a, T: Item+'a> Sink<T> for RangeDriver<'a, T> {
    #[inline(always)]
    fn consume(&mut self, item: T) {
        self.output.push(item);
    }

    #[inline(always)]
    fn consume_unchecked(&mut self, item: T) {
        consume_unchecked(&mut self.output, item);
    }

    #[inline(always)]
    fn consume_ptr(&mut self, src: *const T) {
        consume_ptr(&mut self.output, src);
    }
}


#[inline(always)]
fn consume_unchecked<T>(output: &mut Vec<T>, item: T) {
    unsafe {
        let len = output.len();
        debug_assert!(len < output.capacity());
        output.set_len(len + 1);
        let p = output.get_unchecked_mut(len);
        ptr::write(p, item);
    }
}


#[inline(always)]
fn consume_ptr<T>(output: &mut Vec<T>, src: *const T) {
    unsafe {
        let len = output.len();
        debug_assert!(len < output.capacity());
        output.set_len(len + 1);
        let p = output.get_unchecked_mut(len);
        let item = ptr::read(src);
        ptr::write(p, item);
    }
}
