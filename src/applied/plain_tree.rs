use base::{TeardownTree, TeardownTreeInternal, righti, lefti, InternalAccess};
use base::{TraversalDriver, TraversalDecision, RangeRefDriver, RangeDriver};
use base::BulkDeleteCommon;
use std::mem;

pub trait PlainTeardownTree<T: Ord> {
    fn delete(&mut self, search: &T) -> Option<T>;
    fn delete_range(&mut self, from: T, to: T, output: &mut Vec<T>);
    fn delete_range_ref(&mut self, from: &T, to: &T, output: &mut Vec<T>);
}

impl<T: Ord> PlainTeardownTree<T> for TeardownTree<T> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    fn delete(&mut self, search: &T) -> Option<T> {
        self.internal().delete(search)
    }

    /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output
    /// Vec. The items are returned in order.
    fn delete_range(&mut self, from: T, to: T, output: &mut Vec<T>) {
        self.internal().delete_range(from, to, output)
    }

    /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output Vec.
    fn delete_range_ref(&mut self, from: &T, to: &T, output: &mut Vec<T>) {
        self.internal().delete_range_ref(from, to, output)
    }
}


pub trait PlainDeleteRange<T: Ord> {
    /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output
    /// Vec. The items are returned in order.
    fn delete_range(&mut self, from: T, to: T, output: &mut Vec<T>);

    /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output Vec.
    fn delete_range_ref(&mut self, from: &T, to: &T, output: &mut Vec<T>);

    fn delete_with_driver<D: TraversalDriver<T>>(&mut self, drv: &mut D);

    fn delete_range_loop<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize);

    fn delete_range_min<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize);
    fn delete_range_max<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize);

    fn descend_delete_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool;
    fn descend_delete_right<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool;
}

pub trait PlainDelete<T: Ord> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    fn delete(&mut self, search: &T) -> Option<T>;

    #[inline] fn delete_idx(&mut self, idx: usize) -> T;
    #[inline] fn delete_max(&mut self, idx: usize) -> T;
    #[inline] fn delete_min(&mut self, idx: usize) -> T;
}



impl<T: Ord> PlainDelete<T> for TeardownTreeInternal<T> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    fn delete(&mut self, search: &T) -> Option<T> {
        self.index_of(search).map(|idx| {
            self.delete_idx(idx)
        })
    }


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


impl<T: Ord> PlainDeleteRange<T> for TeardownTreeInternal<T> {
    /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output
    /// Vec. The items are returned in order.
    #[inline]
    fn delete_range(&mut self, from: T, to: T, output: &mut Vec<T>) {
        debug_assert!(output.is_empty());
        output.truncate(0);

        self.delete_with_driver(&mut RangeDriver::new(from, to, output))
    }

    /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output Vec.
    #[inline]
    fn delete_range_ref(&mut self, from: &T, to: &T, output: &mut Vec<T>) {
        debug_assert!(output.is_empty());
        output.truncate(0);

        self.delete_with_driver(&mut RangeRefDriver::new(from, to, output))
    }


    /// Delete based on driver decisions.
    /// The items are returned in order.
    #[inline]
    fn delete_with_driver<D: TraversalDriver<T>>(&mut self, drv: &mut D) {
        self.delete_range_loop(drv, 0);
        debug_assert!(self.slots_min().is_empty() && self.slots_max().is_empty());
    }

    #[inline]
    fn delete_range_loop<D: TraversalDriver<T>>(&mut self, drv: &mut D, mut idx: usize) {
        loop {
            if self.is_nil(idx) {
                return;
            }

            let decision = drv.decide(&self.item(idx));

            if decision.left() && decision.right() {
                let item = self.take(idx);
                let removed = self.descend_delete_left(drv, idx, true);
                drv.consume_unchecked(item);
                self.descend_delete_right(drv, idx, removed);
                return;
            } else if decision.left() {
                idx = lefti(idx);
            } else {
                debug_assert!(decision.right());
                idx = righti(idx);
            }
        }
    }

    #[inline(never)]
    fn delete_range_min<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize) {
        let decision = drv.decide(&self.item(idx));
        debug_assert!(decision.left());

        if decision.right() {
            // the root and the whole left subtree are inside the range
            let item = self.take(idx);
            self.consume_subtree(lefti(idx), drv);
            drv.consume_unchecked(item);
            self.descend_delete_right(drv, idx, true);
        } else {
            // the root and the right subtree are outside the range
            self.descend_left(idx,
                              |this: &mut Self, child_idx| this.delete_range_min(drv, child_idx)
            );

            if self.slots_min().has_open() {
                self.fill_slot_min(idx);
                self.descend_fill_right(idx);
            }
        }
    }

    #[inline(never)]
    fn delete_range_max<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize) {
        let decision = drv.decide(&self.item(idx));
        debug_assert!(decision.right());

        if decision.left() {
            // the root and the whole right subtree are inside the range
            let item = self.take(idx);
            self.descend_delete_left(drv, idx, true);
            drv.consume_unchecked(item);
            self.consume_subtree(righti(idx), drv);
        } else {
            // the root and the left subtree are outside the range
            self.descend_right(idx,
                               |this: &mut Self, child_idx| this.delete_range_max(drv, child_idx)
            );

            if self.slots_max().has_open() {
                self.fill_slot_max(idx);
                self.descend_fill_left(idx);
            }
        }
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
        if with_slot {
            self.descend_left_with_slot(idx,
                                        |this: &mut Self, child_idx| this.delete_range_max(drv, child_idx)
            )
        } else {
            self.descend_left(idx,
                              |this: &mut Self, child_idx| this.delete_range_max(drv, child_idx)
            );

            false
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_right<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
        if with_slot {
            self.descend_right_with_slot(idx,
                                         |this: &mut Self, child_idx| this.delete_range_min(drv, child_idx)
            )
        } else {
            self.descend_right(idx,
                               |this: &mut Self, child_idx| this.delete_range_min(drv, child_idx)
            );

            false
        }
    }
}
