use std::cmp::Ordering;


pub trait Interval: Sized+Ord {
    type K: Ord+Clone;

    fn a(&self) -> &Self::K;
    fn b(&self) -> &Self::K;
}

pub struct KeyInterval<K: Ord+Clone> {
    a: K,
    b: K
}

impl<K: Ord+Clone> KeyInterval<K> {
    pub fn new(a: K, b: K) -> KeyInterval<K> {
        KeyInterval { a:a, b:b }
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

#[derive(Clone)]
pub struct IntervalNode<Iv: Interval> {
    pub ivl: Iv,
    pub maxb: Iv::K
}

impl<Iv: Interval> IntervalNode<Iv> {
    #[inline] pub fn a(&self) -> &Iv::K {
        self.ivl.a()
    }

    #[inline] pub fn b(&self) -> &Iv::K {
        self.ivl.b()
    }

    #[inline] pub fn max(&self) -> &Iv::K {
        &self.maxb
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



impl<Iv: Interval> PartialEq for IntervalNode<Iv> {
    fn eq(&self, other: &Self) -> bool {
        self.a() == other.a() && self.b() == other.b()
    }
}
impl<Iv: Interval> Eq for IntervalNode<Iv> {}

impl<Iv: Interval> PartialOrd for IntervalNode<Iv> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Iv: Interval> Ord for IntervalNode<Iv> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.a().cmp(other.a()) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.b().cmp(other.b())
        }
    }
}
