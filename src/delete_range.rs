use base::{TeardownTree, TeardownTreeInternal, Item, Node};
use slot_stack::SlotStack;

use std::mem;

pub trait TraversalDriver<T: Item> {
    #[inline(always)]
    fn decide(&self, key: &T::Key) -> TraversalDecision;
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
            self.delete_range_rec(drv, 0);
            debug_assert!(self.slots_min.is_empty() && self.slots_max.is_empty() && self.delete_subtree_stack.is_empty());
        }

        self.tree.delete_range_cache = Some(DeleteRangeCache { slots_min: self.slots_min, slots_max: self.slots_max, delete_subtree_stack: self.delete_subtree_stack });
    }


    //---- descend_* -------------------------------------------------------------------------------

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_left<F>(&mut self, idx: usize, f: F) -> bool
                                                        where F: Fn(&mut Self, usize) {
        debug_assert!(self.node(idx).item.is_none());

        let child_idx = Self::lefti(idx);
        if self.tree.is_null(child_idx) {
            return true;
        }

        self.slots_max.push();

        f(self, child_idx);

        let slot = self.slots_max.pop();

        if slot.is_some() {
            debug_assert!(self.node(idx).item.is_none());
            self.node_mut(idx).item = slot;
            false
        } else {
            true
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_right<F>(&mut self, idx: usize, f: F) -> bool
                                                        where F: Fn(&mut Self, usize) {
        debug_assert!(self.node(idx).item.is_none());

        let child_idx = Self::righti(idx);
        if self.tree.is_null(child_idx) {
            return true;
        }

        self.slots_min.push();

        f(self, child_idx);

        let slot = self.slots_min.pop();

        if slot.is_some() {
            debug_assert!(self.node(idx).item.is_none());
            self.node_mut(idx).item = slot;
            false
        } else {
            true
        }
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


// v5
impl<'a, T: Item> DeleteRange<'a, T> {
    #[inline(never)]
    fn delete_range_rec<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize) {
        if self.tree.is_null(idx) {
            return;
        }

        let node = self.node_mut(idx);
        let decision = drv.decide(node.item.as_ref().unwrap().key());

        if decision.left && decision.right {
            self.consume(node);
            let removed = self.delete_range_descend_left(drv, idx);
            if removed {
                self.delete_range_descend_right(drv, idx);
            } else {
                let child_idx = Self::righti(idx);
                if !self.tree.is_null(child_idx) {
                    self.delete_range_min(drv, child_idx);
                }
            }
        } else if decision.left {
            self.delete_range_rec(drv, Self::lefti(idx));
        } else {
            debug_assert!(decision.right);
            self.delete_range_rec(drv, Self::righti(idx));
        }
    }

    #[inline(never)]
    fn delete_range_min<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize) {
        let node = self.node_mut(idx);
        let decision = drv.decide(node.item.as_ref().unwrap().key());
        debug_assert!(decision.left);

        if decision.right {
            // the root and the whole left subtree are inside the range
            self.consume(node);
            self.consume_subtree(Self::lefti(idx));
            self.delete_range_descend_right(drv, idx);
        } else {
            // the root and the right subtree are outside the range
            let child_idx = Self::lefti(idx);
            if !self.tree.is_null(child_idx) {
                self.delete_range_min(drv, child_idx);
            }

            if self.slots_min.has_open() {
                self.slots_min.fill(node.item.take().unwrap());

                self.descend_right(idx, |this: &mut Self, child_idx| {
                    this.fill_slots_min(child_idx);
                });
            }
        }
    }

    #[inline(never)]
    fn delete_range_max<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize) {
        let node = self.node_mut(idx);
        let decision = drv.decide(node.item.as_ref().unwrap().key());
        debug_assert!(decision.right);

        if decision.left {
            // the root and the whole right subtree are inside the range
            self.consume(node);
            self.consume_subtree(Self::righti(idx));
            self.delete_range_descend_left(drv, idx);
        } else {
            // the root and the left subtree are outside the range
            let child_idx = Self::righti(idx);
            if !self.tree.is_null(child_idx) {
                self.delete_range_max(drv, child_idx);
            }

            if self.slots_max.has_open() {
                self.slots_max.fill(node.item.take().unwrap());

                self.descend_left(idx, |this: &mut Self, child_idx| {
                    this.fill_slots_max(child_idx);
                });
            }
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn delete_range_descend_left<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize) -> bool {
        self.descend_left(idx, |this: &mut Self, child_idx|
            this.delete_range_max(drv, child_idx)
        )
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn delete_range_descend_right<D: TraversalDriver<T>>(&mut self, drv: &D, idx: usize) -> bool {
        self.descend_right(idx, |this: &mut Self, child_idx|
            this.delete_range_min(drv, child_idx)
        )
    }


    //---- delete_subtree_* ---------------------------------------------------------------
    #[inline]
    fn consume_subtree(&mut self, root: usize) {
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
            self.slots_min.fill(item);

            self.descend_right(idx, |this, child_idx| {
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
            self.slots_max.fill(item);

            self.descend_left(idx, |this, child_idx| {
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
}
