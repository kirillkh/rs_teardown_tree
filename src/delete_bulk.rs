use base::{ImplicitTree, Item, Node};

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
    fn decide(&mut self, _: &mut Node<T>) -> TraversalDecision {
        TraversalDecision { traverse_left: false, traverse_right: false, consume_curr: false }
    }
}


enum ItemListId {
    MIN, MAX, OUT
}

pub struct DeleteBulk<'a, T: 'a+Item> {
    tree: &'a mut ImplicitTree<T>,
    replacements_min: Vec<T>, replacements_max: Vec<T>,
    pub output: &'a mut Vec<T>
}

impl<'a, T: Item> DeleteBulk<'a, T> {
    pub fn new(tree: &'a mut ImplicitTree<T>, output: &'a mut Vec<T>) -> DeleteBulk<'a, T> {
        let height = tree.node(0).height as usize;
        let replacements_min = Vec::with_capacity(height);
        let replacements_max = Vec::with_capacity(height);
        DeleteBulk { tree: tree, replacements_min: replacements_min, replacements_max: replacements_max, output: output }
    }

    pub fn delete_bulk<D: TraversalDriver<T>>(&mut self, drv: &mut D) {
//        // TEST
//        let orig = self.tree.clone();

        if !self.tree.is_null(0) {
            self.delete_bulk_recurse(drv, 0, 0, 0);
            assert!(self.replacements_min.is_empty() && self.replacements_max.is_empty(),
                    "tree: {:?}, replacements_min: {:?}, replacements_max: {:?}, output: {:?}", self.tree, self.replacements_min, self.replacements_max, self.output);
        }
    }


    fn delete_bulk_recurse<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize,
                                                  min_reqs: usize, max_reqs: usize) {
        let decision = drv.decide(self.tree.node_mut(idx));
        if decision.consume_curr {
            self.take_item(idx, ItemListId::OUT);
        }

        match (self.tree.has_left(idx), self.tree.has_right(idx)) {
            (false, false) => {
                self.traverse_shape_leaf(idx, decision, min_reqs, max_reqs)
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
    fn traverse_shape_leaf(&mut self, idx: usize, decision: TraversalDecision,
                           min_reqs: usize, max_reqs: usize) {
        if self.traversal_forced(decision, min_reqs, max_reqs) {
            if decision.consume_curr {
                let node = self.tree.node_mut(idx);
                node.height = 0;
            } else if self.open_min_reqs(min_reqs) {
                let node = self.take_item(idx, ItemListId::MIN);
                node.height = 0;
            } else if self.open_max_reqs(max_reqs) {
                let node = self.take_item(idx, ItemListId::MAX);
                node.height = 0;
            }
        }
    }

    #[inline]
    fn traverse_shape_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, decision: TraversalDecision,
                                                  min_reqs: usize, mut max_reqs: usize) {
        if self.traversal_forced(decision, min_reqs, max_reqs) || decision.traverse_left {
            if decision.consume_curr {
                max_reqs += 1;
            } else if self.open_max_reqs(max_reqs) {
                max_reqs += 1;
                self.take_item(idx, ItemListId::MAX);
            };

            self.descend_minmax_left(drv, idx, min_reqs, max_reqs);

            // fulfill a min req if necessary
            if self.tree.node(idx).item.is_some() && self.open_min_reqs(min_reqs) {
                self.take_item(idx, ItemListId::MIN);
            }

            // update height
            let height = if self.tree.node(idx).item.is_none() {
                0
            } else {
                let left = self.tree.node_mut(ImplicitTree::<T>::lefti(idx));
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
            } else if self.open_min_reqs(min_reqs) {
                min_reqs += 1;
                self.take_item(idx, ItemListId::MIN);
            }

            self.descend_minmax_right(drv, idx, min_reqs, max_reqs);

            // fulfill a max req if necessary
            if self.tree.node(idx).item.is_some() && self.open_max_reqs(max_reqs) {
                self.take_item(idx, ItemListId::MAX);
            }

            // update height
            let height = if self.tree.node(idx).item.is_none() {
                0
            } else {
                let right = self.tree.node_mut(ImplicitTree::<T>::righti(idx));
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
        if decision.traverse_left || self.traversal_forced(decision, min_reqs, max_reqs_left) {
            let len = self.replacements_max.len();
            let replacements_max_left = Self::subvec(&mut self.replacements_max, len);
            let replacements_max_orig = mem::replace(&mut self.replacements_max, replacements_max_left);
            {
                self.descend_minmax_left(drv, idx, min_reqs, max_reqs_left);
            }
            let replacements_max_left = mem::replace(&mut self.replacements_max, replacements_max_orig);
            mem::forget(replacements_max_left);
        }

        // this node
        if self.tree.node(idx).item.is_none() {
            // this node was consumed, and the left subtree did not have a replacement
            min_reqs += 1;
        } else if self.open_min_reqs(min_reqs) {
            // this node is the minimum of the tree: use it to fulfill a min request
            min_reqs += 1;
            self.take_item(idx, ItemListId::MIN);
        }

        // right subtree
        if decision.traverse_right || self.open_reqs(min_reqs, max_reqs) {
            self.descend_minmax_right(drv, idx, min_reqs, max_reqs);
        }

        // this node again
        if self.tree.node(idx).item.is_none() {
            max_reqs += 1;
        } else if self.open_max_reqs(max_reqs) {
            // this node is the maximum of the tree: use it to fulfill a max request
            max_reqs += 1;
            self.take_item(idx, ItemListId::MAX);
        }

        // fulfill the remaining max requests from the left subtree
        if self.open_max_reqs(max_reqs) && self.tree.has_left(idx) {
            self.take_max_left(idx, max_reqs)
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
        self.delete_bulk_recurse(drv, ImplicitTree::<T>::lefti(idx), min_reqs, max_reqs);

        // TODO: we do not handle correctly the case where after return from recursion there are some open min_reqs.
        // That is because it's a case that doesn't happen with range queries.

        if !self.open_max_reqs(max_reqs) {
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
        self.delete_bulk_recurse(drv, ImplicitTree::<T>::righti(idx), min_reqs, max_reqs);

        if !self.open_min_reqs(min_reqs) {
            let node = self.tree.node_mut(idx);
            if node.item.is_none() {
                node.item = self.replacements_min.pop();
                assert!(node.item.is_some());
            }
        }
    }

    #[inline]
    fn take_max_left(&mut self, idx: usize, max_reqs: usize) {
        self.descend_minmax_left(&mut RejectDriver, idx, 0, max_reqs);
    }


    //---- helpers ---------------------------------------------------------------------------------
    #[inline]
    fn open_reqs(&self, min_reqs: usize, max_reqs: usize) -> bool {
        self.open_min_reqs(min_reqs) || self.open_max_reqs(max_reqs)
    }

    #[inline]
    fn open_min_reqs(&self, min_reqs: usize) -> bool {
        min_reqs != self.replacements_min.len()
    }

    #[inline]
    fn open_max_reqs(&self, max_reqs: usize) -> bool {
        max_reqs != self.replacements_max.len()
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


    #[inline]
    fn take_item(&mut self, idx: usize, list_id: ItemListId) -> &mut Node<T> {
        let node = self.tree.node_mut(idx);
        let item = node.item.take().unwrap();
        let mut move_to = match list_id {
            ItemListId::MIN => &mut self.replacements_min,
            ItemListId::MAX => &mut self.replacements_max,
            ItemListId::OUT => &mut self.output
        };
        assert!(move_to.len() < move_to.capacity()); // there should be no dynamic allocation here! we pre-allocate enough space in all 3 vecs
        move_to.push(item);
        node
    }
}
