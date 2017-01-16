use base::{Key, Node, TreeWrapper, TreeBase, BulkDeleteCommon, ItemVisitor, KeyVal, righti, lefti, parenti, consume_unchecked};
use base::{TraversalDriver, TraversalDecision, RangeRefDriver, RangeDriver};
use std::ops::Range;
use std::ops::{Deref, DerefMut};
use std::fmt;

type Tree<K, V> = TreeWrapper<PlNode<K,V>>;


#[derive(Clone)]
pub struct PlNode<K: Key, V> {
    pub kv: KeyVal<K, V>,
}


impl<K: Key, V> Deref for PlNode<K, V> {
    type Target = KeyVal<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.kv
    }
}

impl<K: Key, V> DerefMut for PlNode<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.kv
    }
}

impl<K: Key, V> Node for PlNode<K, V> {
    type K = K;
    type V = V;

    fn new(key: K, val: V) -> Self {
        PlNode { kv: KeyVal::new(key, val) }
    }

    fn into_kv(self) -> KeyVal<K, V> {
        self.kv
    }
}

impl<K: Key+fmt::Debug, V> fmt::Debug for PlNode<K, V> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.kv.key, fmt)
    }
}



pub trait PlainDeleteInternal<K: Key, V> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    #[inline] fn delete(&mut self, search: &K) -> Option<V>;

    /// Deletes all items inside the half-open `range` from the tree and stores them in the output
    /// Vec. The items are returned in order.
    #[inline] fn delete_range(&mut self, range: Range<K>, output: &mut Vec<(K, V)>);

    /// Deletes all items inside the half-open `range` from the tree and stores them in the output Vec.
    #[inline] fn delete_range_ref(&mut self, range: Range<&K>, output: &mut Vec<(K, V)>);
}

impl<K: Key, V> PlainDeleteInternal<K, V> for TreeWrapper<PlNode<K,V>> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    #[inline]
    fn delete(&mut self, search: &K) -> Option<V> {
        self.index_of(search).map(|idx| {
            self.delete_idx(idx)
        })
    }

    /// Deletes all items inside the half-open `range` from the tree and stores them in the output
    /// Vec. The items are returned in order.
    #[inline]
    fn delete_range(&mut self, range: Range<K>, output: &mut Vec<(K, V)>) {
        debug_assert!(output.is_empty());
        output.truncate(0);

        self.delete_with_driver(&mut RangeDriver::new(range, output))
    }

    /// Deletes all items inside the half-open `range` from the tree and stores them in the output Vec.
    #[inline]
    fn delete_range_ref(&mut self, range: Range<&K>, output: &mut Vec<(K, V)>) {
        debug_assert!(output.is_empty());
        output.truncate(0);

        self.delete_with_driver(&mut RangeRefDriver::new(range, output))
    }
}



trait PlainDelete<K: Key, V>: TreeBase<PlNode<K,V>> {
    #[inline]
    fn delete_idx(&mut self, idx: usize) -> V {
        debug_assert!(!self.is_nil(idx));

        let node = self.take(idx);
        if self.has_left(idx) {
            self.delete_max(idx, lefti(idx));
        } else if self.has_right(idx) {
            self.delete_min(idx, righti(idx));
        }
        node.kv.val
    }


    #[inline]
    fn delete_max(&mut self, mut hole: usize, mut idx: usize) {
        loop {
            debug_assert!(self.is_nil(hole) && !self.is_nil(idx) && idx == lefti(hole));

            idx = self.find_max(idx);
            unsafe { self.move_from_to(idx, hole); }
            hole = idx;

            idx = lefti(idx);
            if self.is_nil(idx) {
                break;
            }
        }
    }

    #[inline]
    fn delete_min(&mut self, mut hole: usize, mut idx: usize) {
        loop {
            debug_assert!(self.is_nil(hole) && !self.is_nil(idx) && idx == righti(hole));

            idx = self.find_min(idx);
            unsafe { self.move_from_to(idx, hole); }
            hole = idx;

            idx = righti(idx);
            if self.is_nil(idx) {
                break;
            }
        }
    }
}


trait PlainDeleteRange<K: Key, V>: BulkDeleteCommon<PlNode<K, V>> {
    /// Delete based on driver decisions.
    /// The items are returned in order.
    #[inline]
    fn delete_with_driver<D: TraversalDriver<K, V>>(&mut self, drv: &mut D) {
        self.delete_range_loop(drv, 0);
        debug_assert!(self.slots_min().is_empty(), "slots_min={:?}", self.slots_min());
        debug_assert!(self.slots_max().is_empty());
    }

    #[inline]
    fn delete_range_loop<D: TraversalDriver<K, V>>(&mut self, drv: &mut D, mut idx: usize) {
        loop {
            if self.is_nil(idx) {
                return;
            }

            let decision = drv.decide(&self.key(idx));

            if decision.left() && decision.right() {
                let item = self.take(idx);
                let removed = self.descend_delete_max_left(drv, idx, true);
                consume_unchecked(drv.output(), item.into_kv());
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
    fn delete_range_min<D: TraversalDriver<K, V>>(&mut self, drv: &mut D, idx: usize) {
        let decision = drv.decide(&self.key(idx));
        debug_assert!(decision.left());

        if decision.right() {
            // the root and the whole left subtree are inside the range
            let item = self.take(idx);
            self.consume_subtree(lefti(idx), drv.output());
            consume_unchecked(drv.output(), item.into_kv());
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
    fn delete_range_max<D: TraversalDriver<K, V>>(&mut self, drv: &mut D, idx: usize) {
        let decision = drv.decide(&self.key(idx));
        debug_assert!(decision.right(), "idx={}", idx);

        if decision.left() {
            // the root and the whole right subtree are inside the range
            let item = self.take(idx);
            self.descend_delete_max_left(drv, idx, true);
            consume_unchecked(drv.output(), item.into_kv());
            self.consume_subtree(righti(idx), drv.output());
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
    fn descend_delete_min_left<D: TraversalDriver<K, V>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot,
                          |this: &mut Self, child_idx| this.delete_range_min(drv, child_idx))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_max_left<D: TraversalDriver<K, V>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot,
                          |this: &mut Self, child_idx| this.delete_range_max(drv, child_idx))
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_min_right<D: TraversalDriver<K, V>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
        self.descend_right(idx, with_slot,
                           |this: &mut Self, child_idx| this.delete_range_min(drv, child_idx))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_max_right<D: TraversalDriver<K, V>>(&mut self, drv: &mut D, idx: usize, with_slot: bool) -> bool {
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


pub struct NoUpdate;

impl<K: Key, V> ItemVisitor<PlNode<K, V>> for NoUpdate {
    type Tree = Tree<K,V>;

    #[inline]
    fn visit<F>(tree: &mut Self::Tree, idx: usize, mut f: F)
                                                where F: FnMut(&mut Self::Tree, usize) {
        f(tree, idx)
    }
}

impl<K: Key, V> BulkDeleteCommon<PlNode<K, V>> for Tree<K,V> {
    type Visitor = NoUpdate;
}

impl<K: Key, V> PlainDelete<K, V> for Tree<K,V> {}
impl<K: Key, V> PlainDeleteRange<K, V> for Tree<K,V> {}
