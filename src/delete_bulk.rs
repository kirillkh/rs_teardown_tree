use base::{ImplicitIntervalTree, Item, Node};

use std::mem;

pub trait TraversalDriver<T: Item> {
    fn decide(&mut self, node: &mut Node<T>) -> TraversalDecision;
}

#[derive(Clone, Copy)]
pub struct TraversalDecision {
    pub traverse_left: bool,
    pub traverse_right: bool,
    pub consume_curr: bool
}

struct RejectDriver;
impl<T: Item> TraversalDriver<T> for RejectDriver {
    fn decide(&mut self, node: &mut Node<T>) -> TraversalDecision {
        TraversalDecision { traverse_left: false, traverse_right: false, consume_curr: false }
    }
}



pub struct DeleteBulk<'a, T: 'a+Item> {
    tree: &'a mut ImplicitIntervalTree<T>,
    replacements_min: Vec<T>, replacements_max: Vec<T>,
    pub output: Vec<T>
}

impl<'a, T: Item> DeleteBulk<'a, T> {
    pub fn new(tree: &'a mut ImplicitIntervalTree<T>) -> DeleteBulk<'a, T> {
        let height = tree.node(0).height as usize;
        let replacements_min = Vec::with_capacity(height);
        let replacements_max = Vec::with_capacity(height);
        let output = Vec::with_capacity(tree.len());
        DeleteBulk { tree: tree, replacements_min: replacements_min, replacements_max: replacements_max, output: output }
    }

    pub fn delete_bulk<D: TraversalDriver<T>>(&mut self, drv: &mut D) {
        if !self.tree.is_null(0) {
            self.delete_bulk_recurse(drv, 0, 0, 0);
            assert!(self.replacements_min.is_empty() && self.replacements_max.is_empty());
        }
    }


    fn delete_bulk_recurse<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize,
                                                  min_reqs: usize, max_reqs: usize) {
        let decision = drv.decide(self.tree.node_mut(idx));
        if decision.consume_curr {
            let node = self.tree.node_mut(idx);
            let item = node.item.take().unwrap();
            self.output.push(item);
        }

        match (self.tree.has_left(idx), self.tree.has_right(idx)) {
            (false, false) => {
                self.traverse_shape_leaf(drv, idx, decision, min_reqs, max_reqs)
            }

            (true, false) => {
                self.traverse_shape_left(drv, idx, decision, min_reqs, max_reqs)
            }

            (false, true) => {
                self.traverse_shape_right(drv, idx, decision, min_reqs, max_reqs)
            }

            (true, true) => {
                self.traverse_shape_dual(drv, idx, decision, min_reqs, max_reqs)
            }
        }
    }


