use base::{TeardownTree, TeardownTreeInternal, righti, lefti, parenti, InternalAccess};
use base::{TraversalDriver, TraversalDecision, RangeRefDriver, RangeDriver};
use base::{BulkDeleteCommon, TreeInternal};
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


pub trait PlainDelete<T: Ord>: TreeInternal<T> {
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
impl<T: Ord> PlainDelete<T> for TeardownTreeInternal<T> {}

pub trait PlainDeleteRange<T: Ord>: BulkDeleteCommon<T> {
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
        debug_assert!(self.slots_min().is_empty(), "slots_min={:?}", self.slots_min());
        debug_assert!(self.slots_max().is_empty());
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
                let removed = self.descend_delete_max_left(drv, idx, true);
                drv.consume_unchecked(item);
                self.descend_delete_min_right(drv, idx, removed);
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
            self.descend_delete_min_right(drv, idx, true);
        } else {
            // the root and the right subtree are outside the range
            self.descend_delete_min_left(drv, idx, false);

            if self.slots_min().has_open() {
                self.fill_slot_min(idx);
                self.descend_fill_min_right(idx, true);
//                self.descend_right(idx, true, |this: &mut Self, child_idx| {
//                    this.fill_slots_min2(child_idx);
//                });
            }
        }
    }

    #[inline(never)]
    fn delete_range_max<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize) {
        let decision = drv.decide(&self.item(idx));
        debug_assert!(decision.right(), "idx={}", idx);

        if decision.left() {
            // the root and the whole right subtree are inside the range
            let item = self.take(idx);
            self.descend_delete_max_left(drv, idx, true);
            drv.consume_unchecked(item);
            self.consume_subtree(righti(idx), drv);
        } else {
            // the root and the left subtree are outside the range
            self.descend_delete_max_right(drv, idx, false);

            if self.slots_max().has_open() {
                self.fill_slot_max(idx);
                self.descend_fill_max_left(idx, true);
            }
        }
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_min_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot,
                          |this: &mut Self, child_idx| this.delete_range_min(drv, child_idx))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_max_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot,
                          |this: &mut Self, child_idx| this.delete_range_max(drv, child_idx))
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_min_right<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
        self.descend_right(idx, with_slot,
                           |this: &mut Self, child_idx| this.delete_range_min(drv, child_idx))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_max_right<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
        self.descend_right(idx, with_slot,
                           |this: &mut Self, child_idx| this.delete_range_max(drv, child_idx))
    }



    fn fill_slots_min2(&mut self, root: usize) {
        debug_assert!(!self.is_nil(root));

        struct State {
            prev: usize,
            stopped: bool
        }

        let mut state = State { prev:0, stopped:false };
        self.traverse_inorder(root, &mut state,
            |this: &mut Self, state, idx| {
                // unwind the stack to the current node
                if idx < state.prev {
                    let mut curr = state.prev;
                    while idx != curr {
                        debug_assert!(idx < curr);
                        debug_assert!(curr&1==0 || parenti(curr) == idx);

                        this.slots_min().pop();
                        curr = parenti(curr);
                    }
                    debug_assert!(idx == curr);
                }
                state.prev = idx;

                if this.slots_min().has_open() {
                    this.fill_slot_min(idx);
                    this.slots_min().push(idx);
                    false
                } else {
                    state.stopped = true;
                    true
                }
            }
        );

        let mut curr = state.prev;
        while root != curr {
            debug_assert!(root < curr);
            if curr & 1 == 0 {
                self.slots_min().pop();
            }
            curr = parenti(curr);
        }

        if !state.stopped {
            self.slots_min().pop();
        }
    }
}
impl<T: Ord> PlainDeleteRange<T> for TeardownTreeInternal<T> {}
