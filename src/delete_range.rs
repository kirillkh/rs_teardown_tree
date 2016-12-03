use base::{TeardownTree, Item, Node};
use std::ptr::Unique;

use std::{mem, ptr};
use std::fmt::{Debug, Formatter};

pub trait TraversalDriver<T: Item> {
    #[inline(always)]
    fn decide(&self, node: &T) -> TraversalDecision;
}

#[derive(Clone, Copy, Debug)]
pub struct TraversalDecision {
    pub left: bool,
    pub right: bool,
}

impl TraversalDecision {
    #[inline(always)]
    pub fn consume(&self) -> bool {
        self.left && self.right
    }
}

pub struct DeleteRangeCache<T: Item> {
    pub slots_min: SlotStack<T>, pub slots_max: SlotStack<T>,
    pub delete_subtree_stack: Vec<usize>, // TODO: this can be made a little faster by avoiding bounds checking (cf. SlotStack)
}

impl<T: Item> Clone for DeleteRangeCache<T> {
    fn clone(&self) -> Self {
        debug_assert!(self.slots_min.is_empty() && self.slots_max.is_empty() && self.delete_subtree_stack.is_empty());
        let capacity = self.slots_max.capacity;
        DeleteRangeCache::new(capacity)
    }
}

impl<T: Item> DeleteRangeCache<T> {
    pub fn new(height: usize) -> DeleteRangeCache<T> {
        let slots_min = SlotStack::new(height);
        let slots_max = SlotStack::new(height);
        let delete_subtree_stack = Vec::with_capacity(height);
        DeleteRangeCache { slots_min: slots_min, slots_max: slots_max, delete_subtree_stack: delete_subtree_stack }
    }
}



type Slot<T> = Option<T>;

pub struct SlotStack<T: Item> {
    nslots: usize,
    nfilled: usize,
    slots: Unique<T>,
    capacity: usize
}

impl<T: Item> SlotStack<T> {
    fn new(capacity: usize) -> SlotStack<T> {
        unsafe {
            let mut slots = vec![mem::uninitialized(); capacity];
            let ptr: *mut T = slots.as_mut_ptr();
            mem::forget(slots);
            SlotStack { nslots: 0, nfilled: 0, slots: Unique::new(ptr), capacity: capacity }
        }
    }

    #[inline(always)]
    fn push_slot(&mut self) {
        debug_assert!(self.nslots < self.capacity);
        self.nslots += 1;
    }

    #[inline(always)]
    fn pop(&mut self) -> Slot<T> {
        debug_assert!(self.nslots > 0);
        if self.nfilled == self.nslots {
            self.nfilled -= 1;
            self.nslots -= 1;
            unsafe {
                Some(ptr::read(self.slot_at(self.nslots) as *const T))
            }
        } else {
            self.nslots -= 1;
            None
        }
    }

    #[inline(always)]
    fn fill_slot(&mut self, item: T) {
        debug_assert!(self.nfilled < self.nslots);
        *self.slot_at(self.nfilled) = item;
        self.nfilled += 1;
    }

    #[inline(always)]
    fn fill_slot_opt(&mut self, item: Option<T>) {
        debug_assert!(item.is_some());
        debug_assert!(self.nfilled < self.nslots);
        *self.slot_at(self.nfilled) = item.unwrap();
        self.nfilled += 1;
    }


    #[inline(always)]
    fn slot_at(&self, idx: usize) -> &mut T {
        unsafe {
            mem::transmute(self.slots.offset(idx as isize))
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.nslots == 0
    }

    #[inline(always)]
    pub fn nslots(&self) -> usize {
        self.nslots
    }

    #[inline(always)]
    pub fn nfilled(&self) -> usize {
        self.nfilled
    }

    #[inline(always)]
    pub fn has_open(&self) -> bool {
        self.nslots != self.nfilled
    }
}

impl <T: Item+Debug> Debug for SlotStack<T> {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        unsafe {
            let ptr: *mut Slot<T> = mem::transmute(self.slots.get());
            let slots_vec = Vec::from_raw_parts(ptr, self.capacity, self.capacity);
            let result = write!(fmt, "SlotStack: nslots={}, nfilled={}, slots={:?}", self.nslots, self.nfilled, slots_vec);
            mem::forget(slots_vec);
            result
        }
    }
}

impl <T: Item> Drop for SlotStack<T> {
    fn drop(&mut self) {
        unsafe {
            Vec::from_raw_parts(self.slots.get_mut(), self.nfilled, self.capacity);
            // let it drop
        }
    }
}




pub struct DeleteRange<'a, T: 'a+Item> {
    tree: &'a mut TeardownTree<T>,
    slots_min: SlotStack<T>, slots_max: SlotStack<T>,
    delete_subtree_stack: Vec<usize>, // TODO: this can be made a little faster by avoiding bounds checking (cf. SlotStack)
    pub output: &'a mut Vec<T>
}

