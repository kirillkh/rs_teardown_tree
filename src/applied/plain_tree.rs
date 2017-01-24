use base::{Key, Node, TreeRepr, Traverse, TeardownTreeRefill, BulkDeleteCommon, ItemVisitor, KeyVal, righti, lefti, parenti, consume_unchecked};
use base::{ItemFilter, TraversalDriver, TraversalDecision, RangeRefDriver, RangeDriver, NoopFilter};
use std::ops::Range;
use std::ops::{Deref, DerefMut};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::cell::UnsafeCell;
use std::{fmt, ptr, mem};

pub struct PlTree<K: Key, V> {
    pub repr: UnsafeCell<TreeRepr<PlNode<K, V>>>,
}

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


impl<K: Key, V> PlTree<K, V> {
    /// Constructs a new PlTree
    pub fn new(items: Vec<(K, V)>) -> PlTree<K, V> {
        PlTree::with_repr(TreeRepr::new(items))
    }

    pub fn with_repr(repr: TreeRepr<PlNode<K, V>>) -> PlTree<K, V> {
        PlTree { repr: UnsafeCell::new(repr) }
    }

    /// Constructs a new PlTree
    /// Note: the argument must be sorted!
    pub fn with_sorted(sorted: Vec<(K, V)>) -> PlTree<K, V> {
        PlTree::with_repr(TreeRepr::with_sorted(sorted))
    }

    pub fn with_nodes(nodes: Vec<Option<PlNode<K, V>>>) -> PlTree<K, V> {
        PlTree::with_repr(TreeRepr::with_nodes(nodes))
    }

    /// Deletes the item with the given key from the tree and returns it (or None).
    #[inline]
    pub fn delete(&mut self, search: &K) -> Option<V> {
        self.work(NoopFilter, |tree| tree.delete(search))
    }

    /// Deletes all items inside the half-open `range` from the tree and stores them in the output
    /// Vec. The items are returned in order.
    #[inline]
    pub fn delete_range(&mut self, range: Range<K>, output: &mut Vec<(K, V)>) {
        output.reserve(self.size());
        self.filter_with_driver(&mut RangeDriver::new(range, output), NoopFilter)
    }

    /// Deletes all items inside the half-open `range` from the tree and stores them in the output Vec.
    #[inline]
    pub fn delete_range_ref(&mut self, range: Range<&K>, output: &mut Vec<(K, V)>) {
        output.reserve(self.size());
        self.filter_with_driver(&mut RangeRefDriver::new(range, output), NoopFilter)
    }

    #[inline]
    pub fn filter_with_driver<D, Flt>(&mut self, driver: &mut D, filter: Flt)
        where D: TraversalDriver<K, V>, Flt: ItemFilter<K>
    {
        self.work(filter, |worker: &mut PlWorker<K,V,Flt>| worker.filter_with_driver(driver))
    }

    #[inline]
    fn work<Flt, F, R>(&mut self, filter: Flt, mut f: F) -> R where Flt: ItemFilter<K>,
                                                                    F: FnMut(&mut PlWorker<K,V,Flt>) -> R
    {
        let repr: TreeRepr<PlNode<K, V>> = unsafe {
            ptr::read(self.repr.get())
        };

        let mut worker = PlWorker::new(repr, filter);
        let result = f(&mut worker);

        unsafe {
            let x = mem::replace(&mut *self.repr.get(), worker.repr);
            mem::forget(x);
        }

        result
    }


    fn repr(&self) -> &TreeRepr<PlNode<K, V>> {
        unsafe { &*self.repr.get() }
    }

    fn repr_mut(&mut self) -> &mut TreeRepr<PlNode<K, V>> {
        unsafe { &mut *self.repr.get() }
    }
}



impl<K: Key+Clone+Debug, V> Debug for PlTree<K, V> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.repr, fmt)
    }
}

impl<K: Key+Clone+Debug, V> Display for PlTree<K, V> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Display::fmt(self.deref(), fmt)
    }
}

impl<K: Key, V: Clone> Clone for PlTree<K, V> {
    fn clone(&self) -> Self {
        PlTree { repr: UnsafeCell::new(self.repr().clone()) }
    }
}



pub struct NoUpdate<K: Key, Flt: ItemFilter<K>> {
    _ph: PhantomData<(K, Flt)>
}

impl<K: Key, V, Flt: ItemFilter<K>> ItemVisitor<PlNode<K, V>> for NoUpdate<K, Flt> {
    type Tree = PlWorker<K,V, Flt>;

    #[inline(always)]
    fn visit<F>(tree: &mut Self::Tree, idx: usize, mut f: F)
                                                where F: FnMut(&mut Self::Tree, usize) {
        f(tree, idx)
    }
}



impl<K: Key, V> Deref for PlTree<K, V> {
    type Target = TreeRepr<PlNode<K, V>>;

    fn deref(&self) -> &Self::Target {
        self.repr()
    }
}

impl<K: Key, V> DerefMut for PlTree<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.repr_mut()
    }
}




pub struct PlWorker<K: Key, V, Flt> where Flt: ItemFilter<K> {
    repr: TreeRepr<PlNode<K, V>>,
    filter: Flt
}

impl<K: Key, V, Flt> PlWorker<K, V, Flt> where Flt: ItemFilter<K> {
    /// Constructs a new FilterTree
    #[inline]
    pub fn new(repr: TreeRepr<PlNode<K, V>>, filter: Flt) -> Self {
        PlWorker { repr:repr, filter:filter }
    }


    /// Deletes the item with the given key from the tree and returns it (or None).
    #[inline]
    pub fn delete(&mut self, search: &K) -> Option<V> {
        self.index_of(search).map(|idx| {
            self.delete_idx(idx)
        })
    }

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



    /// Delete based on driver decisions.
    /// The items are returned in order.
    #[inline]
    fn filter_with_driver<D: TraversalDriver<K, V>>(&mut self, drv: &mut D) {
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





impl<K: Key, V, Flt: ItemFilter<K>> Deref for PlWorker<K, V, Flt> {
    type Target = TreeRepr<PlNode<K, V>>;

    fn deref(&self) -> &Self::Target {
        &self.repr
    }
}

impl<K: Key, V, Flt: ItemFilter<K>> DerefMut for PlWorker<K, V, Flt> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.repr
    }
}

impl<K: Key, V, Flt: ItemFilter<K>> BulkDeleteCommon<PlNode<K, V>> for PlWorker<K, V, Flt> {
    type Visitor = NoUpdate<K, Flt>;
    type Filter = Flt;

    fn filter_mut(&mut self) -> &mut Self::Filter {
        &mut self.filter
    }
}


impl<K: Key, V, Flt: ItemFilter<K>> TeardownTreeRefill for PlWorker<K, V, Flt> where K: Copy, V: Copy {
    #[inline] fn refill(&mut self, master: &PlWorker<K, V, Flt>) {
        self.repr.refill(&master.repr);
    }
}
