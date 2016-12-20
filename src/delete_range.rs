use base::{TeardownTreeInternal, Item, Node, lefti, righti};
use drivers::{TraversalDriver, TraversalDecision, Sink};
use slot_stack::SlotStack;
use std::mem;


pub trait DeleteRange<T: Item> {
    fn delete_range(&mut self, from: T::Key, to: T::Key, output: &mut Vec<T>);
}

pub trait BulkDeleteCommon<T: Item> {
    fn consume_subtree<S: Sink<T>>(&mut self, root: usize, sink: &mut S);

    fn fill_slots_min(&mut self, idx: usize) -> bool;
    fn fill_slots_max(&mut self, idx: usize) -> bool;

    fn fill_slot_min(&mut self, idx: usize);
    fn fill_slot_max(&mut self, idx: usize);

    fn descend_left<F>(&mut self, idx: usize, f: F)
                            where F: FnMut(&mut Self, usize);
    fn descend_right<F>(&mut self, idx: usize, f: F)
                            where F: FnMut(&mut Self, usize);

    fn descend_left_with_slot<F>(&mut self, idx: usize, f: F) -> bool
                            where F: FnMut(&mut Self, usize);
    fn descend_right_with_slot<F>(&mut self, idx: usize, f: F) -> bool
                            where F: FnMut(&mut Self, usize);

    fn descend_fill_left(&mut self, idx: usize) -> bool;
    fn descend_fill_right(&mut self, idx: usize) -> bool;


    fn slots_min(&mut self) -> &mut SlotStack;
    fn slots_max(&mut self) -> &mut SlotStack;

    #[inline(always)]
    fn node_unsafe<'b>(&self, idx: usize) -> &'b Node<T>;
}

pub trait DeleteRangeInternal<T: Item> {
    fn delete_range<D: TraversalDriver<T>>(&mut self, drv: &mut D);
    fn delete_range_loop<D: TraversalDriver<T>>(&mut self, drv: &mut D, mut idx: usize);

    fn delete_range_min<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize);
    fn delete_range_max<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize);

    fn descend_delete_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool;
    fn descend_delete_right<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool;
}


pub struct DeleteRangeCache {
    pub slots_min: SlotStack, pub slots_max: SlotStack,   // TODO: we no longer need both stacks, one is enough... but removing one causes 3% performance regression
}

impl Clone for DeleteRangeCache {
    fn clone(&self) -> Self {
        debug_assert!(self.slots_min.is_empty() && self.slots_max.is_empty());
        let capacity = self.slots_max.capacity;
        DeleteRangeCache::new(capacity)
    }
}

impl DeleteRangeCache {
    pub fn new(height: usize) -> DeleteRangeCache {
        let slots_min = SlotStack::new(height);
        let slots_max = SlotStack::new(height);
        DeleteRangeCache { slots_min: slots_min, slots_max: slots_max }
    }
}


//pub struct DeleteRange<T: Item> {
//    pub tree: TeardownTreeInternal<T>,
////    pub slots_min: SlotStack, pub slots_max: SlotStack,
//    pub output: Vec<T>
//}

//==== generic methods =============================================================================
impl<T: Item> BulkDeleteCommon<T> for TeardownTreeInternal<T> {
//    //---- helpers ---------------------------------------------------------------------------------
//    #[inline(always)]
//    pub fn node(&self, idx: usize) -> &Node<T> {
//        self.tree.node(idx)
//    }
//
//    #[inline(always)]
//    pub fn item(&mut self, idx: usize) -> &T {
//        &self.node(idx).item
//    }
//
//    #[inline(always)]
//    pub fn slots_min(&mut self) -> &mut SlotStack {
//        &mut self.tree.delete_range_cache.slots_min
//    }
//
//    #[inline(always)]
//    pub fn slots_max(&mut self) -> &mut SlotStack {
//        &mut self.tree.delete_range_cache.slots_max
//    }

    //---- consume_subtree_* ---------------------------------------------------------------
    #[inline]
    fn consume_subtree<S: Sink<T>>(&mut self, root: usize, sink: &mut S) {
        self.traverse_inorder(root, sink, |tree, sink, idx| {
            unsafe {
                tree.move_to(idx, sink);
            }
        });
    }


    //---- fill_slots_* -------------------------------------------------------------------
    #[inline(never)]
    fn fill_slots_min(&mut self, idx: usize) -> bool {
        debug_assert!(!self.is_nil(idx));

        if self.has_left(idx) {
            let done = self.fill_slots_min(lefti(idx));
            if done {
                return true;
            }
        }

        debug_assert!(self.slots_min().has_open());

        self.fill_slot_min(idx);

        let done = !self.descend_fill_right(idx);
        done || !self.slots_min().has_open()
    }


    #[inline(never)]
    fn fill_slots_max(&mut self, idx: usize) -> bool {
        debug_assert!(!self.is_nil(idx));

        if self.has_right(idx) {
            let done = self.fill_slots_max(righti(idx));
            if done {
                return true;
            }
        }

        debug_assert!(self.slots_max().has_open());

        self.fill_slot_max(idx);

        let done = !self.descend_fill_left(idx);
        done || !self.slots_max().has_open()
    }