impl<'a, T: Item> DeleteRange<'a, T> {
    pub fn new(tree: &'a mut TeardownTree<T>, output: &'a mut Vec<T>) -> DeleteRange<'a, T> {
        let cache = tree.delete_range_cache.take().unwrap();
        DeleteRange { tree: tree, slots_min: cache.slots_min, slots_max: cache.slots_max, output: output, delete_subtree_stack: cache.delete_subtree_stack }
    }

    /// The items are not guaranteed to be returned in any particular order.
    pub fn delete_range<D: TraversalDriver<T>>(mut self, drv: &mut D) {
//        // TEST
//        let orig = self.tree.clone();

        if !self.tree.is_null(0) {
            self.delete_range_rec(drv, 0, false, false);
            debug_assert!(self.slots_min.is_empty() && self.slots_max.is_empty() && self.delete_subtree_stack.is_empty());
        }

        self.tree.delete_range_cache = Some(DeleteRangeCache { slots_min: self.slots_min, slots_max: self.slots_max, delete_subtree_stack: self.delete_subtree_stack });
    }


    //---- descend_* -------------------------------------------------------------------------------

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_left<F>(&mut self, idx: usize, push_slot: bool, f: F) -> bool
        where F: Fn(&mut Self, usize) {
        if push_slot {
            debug_assert!(self.node(idx).item.is_none());
            self.slots_max.push_slot()
        }

        f(self, Self::lefti(idx));

        // TODO: we do not handle correctly the case where after return from recursion there are some open min_reqs.
        // That is because it's a case that doesn't happen with range queries.

        if push_slot {
            let slot = self.slots_max.pop();

            if slot.is_some() {
                debug_assert!(self.node(idx).item.is_none());
                self.node_mut(idx).item = slot;
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_right<F>(&mut self, idx: usize, push_slot: bool, f: F) -> bool
                                                        where F: Fn(&mut Self, usize) {
        if push_slot {
            debug_assert!(self.node(idx).item.is_none());
            self.slots_min.push_slot()
        }

        f(self, Self::righti(idx));

        // TODO: we do not handle correctly the case where after return from recursion there are some open min_reqs.
        // That is because it's a case that doesn't happen with range queries.

        if push_slot {
            let slot = self.slots_min.pop();

            if slot.is_some() {
                debug_assert!(self.node(idx).item.is_none());
                self.node_mut(idx).item = slot;
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn delete_range_descend_left<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize,
                                                        push_slot: bool,
                                                        min_included: bool, max_included: bool) -> bool {
        self.descend_left(idx, push_slot,
                          |this, child_idx| this.delete_range_rec(drv, child_idx, min_included, max_included)
        )
    }



    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn delete_range_descend_right<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize,
                                                         push_slot: bool,
                                                         min_included: bool, max_included: bool) -> bool {
        self.descend_right(idx, push_slot,
                           |this, child_idx| this.delete_range_rec(drv, child_idx, min_included, max_included)
        )
    }

    //---- helpers ---------------------------------------------------------------------------------
    #[inline(always)]
    fn node(&self, idx: usize) -> &Node<T> {
        self.tree.node(idx)
    }

    #[inline(always)]
    fn node_mut<'b>(&mut self, idx: usize) -> &'b mut Node<T> {
        unsafe {
            mem::transmute(self.tree.node_mut(idx))
        }
    }

    #[inline(always)]
    fn item(&mut self, idx: usize) -> &Option<T> {
        &self.node(idx).item
    }

    #[inline(always)]
    fn item_mut(&mut self, idx: usize) -> &mut Option<T> {
        &mut self.node_mut(idx).item
    }

    #[inline(always)]
    fn lefti(idx: usize) -> usize {
        TeardownTree::<T>::lefti(idx)
    }

    #[inline(always)]
    fn righti(idx: usize) -> usize {
        TeardownTree::<T>::righti(idx)
    }

    #[inline(always)]
    fn consume(&mut self, node: &mut Node<T>) {
        let item = node.item.take().unwrap();
        self.output.push(item);
    }
}




// delete_range_recurse 3
impl<'a, T: Item> DeleteRange<'a, T> {
    #[inline(never)]
    fn delete_range_rec<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize,
                                               min_included: bool, max_included: bool) {
        if min_included && max_included {
            self.delete_subtree(idx);
            return;
        }

        let item: &mut Option<T> = &mut self.node_mut(idx).item;
        let decision = drv.decide(item.as_ref().unwrap());
        match (decision.left, decision.right) {
            (true, false)  => self.traverse_left(drv, idx,
                                                 min_included, max_included),
            (false, true)  => self.traverse_right(drv, idx,
                                                  min_included, max_included),
            (true, true)   => self.traverse_dual(drv, idx,
                                                 min_included, max_included),
            (false, false) =>
                if self.slots_min.has_open() {
                    self.fill_slots_min(idx);
                } else {
                    debug_assert!(self.slots_max.has_open());
                    self.fill_slots_max(idx);
                }
        }
    }


    //---- delete_subtree_* ---------------------------------------------------------------
    #[inline]
    fn delete_subtree(&mut self, root: usize) {
        debug_assert!(self.delete_subtree_stack.is_empty());

        if self.tree.is_null(root) {
            return;
        }

        let mut next = root;

        loop {
            next = {
                let node = &mut self.node_mut(next);
                let item: &mut Option<T> = &mut node.item;
                let item = item.take().unwrap();
                self.output.push(item);

                match (self.tree.has_left(next), self.tree.has_right(next)) {
                    (false, false) => {
                        if let Some(n) = self.delete_subtree_stack.pop() {
                            n
                        } else {
                            break;
                        }
                    },

                    (true, false)  => {
                        Self::lefti(next)
                    },

                    (false, true)  => {
                        Self::righti(next)
                    },

                    (true, true)   => {
                        debug_assert!(self.delete_subtree_stack.len() < self.delete_subtree_stack.capacity());

                        self.delete_subtree_stack.push(Self::righti(next));
                        Self::lefti(next)
                    },
                }
            };
        }
    }


    //---- fill_slots_* -------------------------------------------------------------------
    #[inline(never)]
    fn fill_slots_min(&mut self, idx: usize) -> bool {
        if self.tree.is_null(idx) {
            return false;
        }

        let done = self.fill_slots_min(Self::lefti(idx));

        if done {
            true
        } else {
            debug_assert!(self.slots_min.has_open());

            let node = self.node_mut(idx);
            let item = node.item.take().unwrap();
            self.slots_min.fill_slot(item);

            self.descend_right(idx, true, |this, child_idx| {
                this.fill_slots_min(child_idx);
            });
            let done = node.item.is_some();
            if done {
                true
            } else {
                !self.slots_min.has_open()
            }
        }
    }


    #[inline(never)]
    fn fill_slots_max(&mut self, idx: usize) -> bool {
        if self.tree.is_null(idx) {
            return false;
        }

        let done = self.fill_slots_max(Self::righti(idx));

        if done {
            true
        } else {
            debug_assert!(self.slots_max.has_open());

            let node = self.node_mut(idx);
            let item = node.item.take().unwrap();
            self.slots_max.fill_slot(item);

            self.descend_left(idx, true, |this, child_idx| {
                this.fill_slots_max(child_idx);
            });
            let done = node.item.is_some();
            if done {
                true
            } else {
                !self.slots_max.has_open()
            }
        }
    }


    //---- traverse_* ---------------------------------------------------------------------
    #[inline(always)]
    fn traverse_left<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize,
                                            min_included: bool, max_included: bool) {
        if self.tree.has_left(idx) {
            self.delete_range_rec(drv, Self::lefti(idx), min_included, max_included);
        }

        self.fill_minmax_after_left_traverse(idx);
    }


    /// mirrors traverse_left()
    #[inline(always)]
    fn traverse_right<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize,
                                             min_included: bool, max_included: bool) {
        if self.tree.has_right(idx) {
            self.delete_range_rec(drv, Self::righti(idx), min_included, max_included);
        }

        self.fill_minmax_after_right_traverse(idx);
    }


    #[inline(always)]
    fn fill_minmax_after_left_traverse(&mut self, idx: usize) {
        let node = self.node_mut(idx);
        let mut removed = false;

        if self.slots_min.has_open() {
            // fill a slot_min with this node's item
            let item = node.item.take();
            self.slots_min.fill_slot_opt(item);
            removed = true;
        }

        let (min_open, max_open) = (removed, self.slots_max.has_open());
        if min_open || max_open {
            if self.tree.has_right(idx) {
                removed = self.descend_right(idx, min_open, |this, child_idx| { this.fill_slots_min(child_idx); } );
            }

            if removed && self.slots_max.has_open() && self.tree.has_left(idx) {
                self.descend_left(idx, true, |this, child_idx| { this.fill_slots_max(child_idx); } );
            }
        }
    }


    #[inline(always)]
    fn fill_minmax_after_right_traverse(&mut self, idx: usize) {
        let node = self.node_mut(idx);
        let mut removed = false;

        if self.slots_max.has_open() {
            // fill a slot_max with this node's item
            let item = node.item.take();
            self.slots_max.fill_slot_opt(item);
            removed = true;
        }

        let (max_open, min_open) = (removed, self.slots_min.has_open());
        if min_open || max_open {
            if self.tree.has_left(idx) {
                removed = self.descend_left(idx, max_open, |this, child_idx| { this.fill_slots_max(child_idx); } );
            }

            if removed && self.slots_min.has_open() && self.tree.has_right(idx) {
                self.descend_right(idx, true, |this, child_idx| { this.fill_slots_min(child_idx); } );
            }
        }
    }


    #[inline(always)]
    fn traverse_dual<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize,
                                            min_included: bool, max_included: bool) {
        let node = self.node_mut(idx);
        self.consume(node);
        let mut removed = true;

        if self.tree.has_right(idx) {
            removed = self.delete_range_descend_right(drv, idx, true,
                                                      true, max_included);
        }

        if self.tree.has_left(idx) {
            removed = self.delete_range_descend_left(drv, idx, removed,
                                                     min_included, true);
        }

        if removed {
            // this node was consumed, and both subtrees are empty now
            debug_assert!(!self.tree.has_left(idx));
            debug_assert!(!self.tree.has_right(idx));
        }
    }
}
