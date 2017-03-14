use base::{Node, TreeRepr, TreeDerefMut, TraverseMut, Sink, lefti, righti};
use base::{SlotStack, ItemFilter};

use std::mem;


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


pub trait ItemVisitor<N: Node>: Sized {
    type Tree: BulkDeleteCommon<N, Visitor=Self>;

    #[inline(always)]
    fn visit<F>(arg: &mut Self::Tree, idx: usize, f: F)
                                where F: FnMut(&mut Self::Tree, usize);
}



//==== methdos common to bulk-delete operations ====================================================
pub trait BulkDeleteCommon<N: Node>: TreeDerefMut<N>+Sized  {
    type Visitor: ItemVisitor<N, Tree=Self>;
    type Sink: Sink<(N::K, N::V)>;
    type Filter: ItemFilter<N::K>;

    #[inline(always)] fn filter_mut(&mut self) -> &mut Self::Filter;
    #[inline(always)] fn sink_mut(&mut self) -> &mut Self::Sink;

    //---- consume_subtree_* ---------------------------------------------------------------
    #[inline(always)]
    fn consume_subtree<S>(&mut self, idx: usize) {
        if Self::Filter::is_noop() {
            self.consume_subtree_unfiltered(idx);
        } else {
            self.consume_subtree_filtered(idx);
        }
    }

    #[inline]
    fn consume_subtree_unfiltered(&mut self, root: usize) {
        // work around the borrow checker (this is completely safe)
        let sink: &mut Self::Sink = unsafe { mem::transmute(self.sink_mut()) };

        TreeRepr::traverse_inorder_mut(self, root, &mut (), (), |this, _, idx| {
            this.move_to(idx, sink);
            None
        });
    }

    #[inline(never)]
    fn consume_subtree_filtered(&mut self, idx: usize) {
        if self.is_nil(idx) {
            return;
        }

        // This is safe: filter, which is taken by mutable references below, can not mutate `self`.
        let key = self.key_unsafe(idx);
        // consume root if necessary
        let consumed = if self.filter_mut().accept(key)
            { Some(self.take(idx)) }
            else
            { None };
        let mut removed = consumed.is_some();

        // left subtree
        removed = self.descend_consume_left(idx, removed);

        if consumed.is_some() {
            self.sink_mut().consume(consumed.unwrap().into_tuple())
        }

        if !removed && self.slots_min().has_open() {
            removed = true;
            self.fill_slot_min(idx);
        }

        // right subtree
        removed = self.descend_consume_right(idx, removed);

        if !removed && self.slots_max().has_open() {
            removed = true;
            self.fill_slot_max(idx);
        }

        // fill the remaining open slots_max from the left subtree
        if removed {
            self.descend_fill_max_left(idx, true);
        }
     }


    // The caller must make sure that `!is_nil(idx)`.
    #[inline(always)]
    fn filter_take(&mut self, idx: usize) -> Option<N> {
        // This is safe: filter, which is taken by mutable references below, can not mutate `self`.
        let key = self.key_unsafe(idx);
        if self.filter_mut().accept(key) {
            Some(self.take(idx))
        } else {
            None
        }
    }


    //---- fill_slots_* -------------------------------------------------------------------
    // The caller must make sure that `!is_nil(idx)` and there is an open `min_slot`.
    #[inline(never)]
    fn fill_slots_min(&mut self, idx: usize) -> bool {
        debug_assert!(!self.is_nil(idx));
        debug_assert!(self.slots_min().has_open());

        if self.has_left(idx) {
            self.descend_fill_min_left(idx, false);
            if !self.slots_min().has_open() {
                return true;
            }
        }

        self.fill_slot_min(idx);

        let done = !self.descend_fill_min_right(idx, true);
        done || !self.slots_min().has_open()
    }


    // The caller must make sure that `!is_nil(idx)` and there is an open `max_slot`.
    #[inline(never)]
    fn fill_slots_max(&mut self, idx: usize) -> bool {
        debug_assert!(!self.is_nil(idx));
        debug_assert!(self.slots_max().has_open());

        if self.has_right(idx) {
            self.descend_fill_max_right(idx, false);

            if !self.slots_max().has_open() {
                return true;
            }
        }

        self.fill_slot_max(idx);

        let done = !self.descend_fill_max_left(idx, true);
        done || !self.slots_max().has_open()
    }


