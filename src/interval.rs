use drivers::{TraversalDriver, TraversalDecision};
use std::cmp::Ordering;
use std::mem;

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




use delete_range::{DeleteRange, DeleteRangeCache};
use base::{Item, Node, TeardownTree, TeardownTreeInternal, lefti, righti};
use slot_stack::SlotStack;


pub trait DeleteIntersectingIntervals<Iv: Interval> {
    fn delete_intersecting_ivl(&mut self, search: &Iv, idx: usize);
}

trait DeleteIntersectingIntervalsInternal<Iv: Interval, T: Item<Key=IntervalNode<Iv>>> {
    fn delete_intersecting_ivl_rec(&mut self, search: &Iv, idx: usize, min_included: bool);
    fn descend_delete_intersecting_ivl_left(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool) -> bool;
    fn descend_delete_intersecting_ivl_right(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool) -> bool;
    fn node(&self, idx: usize) -> &Node<T>;
    fn node_unsafe<'a>(&self, idx: usize) -> &'a Node<T>;
    fn item(&mut self, idx: usize) -> &T;
    fn slots_min(&self) -> &SlotStack;
    fn slots_max(&self) -> &SlotStack;
    fn tree(&self) -> &TeardownTree<T>;
    fn tree_mut(&mut self) -> &mut TeardownTree<T>;
}


impl<Iv: Interval, T: Item<Key=IntervalNode<Iv>>> DeleteIntersectingIntervals<Iv> for DeleteRange<T> {
    #[inline]
    fn delete_intersecting_ivl(&mut self, search: &Iv, idx: usize) {
        self.delete_intersecting_ivl_rec(search, idx, false);
    }
}

impl<Iv: Interval, T: Item<Key=IntervalNode<Iv>>> DeleteIntersectingIntervalsInternal<Iv, T> for DeleteRange<T> {
    fn delete_intersecting_ivl_rec(&mut self, search: &Iv, idx: usize, mut min_included: bool) {
        let k: &IntervalNode<Iv> = self.node_unsafe(idx).item.key();

        if k.max() <= search.a() {
            // whole subtree outside the range
            if self.slots_min.has_open() {
                self.fill_slots_min(idx);
            }
            if self.slots_max.has_open() && !self.tree().is_null(idx) {
                self.fill_slots_max(idx);
            }
        } else if search.b() <= k.a() {
            // root and right are outside the range
            self.descend_delete_intersecting_ivl_left(search, idx, false, min_included);

            let removed = if self.slots_min.has_open() {
                self.slots_min.fill(self.tree, idx);
                self.descend_fill_right(idx)
            } else {
                false
            };

            if self.slots_max.has_open() {
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
                { Some(self.tree_mut().take(idx)) }
            else
                { None };

            // left subtree
            let mut removed = consume;
            if consume {
                if min_included {
                    self.consume_subtree(lefti(idx))
                } else {
                    removed = self.descend_delete_intersecting_ivl_left(search, idx, true, false);
                }

                self.output.push(item.unwrap());
            } else {
                removed = self.descend_delete_intersecting_ivl_left(search, idx, false, min_included);
            }

            // right subtree
            min_included = min_included || search.a() <= k.a();
            if min_included {
                let max_included = k.max() <= search.b();
                if max_included {
                    self.consume_subtree(righti(idx));
                } else {
                    removed = self.descend_delete_intersecting_ivl_right(search, idx, removed, min_included);
                }
            } else {
                removed = self.descend_delete_intersecting_ivl_right(search, idx, removed, false);
            }

            // fill the remaining open slots_max from the left subtree
            if removed && self.slots_max.has_open() {
                self.descend_fill_left(righti(idx));
            }
        }
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_intersecting_ivl_left(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool) -> bool {
        if with_slot {
            self.descend_left_with_slot(idx,
                                        |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included)
            )
        } else {
            self.descend_left(idx,
                              |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included)
            );

            false
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_intersecting_ivl_right(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool) -> bool {
        if with_slot {
            self.descend_right_with_slot(idx,
                                         |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included)
            )
        } else {
            self.descend_right(idx,
                               |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included)
            );

            false
        }
    }


    #[inline(always)]
    fn node(&self, idx: usize) -> &Node<T> {
        self.tree().node(idx)
    }

    #[inline(always)]
    fn node_unsafe<'b>(&self, idx: usize) -> &'b Node<T> {
        unsafe {
            mem::transmute(self.tree().node(idx))
        }
    }


    #[inline(always)]
    fn item(&mut self, idx: usize) -> &T {
        &self.tree_mut().node(idx).item
    }


    fn slots_min(&self) -> &SlotStack {
        &self.slots_min
    }

    fn slots_max(&self) -> &SlotStack {
        &self.slots_max
    }

    fn tree(&self) -> &TeardownTree<T> {
        self.tree
    }

    fn tree_mut(&mut self) -> &mut TeardownTree<T> {
        self.tree
    }
}
