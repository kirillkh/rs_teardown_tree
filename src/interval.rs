use drivers::{TraversalDriver, TraversalDecision};

pub trait Interval: Sized {
    type K: Ord;

    fn a(&self) -> &Self::K;
    fn b(&self) -> &Self::K;
}

pub struct KeyInterval<K: Ord> {
    a: K,
    b: K
}

impl<K: Ord> KeyInterval<K> {
    pub fn new(a: K, b: K) -> KeyInterval<K> {
        KeyInterval { a:a, b:b }
    }
}


impl<K: Ord> Interval for KeyInterval<K> {
    type K = K;

    fn a(&self) -> &Self::K {
        &self.a
    }

    fn b(&self) -> &Self::K {
        &self.b
    }
}


pub struct IntervalNode<Iv: Interval> {
    ivl: Iv,
    max: Iv::K
}

impl<Iv: Interval> IntervalNode<Iv> {
    #[inline]
    pub fn a(&self) -> &Iv::K {
        self.ivl.a()
    }

    #[inline]
    pub fn b(&self) -> &Iv::K {
        self.ivl.b()
    }

    #[inline]
    pub fn max(&self) -> &Iv::K {
        &self.max
    }
}



pub struct IntervalDecision {
    left: bool,
    right: bool,
    consume: bool
}

impl IntervalDecision {
    pub fn new(left: bool, right: bool, consume: bool) -> IntervalDecision {
        IntervalDecision { left:left, right:right, consume:consume }
    }
}

impl TraversalDecision for IntervalDecision {
    fn left(&self) -> bool {
        self.left
    }

    fn right(&self) -> bool {
        self.right
    }

    fn consume(&self) -> bool {
        self.consume
    }
}


pub struct IntervalDriver<Iv: Interval> {
    ivl: Iv
}

impl<Iv: Interval> IntervalDriver<Iv> {
    pub fn new(ivl: Iv) -> IntervalDriver<Iv> {
        IntervalDriver { ivl: ivl }
    }

//    pub fn with_bounds(a: Iv::K, b: Iv::K) -> IntervalDriver<Iv::K, Iv> {
//        IntervalDriver { ivl: KeyInterval::new(a, b) }
//    }
}


impl<Iv: Interval> TraversalDriver<IntervalNode<Iv>> for IntervalDriver<Iv> {
    type Decision = IntervalDecision;

    fn decide(&self, ivl: &IntervalNode<Iv>) -> Self::Decision {
        if ivl.max() <= self.ivl.a() {
            IntervalDecision::new(false, false, false)
        } else if self.ivl.b() <= ivl.a() {
            IntervalDecision::new(true, false, false)
        } else {
            let consume = self.ivl.a() < ivl.b();
            IntervalDecision::new(true, false, consume)
        }
    }
}
