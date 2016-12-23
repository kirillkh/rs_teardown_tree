use applied::interval::{Interval, IntervalNode};
use base::{TeardownTree, TeardownTreeInternal, lefti, righti, Sink};
use base::drivers::{consume_ptr, consume_unchecked};
use base::BulkDeleteCommon;
use base::InternalAccess;
use std::mem;

pub trait IntervalTeardownTree<Iv: Interval> {
    fn delete(&mut self, search: &IntervalNode<Iv>) -> Option<Iv>;
    fn delete_intersecting(&mut self, search: &Iv, idx: usize, output: &mut Vec<Iv>);
}

trait IntervalTreeInternal<Iv: Interval> {
    fn delete_intersecting_ivl_rec<S: Sink<IntervalNode<Iv>>>(&mut self, search: &Iv, idx: usize,
                                                              min_included: bool, sink: &mut S);
    fn descend_delete_intersecting_ivl_left<S: Sink<IntervalNode<Iv>>>(&mut self, search: &Iv, idx: usize, with_slot: bool,
                                                                       min_included: bool, sink: &mut S) -> bool;
    fn descend_delete_intersecting_ivl_right<S: Sink<IntervalNode<Iv>>>(&mut self, search: &Iv, idx: usize, with_slot: bool,
                                                                        min_included: bool, sink: &mut S) -> bool;
}



trait IntervalDeleteRange<Iv: Interval> {
    fn delete_with_driver<S: Sink<IntervalNode<Iv>>>(&mut self, drv: &mut S);

    fn delete_range<S: Sink<IntervalNode<Iv>>>(&mut self, drv: &mut S);
    fn delete_range_loop<S: Sink<IntervalNode<Iv>>>(&mut self, drv: &mut S, idx: usize);

    fn delete_range_min<S: Sink<IntervalNode<Iv>>>(&mut self, drv: &mut S, idx: usize);
    fn delete_range_max<S: Sink<IntervalNode<Iv>>>(&mut self, drv: &mut S, idx: usize);

    fn descend_delete_left<S: Sink<IntervalNode<Iv>>>(&mut self, drv: &mut S, idx: usize, with_slot: bool) -> bool;
    fn descend_delete_right<S: Sink<IntervalNode<Iv>>>(&mut self, drv: &mut S, idx: usize, with_slot: bool) -> bool;
}

trait PlainDelete<T: Ord> {
    #[inline] fn delete_idx(&mut self, idx: usize) -> T;
    #[inline] fn delete_max(&mut self, idx: usize) -> T;
    #[inline] fn delete_min(&mut self, idx: usize) -> T;
}



impl<Iv: Interval> IntervalTeardownTree<Iv> for TeardownTree<IntervalNode<Iv>> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    // TODO: accepting IntervalNode is super ugly, temporary solution only
    fn delete(&mut self, search: &IntervalNode<Iv>) -> Option<Iv> {
        self.internal().index_of(search).map(|idx| {
            self.internal().delete_idx(idx)
        }).map(|node| node.ivl)
    }

    #[inline]
    fn delete_intersecting(&mut self, search: &Iv, idx: usize, output: &mut Vec<Iv>) {
        self.internal().delete_intersecting_ivl_rec(search, idx, false, &mut self::IntervalSink { output: output });
    }
}


impl<T: Ord> PlainDelete<T> for TeardownTreeInternal<T> {
    #[inline]
    fn delete_idx(&mut self, idx: usize) -> T {
        debug_assert!(!self.is_nil(idx));

        match (self.has_left(idx), self.has_right(idx)) {
            (false, false) => {
                self.take(idx)
            },

            (true, false)  => {
                let left_max = self.delete_max(lefti(idx));
                mem::replace(self.item_mut(idx), left_max)
            },

            (false, true)  => {
                let right_min = self.delete_min(righti(idx));
                mem::replace(self.item_mut(idx), right_min)
            },

            (true, true)   => {
                let left_max = self.delete_max(lefti(idx));
                mem::replace(self.item_mut(idx), left_max)
            },
        }
    }


    #[inline]
    fn delete_max(&mut self, mut idx: usize) -> T {
        idx = self.find_max(idx);

        if self.has_left(idx) {
            let left_max = self.delete_max(lefti(idx));
            mem::replace(self.item_mut(idx), left_max)
        } else {
            self.take(idx)
        }
    }

    #[inline]
    fn delete_min(&mut self, mut idx: usize) -> T {
        idx = self.find_min(idx);

        if self.has_right(idx) {
            let right_min = self.delete_min(righti(idx));
            mem::replace(self.item_mut(idx), right_min)
        } else {
            self.take(idx)
        }
    }
}



impl<Iv: Interval> IntervalTreeInternal<Iv> for TeardownTreeInternal<IntervalNode<Iv>> {
    fn delete_intersecting_ivl_rec<S: Sink<IntervalNode<Iv>>>(&mut self, search: &Iv, idx: usize, mut min_included: bool, sink: &mut S) {
        let k: &IntervalNode<Iv> = &self.node_unsafe(idx).item;

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
    fn descend_delete_intersecting_ivl_left<S: Sink<IntervalNode<Iv>>>(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, sink: &mut S) -> bool {
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
    fn descend_delete_intersecting_ivl_right<S: Sink<IntervalNode<Iv>>>(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, sink: &mut S) -> bool {
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


struct IntervalSink<'a, Iv: Interval+'a> {
    output: &'a mut Vec<Iv>
}

impl<'a, Iv: Interval> Sink<IntervalNode<Iv>> for IntervalSink<'a, Iv> {
    fn consume(&mut self, item: IntervalNode<Iv>) {
        self.output.push(item.ivl)
    }

    fn consume_unchecked(&mut self, item: IntervalNode<Iv>) {
        consume_unchecked(&mut self.output, item.ivl);
    }

    fn consume_ptr(&mut self, src: *const IntervalNode<Iv>) {
        let p = unsafe { &(*src).ivl };
        consume_ptr(&mut self.output, p)
    }
}
