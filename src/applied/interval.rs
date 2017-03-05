use std::cmp::Ordering;
use std::ops::{Deref, DerefMut, Range};
use std::fmt;

use base::{Node, Entry};


pub trait Interval: Sized+Ord+Clone {
    type K: Ord+Clone;

    fn a(&self) -> &Self::K;
    fn b(&self) -> &Self::K;

    fn overlaps<Other: Interval<K=Self::K>>(&self, other: &Other) -> bool {
        self.a() < other.b() && other.a() < self.b()
            || self.a() == other.a() // interpret empty intervals as points
    }

    fn to_range(&self) -> Range<Self::K> {
        self.a().clone() .. self.b().clone()
    }
}


impl Interval for usize {
    type K = usize;

    fn a(&self) -> &Self::K { self }
    fn b(&self) -> &Self::K { self }
}


#[derive(Debug, Clone, Copy)]
pub struct KeyInterval<K: Ord+Clone> {
    a: K,
    b: K
}

impl<K: Ord+Clone> KeyInterval<K> {
    pub fn new(a: K, b: K) -> KeyInterval<K> {
        KeyInterval { a:a, b:b }
    }

    pub fn from_range(r: &Range<K>) -> KeyInterval<K> {
        Self::new(r.start.clone(), r.end.clone())
    }
}

impl<K: Ord+Clone> Interval for KeyInterval<K> {
    type K = K;

    fn a(&self) -> &Self::K {
        &self.a
    }

    fn b(&self) -> &Self::K {
        &self.b
    }
}

impl<K: Ord+Clone> From<Range<K>> for KeyInterval<K> {
    fn from(range: Range<K>) -> Self {
        Self::from_range(&range)
    }
}



#[derive(Clone)]
pub struct IvNode<Iv: Interval, V> {
    pub entry: Entry<Iv, V>,
    pub maxb: Iv::K
}

impl<Iv: Interval, V> Deref for IvNode<Iv, V> {
    type Target = Entry<Iv, V>;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl<Iv: Interval, V> DerefMut for IvNode<Iv, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry
    }
}


impl<Iv: Interval, V> Node for IvNode<Iv, V> {
    type K = Iv;
    type V = V;

    fn new(key: Iv, val: V) -> Self {
        let maxb = key.b().clone();
        IvNode { entry: Entry::new(key, val), maxb:maxb }
    }

    fn from_tuple(t: (Iv, V)) -> Self {
        let maxb = t.0.b().clone();
        IvNode { entry: Entry::from_tuple(t), maxb:maxb }
    }


    #[inline(always)]
    fn into_entry(self) -> Entry<Iv, V> {
        self.entry
    }

    #[inline(always)]
    fn into_tuple(self) -> (Iv, V) {
        self.entry.into_tuple()
    }
}

impl<K: Ord+Clone> PartialEq for KeyInterval<K> {
    fn eq(&self, other: &Self) -> bool {
        self.a() == other.a() && self.b() == other.b()
    }
}
impl<K: Ord+Clone> Eq for KeyInterval<K> {}

impl<K: Ord+Clone> PartialOrd for KeyInterval<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Ord+Clone> Ord for KeyInterval<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.a().cmp(other.a()) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.b().cmp(other.b())
        }
    }
}


impl<K: Ord+Clone+fmt::Debug, Iv: Interval<K=K>, V> fmt::Debug for IvNode<Iv, V> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "({:?}..{:?}, m={:?})", self.key().a(), self.key().b(), &self.maxb)
    }
}
