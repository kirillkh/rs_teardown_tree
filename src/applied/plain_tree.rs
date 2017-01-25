use base::{Key, Node, TreeRepr, TeardownTreeRefill, BulkDeleteCommon, ItemVisitor, KeyVal, righti, lefti, consume_unchecked};
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

    #[inline] fn new(key: K, val: V) -> Self {
        PlNode { kv: KeyVal::new(key, val) }
    }

    #[inline] fn into_kv(self) -> KeyVal<K, V> {
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

    /// Deletes all items inside the half-open `range` from the tree for which filter.accept() returns
    /// true and stores them in the output Vec. The items are returned in order.
    pub fn filter_range<Flt>(&mut self, range: Range<K>, filter: Flt, output: &mut Vec<(K, V)>)
        where Flt: ItemFilter<K>
    {
        output.reserve(self.size());
        self.filter_with_driver(&mut RangeDriver::new(range, output), filter)
    }

    /// Deletes all items inside the half-open `range` from the tree and stores them in the output Vec.
    #[inline]
    pub fn delete_range_ref(&mut self, range: Range<&K>, output: &mut Vec<(K, V)>) {
        output.reserve(self.size());
        self.filter_with_driver(&mut RangeRefDriver::new(range, output), NoopFilter)
    }

    /// Deletes all items inside the half-open `range` from the tree for which filter.accept() returns
    /// true and stores them in the output Vec. The items are returned in order.
    pub fn filter_range_ref<Flt>(&mut self, range: Range<&K>, filter: Flt, output: &mut Vec<(K, V)>)
        where Flt: ItemFilter<K>
    {
        output.reserve(self.size());
        self.filter_with_driver(&mut RangeRefDriver::new(range, output), filter)
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
        // TODO: this can be sped up in several ways, e.g. having TreeRepr::filter of &Flt type, then we don't have to copy repr
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
        Debug::fmt(self.repr(), fmt)
    }
}

impl<K: Key+Clone+Debug, V> Display for PlTree<K, V> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Display::fmt(self.repr(), fmt)
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

            let key = self.key_unsafe(idx);
            let decision = drv.decide(key);

            if decision.left() && decision.right() {
                let item = self.filter_take(idx);
                let mut removed = item.is_some();

                removed = self.descend_delete_max_left(drv, idx, removed);
                if let Some(item) = item {
                    consume_unchecked(drv.output(), item.into_kv());
                }
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
        let key = self.key_unsafe(idx);
        let decision = drv.decide(key);
        debug_assert!(decision.left());

        if decision.right() {
            // the root and the whole left subtree are inside the range
            let item = self.filter_take(idx);
            let mut removed = item.is_some();
            removed = self.descend_consume_left(idx, removed, drv.output());
            if let Some(item) = item {
                consume_unchecked(drv.output(), item.into_kv());
            }

            if !Flt::is_noop() {
                if removed {
                    removed = self.descend_fill_max_left(idx, true);
                }
                if !removed && self.slots_min().has_open() {
                    self.descend_fill_min_left(idx, false);
                    debug_assert!(self.slots_min().has_open());
                    self.fill_slot_min(idx);
                    removed = true;
                }
            }

            self.descend_delete_min_right(drv, idx, removed);
        } else {
            // the root and the right subtree are outside the range
            self.descend_delete_min_left(drv, idx, false);

            if self.slots_min().has_open() {
                self.fill_slot_min(idx);
                self.descend_fill_min_right(idx, true);
            }
        }
    }

    #[inline(never)]
    fn delete_range_max<D: TraversalDriver<K, V>>(&mut self, drv: &mut D, idx: usize) {
        let key = self.key_unsafe(idx);
        let decision = drv.decide(key);
        debug_assert!(decision.right(), "idx={}", idx);

        if decision.left() {
            // the root and the whole right subtree are inside the range
            let item = self.filter_take(idx);
            let mut removed = self.descend_delete_max_left(drv, idx, item.is_some());
            if let Some(item) = item {
                consume_unchecked(drv.output(), item.into_kv());
            }
            removed = self.descend_consume_right(idx, removed, drv.output());

            if !Flt::is_noop() {
                if !removed && self.slots_max().has_open() {
                    self.fill_slot_max(idx);
                    removed = true
                }
                if removed {
                    self.descend_fill_max_left(idx, true);
                }
            }
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
        if Flt::is_noop() {
            self.descend_left(idx, with_slot,
                              |this: &mut Self, child_idx| this.delete_range_max(drv, child_idx))
        } else {
            self.descend_left_fresh_slots(idx, with_slot,
                                          |this: &mut Self, child_idx| this.delete_range_max(drv, child_idx))
        }
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
