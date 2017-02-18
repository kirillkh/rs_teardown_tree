use std::ops::Range;
use std::marker::PhantomData;

use base::{Key, Sink};

pub trait TraversalDriver<K: Key, V>: Sink<(K, V)> {
    type Decision: TraversalDecision;

    #[inline(always)]
    fn decide(&self, key: &K) -> Self::Decision;
}


pub trait TraversalDecision {
    #[inline] fn left(&self) -> bool;
    #[inline] fn right(&self) -> bool;
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
}




pub struct RangeRefDriver<'a, K, Q, V, S>
    where K: Key +'a, Q: PartialOrd<K>+'a, V: 'a, S: Sink<(K, V)>
{
    range: Range<&'a Q>,
    sink: S,
    _ph: PhantomData<(K, V)>
}

impl<'a, K, Q, V, S> RangeRefDriver<'a, K, Q, V, S>
    where K: Key +'a, Q: PartialOrd<K>+'a, V: 'a, S: Sink<(K, V)>
{
    pub fn new(range: Range<&'a Q>, sink: S) -> Self {
        RangeRefDriver { range:range, sink:sink, _ph: PhantomData }
    }

    pub fn from(&self) -> &'a Q {
        self.range.start
    }

    pub fn to(&self) -> &'a Q {
        self.range.end
    }
}

impl<'a, K, Q, V, S> TraversalDriver<K, V> for RangeRefDriver<'a, K, Q, V, S>
    where K: Key +'a, Q: PartialOrd<K>+'a, V: 'a, S: Sink<(K, V)>
{
    type Decision = RangeDecision;

    #[inline(always)]
    fn decide(&self, x: &K) -> Self::Decision {
        let left = self.from() <= x;
        let right = self.to() > x;

        RangeDecision { left: left, right: right }
    }
}

impl<'a, K, Q, V, S> Sink<(K, V)> for RangeRefDriver<'a, K, Q, V, S>
    where K: Key +'a, Q: PartialOrd<K>+'a, V: 'a, S: Sink<(K, V)>
{
    #[inline(always)]
    fn consume(&mut self, item: (K, V)) {
        self.sink.consume(item)
    }
}


pub struct RangeDriver<K, Q, V, S>
    where K: Key, Q: PartialOrd<K>, S: Sink<(K, V)>
{
    range: Range<Q>,
    sink: S,
    _ph: PhantomData<(K, V)>
}

impl<K, Q, V, S> RangeDriver<K, Q, V, S>
    where K: Key, Q: PartialOrd<K>, S: Sink<(K, V)>
{
    pub fn new(range: Range<Q>, sink: S) -> RangeDriver<K, Q, V, S> {
        RangeDriver { range:range, sink: sink, _ph: PhantomData }
    }

    pub fn from(&self) -> &Q {
        &self.range.start
    }

    pub fn to(&self) -> &Q {
        &self.range.end
    }
}

impl<K, Q, V, S> TraversalDriver<K, V> for RangeDriver<K, Q, V, S>
    where K: Key, Q: PartialOrd<K>, S: Sink<(K, V)>
{
    type Decision = RangeDecision;

    #[inline(always)]
    fn decide(&self, key: &K) -> Self::Decision {
        let left = self.from() <= key;
        let right = self.to() > key || self.from() == key;

        RangeDecision { left: left, right: right }
    }
}

impl<K, Q, V, S> Sink<(K, V)> for RangeDriver<K, Q, V, S>
    where K: Key, Q: PartialOrd<K>, S: Sink<(K, V)>
{
    #[inline(always)]
    fn consume(&mut self, item: (K, V)) {
        self.sink.consume(item)
    }
}
