use base::{TeardownTreeInternal, Node, lefti, righti};
use base::Sink;
use base::SlotStack;
use std::mem;


pub trait BulkDeleteCommon<T: Ord> {
    fn consume_subtree<S: Sink<T>>(&mut self, root: usize, sink: &mut S);

    fn fill_slots_min(&mut self, idx: usize) -> bool;
    fn fill_slots_max(&mut self, idx: usize) -> bool;

    fn fill_slot_min(&mut self, idx: usize);
    fn fill_slot_max(&mut self, idx: usize);

    fn descend_left<F>(&mut self, idx: usize, with_slot: bool, f: F) -> bool
                            where F: FnMut(&mut Self, usize);
    fn descend_right<F>(&mut self, idx: usize, with_slot: bool, f: F) -> bool
                            where F: FnMut(&mut Self, usize);

    fn descend_fill_left(&mut self, idx: usize, with_slot: bool) -> bool;
    fn descend_fill_right(&mut self, idx: usize, with_slot: bool) -> bool;


    fn slots_min(&mut self) -> &mut SlotStack;
    fn slots_max(&mut self) -> &mut SlotStack;

    #[inline(always)]
    fn node_unsafe<'b>(&self, idx: usize) -> &'b Node<T>;
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
impl<T: Ord> BulkDeleteCommon<T> for TeardownTreeInternal<T> {
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
            false
        });
    }


    //---- fill_slots_* -------------------------------------------------------------------
    #[inline(never)]
    fn fill_slots_min(&mut self, idx: usize) -> bool {
        debug_assert!(!self.is_nil(idx));

        if self.has_left(idx) {
            self.descend_left(idx, false, |this: &mut Self, child_idx| {
                this.fill_slots_min(child_idx);
            });
            if !self.slots_min().has_open() {
                return true;
            }
        }

        debug_assert!(self.slots_min().has_open());

        self.fill_slot_min(idx);

        let done = !self.descend_fill_right(idx, true);
        done || !self.slots_min().has_open()
    }


    #[inline(never)]
    fn fill_slots_max(&mut self, idx: usize) -> bool {
        debug_assert!(!self.is_nil(idx));

        if self.has_right(idx) {
            !self.descend_right(idx, false, |this: &mut Self, child_idx| {
                this.fill_slots_max(child_idx);
            });
            if !self.slots_max().has_open() {
                return true;
            }
        }

        debug_assert!(self.slots_max().has_open());

        self.fill_slot_max(idx);

        let done = !self.descend_fill_left(idx, true);
        done || !self.slots_max().has_open()
    }


    #[inline(always)]
    fn fill_slot_min(&mut self, idx: usize) {
        let dst_idx = self.slots_min().fill();
        unsafe {
            self.move_from_to(idx, dst_idx);
        }
    }

    #[inline(always)]
    fn fill_slot_max(&mut self, idx: usize) {
        let dst_idx = self.slots_max().fill();
        unsafe {
            self.move_from_to(idx, dst_idx);
        }
    }



    //---- descend_* -------------------------------------------------------------------------------


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_left<F>(&mut self, idx: usize, with_slot: bool, mut f: F) -> bool
                                                    where F: FnMut(&mut Self, usize) {
        let child_idx = lefti(idx);
        if self.is_nil(child_idx) {
            debug_assert!(self.is_nil(idx) == with_slot);
            return with_slot;
        }

        if with_slot {
            self.slots_max().push(idx);

            f(self, child_idx);

            self.slots_max().pop();
            self.is_nil(idx)
        } else {
            f(self, child_idx);
            false
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_right<F>(&mut self, idx: usize, with_slot: bool, mut f: F) -> bool
                                                    where F: FnMut(&mut Self, usize) {
        let child_idx = righti(idx);
        if self.is_nil(child_idx) {
            debug_assert!(self.is_nil(idx) == with_slot);
            return with_slot;
        }

        if with_slot {
            self.slots_min().push(idx);

            f(self, child_idx);

            self.slots_min().pop();
            self.is_nil(idx)
        } else {
            f(self, child_idx);
            false
        }
    }


    #[inline(always)]
    fn descend_fill_left(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot, |this: &mut Self, child_idx| {
            this.fill_slots_max(child_idx);
        })
    }

    #[inline(always)]
    fn descend_fill_right(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_right(idx, with_slot, |this: &mut Self, child_idx| {
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