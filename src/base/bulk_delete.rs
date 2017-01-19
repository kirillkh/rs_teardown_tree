use base::{Node, TreeBase, lefti, righti};
use base::{SlotStack, Slot, Key, ItemFilter};
use base::drivers::consume_unchecked;


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

    #[inline]
    fn visit<F>(arg: &mut Self::Tree, idx: usize, f: F)
                                where F: FnMut(&mut Self::Tree, usize);
}



//==== methdos common to bulk-delete operations ====================================================
pub trait BulkDeleteCommon<N: Node>: TreeBase<N>+Sized  {
    type Visitor: ItemVisitor<N, Tree=Self>;

    //---- consume_subtree_* ---------------------------------------------------------------
//    #[inline]
//    fn consume_subtree<F>(&mut self, idx: usize, filter: &F, output: &mut Vec<(N::K, N::V)>)
//        where F: ItemFilter<N::K>
//    {
//        if F::is_noop() {
//            self.consume_subtree_unfiltered(idx, output);
//        } else {
//            self.consume_subtree_filtered(idx, filter, output);
//        }
//    }

    #[inline]
    fn consume_subtree_unfiltered(&mut self, root: usize, output: &mut Vec<(N::K, N::V)>) {
        self.traverse_inorder(root, output, |this, output, idx| {
            unsafe {
                this.move_to(idx, output);
            }
            false
        });
    }

    #[inline(never)]
    fn consume_subtree_filtered<F>(&mut self, idx: usize, filter: &mut F, output: &mut Vec<(N::K, N::V)>)
        where F: ItemFilter<N::K>
    {
        if self.is_nil(idx) {
            return;
        }

        // consume root if necessary
        let consumed = if filter.accept(&self.node(idx).key)
            { Some(self.take(idx)) }
            else
            { None };
        let mut removed = consumed.is_some();

        // left subtree
        removed = self.descend_left_fresh_slots(idx, removed,
                |this: &mut Self, child_idx| this.consume_subtree_filtered(child_idx, filter, output));

        if consumed.is_some() {
            consume_unchecked(output, consumed.unwrap().into_kv());
        }

        if !removed && self.slots_min().has_open() {
            removed = true;
            self.fill_slot_min(idx);
        }

        // right subtree
        removed = self.descend_right(idx, removed,
                |this: &mut Self, child_idx| this.consume_subtree_filtered(child_idx, filter, output));

        if !removed && self.slots_max().has_open() {
            removed = true;
            self.fill_slot_max(idx);
        }

        // fill the remaining open slots_max from the left subtree
        if removed {
            self.descend_fill_max_left(idx, true);
        }
     }


    //---- fill_slots_* -------------------------------------------------------------------
    #[inline(never)]
    fn fill_slots_min(&mut self, idx: usize) -> bool {
        debug_assert!(!self.is_nil(idx));

        if self.has_left(idx) {
            self.descend_fill_min_left(idx, false);
            if !self.slots_min().has_open() {
                return true;
            }
        }

        debug_assert!(self.slots_min().has_open());

        self.fill_slot_min(idx);

        let done = !self.descend_fill_min_right(idx, true);
        done || !self.slots_min().has_open()
    }


    #[inline(never)]
    fn fill_slots_max(&mut self, idx: usize) -> bool {
        debug_assert!(!self.is_nil(idx));

        if self.has_right(idx) {
            self.descend_fill_max_right(idx, false);

            if !self.slots_max().has_open() {
                return true;
            }
        }

        debug_assert!(self.slots_max().has_open());

        self.fill_slot_max(idx);

        let done = !self.descend_fill_max_left(idx, true);
        done || !self.slots_max().has_open()
    }


    #[inline(always)]
    fn fill_slot_min(&mut self, idx: usize) {
        debug_assert!(self.slots_min().has_open());
        let dst_idx = self.slots_min().fill();
        unsafe {
            self.move_from_to(idx, dst_idx);
        }
    }

    #[inline(always)]
    fn fill_slot_max(&mut self, idx: usize) {
        debug_assert!(self.slots_max().has_open());
        let dst_idx = self.slots_max().fill();
        unsafe {
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


    #[inline(always)]
    fn descend_fill_min_left(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot, |this: &mut Self, child_idx| {
            this.fill_slots_min(child_idx);
        })
    }

    #[inline(always)]
    fn descend_fill_max_left(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot, |this: &mut Self, child_idx| {
            this.fill_slots_max(child_idx);
        })
    }


    #[inline(always)]
    fn descend_fill_min_right(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_right(idx, with_slot, |this: &mut Self, child_idx| {
            this.fill_slots_min(child_idx);
        })
    }

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
        // this slots_max business is asymmetric (we don't do it in descend_delete_intersecting_ivl_right) because of the program flow: we enter the left subtree first
        let nfilled_orig = self.slots_max().nfilled;
        self.slots_max().nfilled = self.slots_max().nslots;

        let result = self.descend_left(idx, with_slot, f);

        debug_assert!(!self.slots_max().has_open());
        self.slots_max().nfilled = nfilled_orig;

        result
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