    // The caller must make sure that `!is_nil(idx)` and there is an open `min_slot`.
    #[inline(always)]
    fn fill_slot_min(&mut self, idx: usize) {
        debug_assert!(!self.is_nil(idx));
        debug_assert!(self.slots_min().has_open());
        // Since there is an open `min_slot`, `dst_idx` points to an empty cell.
        let dst_idx = self.slots_min().fill();
        unsafe {
            // We are safe to call `move_from_to()`, as all its requirements are satisfied.
            self.move_from_to(idx, dst_idx);
        }
    }

    // The caller must make sure that `!is_nil(idx)` and there is an open `max_slot`.
    #[inline(always)]
    fn fill_slot_max(&mut self, idx: usize) {
        debug_assert!(self.slots_max().has_open());
        let dst_idx = self.slots_max().fill();
        // Since there is an open `max_slot`, `dst_idx` points to an empty cell.
        unsafe {
            // We are safe to call `move_from_to()`, as all its requirements are satisfied.
            self.move_from_to(idx, dst_idx);
        }
    }



    //---- descend_* -------------------------------------------------------------------------------


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_left<F>(&mut self, idx: usize, with_slot: bool, f: F) -> bool
                                                    where F: FnMut(&mut Self, usize) {
        debug_assert!(self.is_nil(idx) == with_slot, "idx={}, with_slot={}", idx, with_slot);
        let child_idx = lefti(idx);
        if self.is_nil(child_idx) {
            return with_slot;
        }

        if with_slot {
            self.slots_max().push(idx);

            Self::Visitor::visit(self, child_idx, f);

            self.slots_max().pop();
            self.is_nil(idx)
        } else {
            Self::Visitor::visit(self, child_idx, f);
            false
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_right<F>(&mut self, idx: usize, with_slot: bool, f: F) -> bool
                                                    where F: FnMut(&mut Self, usize) {
        debug_assert!(self.is_nil(idx) == with_slot);
        let child_idx = righti(idx);
        if self.is_nil(child_idx) {
            return with_slot;
        }

        if with_slot {
            self.slots_min().push(idx);

            Self::Visitor::visit(self, child_idx, f);

            self.slots_min().pop();
            self.is_nil(idx)
        } else {
            Self::Visitor::visit(self, child_idx, f);
            false
        }
    }


    // The caller must make sure that there is an open min_slot.
    #[inline(always)]
    fn descend_fill_min_left(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot, |this: &mut Self, child_idx| {
            this.fill_slots_min(child_idx);
        })
    }

    // The caller must make sure that there is an open max_slot or `with_slot=true`.
    #[inline(always)]
    fn descend_fill_max_left(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot, |this: &mut Self, child_idx| {
            this.fill_slots_max(child_idx);
        })
    }


    // The caller must make sure that there is an open `min_slot` or `with_slot=true`.
    #[inline(always)]
    fn descend_fill_min_right(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_right(idx, with_slot, |this: &mut Self, child_idx| {
            this.fill_slots_min(child_idx);
        })
    }

    // The caller must make sure that there is an open max_slot.
    #[inline(always)]
    fn descend_fill_max_right(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_right(idx, with_slot, |this: &mut Self, child_idx| {
            this.fill_slots_max(child_idx);
        })
    }


    #[inline(always)]
    fn descend_left_fresh_slots<F>(&mut self, idx: usize, with_slot: bool, f: F) -> bool
        where F: FnMut(&mut Self, usize)
    {
        // this slots_max business is asymmetric (we don't do it in descend_delete_overlap_ivl_right) because of the program flow: we enter the left subtree first
        let nfilled_orig = self.slots_max().nfilled;
        self.slots_max().nfilled = self.slots_max().nslots;

        let result = self.descend_left(idx, with_slot, f);

        debug_assert!(!self.slots_max().has_open());
        self.slots_max().nfilled = nfilled_orig;

        result
    }


    #[inline(always)]
    fn descend_consume_left(&mut self, idx: usize, with_slot: bool) -> bool {
        if Self::Filter::is_noop() {
            self.consume_subtree_unfiltered(lefti(idx));
            with_slot
        } else {
            self.descend_left_fresh_slots(idx, with_slot,
                                          |this: &mut Self, child_idx| this.consume_subtree_filtered(child_idx))
        }
    }

    #[inline(always)]
    fn descend_consume_right(&mut self, idx: usize, with_slot: bool) -> bool {
        if Self::Filter::is_noop() {
            self.consume_subtree_unfiltered(righti(idx));
            with_slot
        } else {
            self.descend_right(idx, with_slot,
                               |this: &mut Self, child_idx| this.consume_subtree_filtered(child_idx))
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