    #[inline(always)]
    fn fill_slot_min(&mut self, idx: usize) {
        let dst_idx = self.slots_min().fill(idx);
        unsafe {
            self.move_from_to(idx, dst_idx);
        }
    }

    #[inline(always)]
    fn fill_slot_max(&mut self, idx: usize) {
        let dst_idx = self.slots_max().fill(idx);
        unsafe {
            self.move_from_to(idx, dst_idx);
        }
    }



    //---- descend_* -------------------------------------------------------------------------------


    #[inline(always)]
    fn descend_left<F>(&mut self, idx: usize, mut f: F)
                                            where F: FnMut(&mut Self, usize) {
        let child_idx = lefti(idx);
        if !self.is_nil(child_idx) {
            f(self, child_idx);
        }
    }

    #[inline(always)]
    fn descend_right<F>(&mut self, idx: usize, mut f: F)
                                            where F: FnMut(&mut Self, usize) {
        let child_idx = righti(idx);
        if !self.is_nil(child_idx) {
            f(self, child_idx);
        }
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_left_with_slot<F>(&mut self, idx: usize, mut f: F) -> bool
                                            where F: FnMut(&mut Self, usize) {
        debug_assert!(self.is_nil(idx));

        let child_idx = lefti(idx);
        if self.is_nil(child_idx) {
            return true;
        }

        self.slots_max().push(idx);

        f(self, child_idx);

        self.slots_max().pop();
        self.is_nil(idx)
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_right_with_slot<F>(&mut self, idx: usize, mut f: F) -> bool
                                            where F: FnMut(&mut Self, usize) {
        debug_assert!(self.is_nil(idx));

        let child_idx = righti(idx);
        if self.is_nil(child_idx) {
            return true;
        }

        self.slots_min().push(idx);

        f(self, child_idx);

        self.slots_min().pop();
        self.is_nil(idx)
    }


    #[inline(always)]
    fn descend_fill_left(&mut self, idx: usize) -> bool {
        self.descend_left_with_slot(idx, |this: &mut Self, child_idx| {
            this.fill_slots_max(child_idx);
        })
    }

    #[inline(always)]
    fn descend_fill_right(&mut self, idx: usize) -> bool {
        self.descend_right_with_slot(idx, |this: &mut Self, child_idx| {
            this.fill_slots_min(child_idx);
        })
    }

    fn slots_min(&mut self) -> &mut SlotStack {
        &mut self.delete_range_cache.slots_min
    }

    fn slots_max(&mut self) -> &mut SlotStack {
        &mut self.delete_range_cache.slots_max
    }

    #[inline(always)]
    fn node_unsafe<'b>(&self, idx: usize) -> &'b Node<T> {
        unsafe {
            mem::transmute(self.node(idx))
        }
    }
}



//==== delete_range() v5 ===========================================================================
impl<T: Item> DeleteRangeInternal<T> for TeardownTreeInternal<T> {
    /// The items are returned in order.
    fn delete_range<D: TraversalDriver<T>>(&mut self, drv: &mut D) {
        self.delete_range_loop(drv, 0);
        debug_assert!(self.slots_min().is_empty() && self.slots_max().is_empty());
    }


    #[inline]
    fn delete_range_loop<D: TraversalDriver<T>>(&mut self, drv: &mut D, mut idx: usize) {
        loop {
            if self.is_nil(idx) {
                return;
            }

            let decision = drv.decide(&self.item(idx).key());

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
        let decision = drv.decide(&self.item(idx).key());
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
        let decision = drv.decide(&self.item(idx).key());
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



//pub trait DeleteBulkDrv<K: Ord> {
//    type Decision: TraversalDecision;
//
//    fn delete_bulk(self);
//
//    #[inline(always)]
//    fn decide(&self, key: &K) -> Self::Decision;
//}
//
//
//
////==== delete_bulk() - a more general (and slower) version of the algorithm that allows to traverse nodes without consuming them ====
//impl<'a, T: Item> DeleteRange<'a, T> {
//    /// The items are returned in order.
//    pub fn delete_bulk<D: TraversalDriver<T>>(mut self, drv: &mut D) {
//        self.delete_bulk_loop(drv, 0);
//        debug_assert!(self.slots_min().is_empty() && self.slots_max().is_empty());
//
//        self.tree.delete_range_cache = Some(DeleteRangeCache { slots_min: self.slots_min(), slots_max: self.slots_max() });
//    }
//
//
//    #[inline]
//    fn delete_bulk_loop<D: TraversalDriver<T>>(&mut self, drv: &D, mut idx: usize) {
//        loop {
//            if self.tree.is_nil(idx) {
//                return;
//            }
//
//            let decision = drv.decide(&self.item(idx).key());
//
//            if decision.left() && decision.right() {
//                let item = self.tree.take(idx);
//                let removed = self.descend_delete_left(drv, idx, true);
//                self.output.push(item);
//                self.descend_delete_right(drv, idx, removed);
//                return;
//            } else if decision.left() {
//                idx = lefti(idx);
//            } else {
//                debug_assert!(decision.right());
//                idx = righti(idx);
//            }
//        }
//    }
//}