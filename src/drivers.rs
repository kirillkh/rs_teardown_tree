pub trait TraversalDriver<K> {
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




pub struct RangeRefDriver<'a, K: Ord+'a> {
    from: &'a K,
    to: &'a K
}

impl<'a, K: Ord+'a> RangeRefDriver<'a, K> {
    pub fn new(from: &'a K, to: &'a K) -> RangeRefDriver<'a, K> {
        RangeRefDriver { from:from, to:to }
    }
}

impl<'a, K: Ord+'a> TraversalDriver<K> for RangeRefDriver<'a, K> {
    type Decision = RangeDecision;

    #[inline(always)]
    fn decide(&self, x: &K) -> Self::Decision {
        let left = self.from <= x;
        let right = x <= self.to;

        RangeDecision { left: left, right: right }
    }
}



pub struct RangeDriver<K: Ord> {
    from: K,
    to: K
}

impl<K: Ord> RangeDriver<K> {
    pub fn new(from: K, to: K) -> RangeDriver<K> {
        RangeDriver { from:from, to:to }
    }
}

impl<K: Ord> TraversalDriver<K> for RangeDriver<K> {
    type Decision = RangeDecision;

    #[inline(always)]
    fn decide(&self, x: &K) -> Self::Decision {
        let left = self.from <= *x;
        let right = *x <= self.to;

        RangeDecision { left: left, right: right }
    }
}
