use std::ptr;
use std::ops::Range;
use base::Node;

pub trait Sink<K: Ord, V> {
    fn consume(&mut self, item: Node<K, V>);
    fn consume_unchecked(&mut self, item: Node<K, V>);
    fn consume_ptr(&mut self, src: *const Node<K, V>);
}

pub trait TraversalDriver<K: Ord, V>: Sink<K, V> {
    type Decision: TraversalDecision;

    #[inline(always)]
    fn decide(&self, key: &K) -> Self::Decision;
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



impl<K: Ord, V> Sink<K, V> for Vec<(K, V)> {
    #[inline(always)]
    fn consume(&mut self, Node{key, val}: Node<K, V>) {
        self.push((key, val))
    }

    #[inline(always)]
    fn consume_unchecked(&mut self, item: Node<K, V>) {
        consume_unchecked(self, item);
    }

    #[inline(always)]
    fn consume_ptr(&mut self, src: *const Node<K, V>) {
        consume_ptr(self, src);
    }
}



pub struct RangeRefDriver<'a, K: Ord +'a, V: 'a> {
    range: Range<&'a K>,
    output: &'a mut Vec<(K, V)>
}

impl<'a, K: Ord +'a, V> RangeRefDriver<'a, K, V> {
    pub fn new(range: Range<&'a K>, output: &'a mut Vec<(K, V)>) -> RangeRefDriver<'a, K, V> {
        RangeRefDriver { range:range, output:output }
    }

    pub fn from(&self) -> &'a K {
        self.range.start
    }

    pub fn to(&self) -> &'a K {
        self.range.end
    }
}

impl<'a, K: Ord +'a, V> TraversalDriver<K, V> for RangeRefDriver<'a, K, V> {
    type Decision = RangeDecision;

    #[inline(always)]
    fn decide(&self, x: &K) -> Self::Decision {
        let left = self.from() <= x;
        let right = x < self.to();

        RangeDecision { left: left, right: right }
    }
}

impl<'a, K: Ord +'a, V> Sink<K, V> for RangeRefDriver<'a, K, V> {
    #[inline(always)]
    fn consume(&mut self, Node{key, val}: Node<K, V>) {
        self.output.push((key, val))
    }

    #[inline(always)]
    fn consume_unchecked(&mut self, item: Node<K, V>) {
        consume_unchecked(&mut self.output, item);
    }

    #[inline(always)]
    fn consume_ptr(&mut self, src: *const Node<K, V>) {
        consume_ptr(&mut self.output, src);
    }
}



pub struct RangeDriver<'a, K: Ord +'a, V: 'a> {
    range: Range<K>,
    output: &'a mut Vec<(K, V)>
}

impl<'a, K: Ord +'a, V> RangeDriver<'a, K, V> {
    pub fn new(range: Range<K>, output: &'a mut Vec<(K, V)>) -> RangeDriver<K, V> {
        RangeDriver { range:range, output: output }
    }

    pub fn from(&self) -> &K {
        &self.range.start
    }

    pub fn to(&self) -> &K {
        &self.range.end
    }
}

impl<'a, K: Ord +'a, V> TraversalDriver<K, V> for RangeDriver<'a, K, V> {
    type Decision = RangeDecision;

    #[inline(always)]
    fn decide(&self, key: &K) -> Self::Decision {
        let left = self.from() <= key;
        let right = key < self.to();

        RangeDecision { left: left, right: right }
    }
}

impl<'a, K: Ord +'a, V> Sink<K, V> for RangeDriver<'a, K, V> {
    #[inline(always)]
    fn consume(&mut self, item: Node<K, V>) {
        self.output.push(item.into_tuple());
    }

    #[inline(always)]
    fn consume_unchecked(&mut self, item: Node<K, V>) {
        consume_unchecked(&mut self.output, item);
    }

    #[inline(always)]
    fn consume_ptr(&mut self, src: *const Node<K, V>) {
        consume_ptr(&mut self.output, src);
    }
}


#[inline(always)]
pub fn consume_unchecked<K: Ord, V>(output: &mut Vec<(K, V)>, item: Node<K, V>) {
    unsafe {
        let len = output.len();
        debug_assert!(len < output.capacity());
        output.set_len(len + 1);
        let p = output.get_unchecked_mut(len);

        // TODO: optimizer fails here, might want to change to "let tuple: (K,V) = mem::transmute(item)" (but that is not guaranteed to work)
        let Node{key, val} = item;
        ptr::write(p, (key, val));
    }
}


#[inline(always)]
pub fn consume_ptr<K: Ord, V>(output: &mut Vec<(K, V)>, src: *const Node<K, V>) {
    unsafe {
        let len = output.len();
        debug_assert!(len < output.capacity());
        output.set_len(len + 1);
        let p = output.get_unchecked_mut(len);

        // TODO: optimizer fails here, might want to change to "let tuple: (K,V) = mem::transmute(item)" (but that is not guaranteed to work)
        let Node{key, val} = ptr::read(src);
        ptr::write(p, (key, val));
    }
}