    //---- traverse_shape_* ------------------------------------------------------------------------
    #[inline]
    fn traverse_shape_leaf<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, decision: TraversalDecision,
                                                  min_reqs: usize, max_reqs: usize) {
        if self.traversal_forced(decision, min_reqs, max_reqs) {
            if decision.consume_curr {
                let node = self.tree.node_mut(idx);
                node.height = 0;
            } else if self.replacements_min.len() != min_reqs {
                let node = self.tree.node_mut(idx);
                let item = node.item.take().unwrap();
                self.replacements_min.push(item);
                node.height = 0;
            } else if self.replacements_max.len() != max_reqs {
                let node = self.tree.node_mut(idx);
                let item = node.item.take().unwrap();
                self.replacements_max.push(item);
                node.height = 0;
            }
        }
    }

    #[inline]
    fn traverse_shape_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, decision: TraversalDecision,
                                                  mut min_reqs: usize, mut max_reqs: usize) {
        if self.traversal_forced(decision, min_reqs, max_reqs) || decision.traverse_left {
            if decision.consume_curr {
                max_reqs += 1;
            }

            self.descend_minmax_left(drv, idx, min_reqs, max_reqs);

            // update height
            let height = if self.tree.node(idx).item.is_none() {
                0
            } else {
                let left = self.tree.node_mut(ImplicitIntervalTree::<T>::lefti(idx));
                left.height + 1
            };
            self.tree.node_mut(idx).height = height;
        }
    }

    #[inline]
    fn traverse_shape_right<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, decision: TraversalDecision,
                                                   mut min_reqs: usize, max_reqs: usize) {
        if decision.traverse_right || self.traversal_forced(decision, min_reqs, max_reqs) {
            if decision.consume_curr {
                min_reqs += 1;
            }

            self.descend_minmax_right(drv, idx, min_reqs, max_reqs);

            // update height
            let height = if self.tree.node(idx).item.is_none() {
                0
            } else {
                let right = self.tree.node_mut(ImplicitIntervalTree::<T>::righti(idx));
                right.height + 1
            };
            self.tree.node_mut(idx).height = height;
        }
    }

    #[inline]
    fn traverse_shape_dual<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, decision: TraversalDecision,
                                                  mut min_reqs: usize, mut max_reqs: usize) {
        // left subtree
        let max_reqs_left = if decision.consume_curr { 1 } else { 0 };
        if self.traversal_forced(decision, min_reqs, max_reqs_left) || decision.traverse_left {
            let len = self.replacements_max.len();
            let replacements_max_left = Self::subvec(&mut self.replacements_max, len);
            let replacements_max_orig = mem::replace(&mut self.replacements_max, replacements_max_left);
            {
                self.descend_minmax_left(drv, idx, min_reqs, max_reqs_left);
            }
            let replacements_max_left = mem::replace(&mut self.replacements_max, replacements_max_orig);
            mem::forget(replacements_max_left);
        }

        // right subtree
        let min_reqs_right = min_reqs + if self.tree.node(idx).item.is_none() { 1 } else { 0 };
        if decision.traverse_right || self.open_reqs(min_reqs_right, max_reqs) {
            self.descend_minmax_right(drv, idx, min_reqs_right, max_reqs);
        }

        // fulfill the remaining max requests from the left subtree
        {
            max_reqs += if self.tree.node(idx).item.is_none() { 1 } else { 0 };

            if max_reqs != self.replacements_max.len() && self.tree.has_left(idx) {
                self.take_max_left(idx, max_reqs)
            }
        }

        // update height
        if self.tree.node(idx).item.is_none() {
            self.tree.node_mut(idx).height = 0;
        } else {
            self.tree.update_height(idx);
        }
    }


    //---- descend_* -------------------------------------------------------------------------------
    #[inline]
    fn descend_minmax_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize,
                                                  min_reqs: usize, max_reqs: usize) {
        self.delete_bulk_recurse(drv, ImplicitIntervalTree::<T>::lefti(idx), min_reqs, max_reqs);

        let replace_max_requested = (max_reqs != self.replacements_max.len());
        if !replace_max_requested {
            let node = self.tree.node_mut(idx);
            if node.item.is_none() {
                node.item = self.replacements_max.pop();
                assert!(node.item.is_some());
            }
        }
    }

    #[inline]
    fn descend_minmax_right<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize,
                                                   min_reqs: usize, max_reqs: usize) {
        self.delete_bulk_recurse(drv, ImplicitIntervalTree::<T>::righti(idx), min_reqs, max_reqs);

        let replace_min_requested = (min_reqs != self.replacements_min.len());
        if !replace_min_requested {
            let node = self.tree.node_mut(idx);
            if node.item.is_none() {
                node.item = self.replacements_min.pop();
                assert!(node.item.is_some());
            }
        }
    }

    #[inline]
    fn take_max_left(&mut self, idx: usize, max_reqs: usize) {
        self.delete_bulk_recurse(&mut RejectDriver, ImplicitIntervalTree::<T>::lefti(idx), 0, max_reqs);
    }


    //---- helpers ---------------------------------------------------------------------------------
    #[inline]
    fn open_reqs(&self, min_reqs: usize, max_reqs: usize) -> bool {
        let replace_min_requested = (min_reqs != self.replacements_min.len());
        let replace_max_requested = (max_reqs != self.replacements_max.len());

        replace_min_requested | replace_max_requested
    }

    #[inline]
    fn traversal_forced(&self, decision: TraversalDecision,
                        min_reqs: usize, max_reqs: usize) -> bool {
        self.open_reqs(min_reqs, max_reqs) | decision.consume_curr
    }

    // Assumes that the returned vec will never be realloc'd!
    #[inline]
    fn subvec(v: &mut Vec<T>, from: usize) -> Vec<T> {
        let ptr = (&mut v[from..]).as_mut_ptr();
        unsafe {
            Vec::from_raw_parts(ptr, v.len() - from, v.capacity() - from)
        }
    }
}
