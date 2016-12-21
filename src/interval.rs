use base::{Item, TeardownTreeInternal, lefti, righti};
use drivers::Sink;
use delete_range::BulkDeleteCommon;
use std::cmp::Ordering;


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




pub trait DeleteIntersectingIntervals<Iv: Interval, T: Item<Key=IntervalNode<Iv>>> {
    fn delete_intersecting_ivl<S: Sink<T>>(&mut self, search: &Iv, idx: usize, sink: &mut S);
}

trait DeleteIntersectingIntervalsInternal<Iv: Interval, T: Item<Key=IntervalNode<Iv>>> {
    fn delete_intersecting_ivl_rec<S: Sink<T>>(&mut self, search: &Iv, idx: usize,
                                               min_included: bool, sink: &mut S);
    fn descend_delete_intersecting_ivl_left<S: Sink<T>>(&mut self, search: &Iv, idx: usize, with_slot: bool,
                                                        min_included: bool, sink: &mut S) -> bool;
    fn descend_delete_intersecting_ivl_right<S: Sink<T>>(&mut self, search: &Iv, idx: usize, with_slot: bool,
                                                         min_included: bool, sink: &mut S) -> bool;
}


impl<Iv: Interval, T: Item<Key=IntervalNode<Iv>>> DeleteIntersectingIntervals<Iv, T> for TeardownTreeInternal<T> {
    #[inline]
    fn delete_intersecting_ivl<S: Sink<T>>(&mut self, search: &Iv, idx: usize, sink: &mut S) {
        self.delete_intersecting_ivl_rec(search, idx, false, sink);
    }
}

impl<Iv: Interval, T: Item<Key=IntervalNode<Iv>>> DeleteIntersectingIntervalsInternal<Iv, T> for TeardownTreeInternal<T> {
    fn delete_intersecting_ivl_rec<S: Sink<T>>(&mut self, search: &Iv, idx: usize, mut min_included: bool, sink: &mut S) {
        let k: &IntervalNode<Iv> = self.node_unsafe(idx).item.key();

        if k.max() <= search.a() {
            // whole subtree outside the range
            if self.slots_min().has_open() {
                self.fill_slots_min(idx);
            }
            if self.slots_max().has_open() && !self.is_nil(idx) {
                self.fill_slots_max(idx);
            }
        } else if search.b() <= k.a() {
            // root and right are outside the range
            self.descend_delete_intersecting_ivl_left(search, idx, false, min_included, sink);

            let removed = if self.slots_min().has_open() {
                self.fill_slot_min(idx);

                self.descend_fill_right(idx)
            } else {
                false
            };

            if self.slots_max().has_open() {
                if removed {
                    self.descend_fill_left(idx);
                } else {
                    self.fill_slots_max(idx);
                }
            }
        } else {
            // consume root if necessary
            let consume = search.a() < k.b() && k.a() < search.b();
            let item = if consume
                { Some(self.take(idx)) }
            else
                { None };

            // left subtree
            let mut removed = consume;
            if consume {
                if min_included {
                    self.consume_subtree(lefti(idx), sink)
                } else {
                    removed = self.descend_delete_intersecting_ivl_left(search, idx, true, false, sink);
                }

                sink.consume_unchecked(item.unwrap());
            } else {
                removed = self.descend_delete_intersecting_ivl_left(search, idx, false, min_included, sink);
            }

            // right subtree
            min_included = min_included || search.a() <= k.a();
            if min_included {
                let max_included = k.max() <= search.b();
                if max_included {
                    self.consume_subtree(righti(idx), sink);
                } else {
                    removed = self.descend_delete_intersecting_ivl_right(search, idx, removed, min_included, sink);
                }
            } else {
                removed = self.descend_delete_intersecting_ivl_right(search, idx, removed, false, sink);
            }

            // fill the remaining open slots_max from the left subtree
            if removed && self.slots_max().has_open() {
                self.descend_fill_left(righti(idx));
            }
        }
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_intersecting_ivl_left<S: Sink<T>>(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, sink: &mut S) -> bool {
        if with_slot {
            self.descend_left_with_slot(idx,
                                        |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included, sink)
            )
        } else {
            self.descend_left(idx,
                              |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included, sink)
            );

            false
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_intersecting_ivl_right<S: Sink<T>>(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, sink: &mut S) -> bool {
        if with_slot {
            self.descend_right_with_slot(idx,
                                         |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included, sink)
            )
        } else {
            self.descend_right(idx,
                               |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included, sink)
            );

            false
        }
    }
}
