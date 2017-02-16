use applied::AppliedTree;
use base::{Key, Node, TreeRepr, Traverse, Sink, BulkDeleteCommon, ItemVisitor, Entry, righti, lefti};
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
    pub entry: Entry<K, V>,
}


impl<K: Key, V> AppliedTree<PlNode<K, V>> for PlTree<K, V> {
    fn with_repr(repr: TreeRepr<PlNode<K, V>>) -> Self {
        PlTree { repr: UnsafeCell::new(repr) }
    }

    unsafe fn with_shape(items: Vec<Option<(K, V)>>) -> Self {
        let nodes = items.into_iter()
            .map(|opt| opt.map(|(k, v)| PlNode::new(k.clone(), v)))
            .collect::<Vec<_>>();
        Self::with_repr(TreeRepr::with_nodes(nodes))
    }
}




impl<K: Key, V> Deref for PlNode<K, V> {
    type Target = Entry<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl<K: Key, V> DerefMut for PlNode<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry
    }
}

impl<K: Key, V> Node for PlNode<K, V> {
    type K = K;
    type V = V;

    #[inline] fn new(key: K, val: V) -> Self {
        PlNode { entry: Entry::new(key, val) }
    }

    #[inline] fn into_entry(self) -> Entry<K, V> {
        self.entry
    }

    #[inline] fn into_tuple(self) -> (K, V) {
        self.entry.into_tuple()
    }
}

impl<K: Key+fmt::Debug, V> fmt::Debug for PlNode<K, V> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.entry.key(), fmt)
    }
}


//---- constructors and helpers --------------------------------------------------------------------
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


    fn repr(&self) -> &TreeRepr<PlNode<K, V>> {
        // This is safe according to UnsafeCell::get(), because there are no mutable aliases to
        // self.repr possible at the time when &self is taken.
        unsafe { &*self.repr.get() }
    }

    fn repr_mut(&mut self) -> &mut TreeRepr<PlNode<K, V>> {
        // This is safe according to UnsafeCell::get(), because the access to self.repr is unique at
        // the time when &mut self is taken.
        unsafe { &mut *self.repr.get() }
    }

    pub fn into_repr(self) -> TreeRepr<PlNode<K, V>> {
        // This is safe according to UnsafeCell::into_inner(), because no thread can be inspecting
        // the inner value when self is passed by value.
        unsafe { self.repr.into_inner() }
    }
}



//---- single-item queries -------------------------------------------------------------------------
impl<K: Key, V> PlTree<K, V> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    #[inline]
    pub fn delete<Q: PartialOrd<K>>(&mut self, query: &Q) -> Option<V> {
        let idx = self.index_of(query);
        if self.is_nil(idx) {
            None
        } else {
            Some(self.delete_idx(idx))
        }
    }

    // The caller must ensure that `!is_nil(idx)`.
    #[inline]
    fn delete_idx(&mut self, idx: usize) -> V {
        debug_assert!(!self.is_nil(idx));

        let node = self.take(idx);
        // All 3 invariants of delete_max/min are satisfied.
        if self.has_left(idx) {
            self.delete_max(idx, lefti(idx));
        } else if self.has_right(idx) {
            self.delete_min(idx, righti(idx));
        }
        node.entry.into_tuple().1
    }


    // The caller must ensure that the following invariants are satisfied:
    //   a) both idx and hole point to valid indices into data
    //   b) the cell at `idx` is non-empty
    //   c) the cell at `hole` is empty
    #[inline]
    fn delete_max(&mut self, mut hole: usize, mut idx: usize) {
        // We maintain all three invariants (a), (b) and (c) for each iteration of the loop.
        loop {
            debug_assert!(self.is_nil(hole) && !self.is_nil(idx) && idx == lefti(hole));

            idx = self.find_max(idx);
            // This is safe because the invariant of `move_from_to()` is exactly (a), (b) and (c).
            unsafe { self.move_from_to(idx, hole); }
            hole = idx;

            idx = lefti(idx);
            if self.is_nil(idx) {
                break;
            }
        }
    }

    // The caller must ensure that the following invariants are satisfied:
    //   a) both idx and hole point to valid indices into data
    //   b) the cell at `idx` is non-empty
    //   c) the cell at `hole` is empty
    #[inline]
    fn delete_min(&mut self, mut hole: usize, mut idx: usize) {
        // We maintain all three invariants (a), (b) and (c) for each iteration of the loop.
        loop {
            debug_assert!(self.is_nil(hole) && !self.is_nil(idx) && idx == righti(hole));

            idx = self.find_min(idx);
            // This is safe because the invariant of `move_from_to()` is exactly (a), (b) and (c).
            unsafe { self.move_from_to(idx, hole); }
            hole = idx;

            idx = righti(idx);
            if self.is_nil(idx) {
                break;
            }
        }
    }
}


//---- range queries -------------------------------------------------------------------------------
impl<K: Key, V> PlTree<K, V> {
    /// Deletes all items inside `range` from the tree and feeds them into `sink`.
    /// The items are returned in order.
    #[inline]
    pub fn delete_range<Q, S>(&mut self, range: Range<Q>, sink: S)
        where Q: PartialOrd<K>, S: Sink<(K, V)>
    {
        self.filter_with_driver(RangeDriver::new(range, sink), NoopFilter)
    }

    /// Deletes all items inside `range` that match `filter` from the tree and feeds them into
    /// `sink`. The items are returned in order.
    pub fn filter_range<Q: PartialOrd<K>, Flt, S>(&mut self, range: Range<Q>, filter: Flt, sink: S)
        where Flt: ItemFilter<K>, S: Sink<(K, V)>
    {
        self.filter_with_driver(RangeDriver::new(range, sink), filter)
    }

    /// Deletes all items inside `range` from the tree and feeds them into `sink`. The items are
    /// returned in order.
    #[inline]
    pub fn delete_range_ref<Q, S>(&mut self, range: Range<&Q>, sink: S)
        where Q: PartialOrd<K>, S: Sink<(K, V)>
    {
        self.filter_with_driver(RangeRefDriver::new(range, sink), NoopFilter)
    }

    /// Deletes all items inside `range` that match `filter` from the tree and feeds them into
    /// `sink`. The items are returned in order.
    pub fn filter_range_ref<Q, Flt, S>(&mut self, range: Range<&Q>, filter: Flt, sink: S)
        where Q: PartialOrd<K>, Flt: ItemFilter<K>, S: Sink<(K, V)>
    {
        self.filter_with_driver(RangeRefDriver::new(range, sink), filter)
    }

    /// Deletes items based on driver decisions and filter. The items are returned in order.
    #[inline]
    pub fn filter_with_driver<D, Flt>(&mut self, driver: D, filter: Flt)
        where D: TraversalDriver<K, V>, Flt: ItemFilter<K>
    {
        self.work(driver, filter, |worker: &mut PlWorker<K,V,D,Flt>| worker.filter())
    }

    pub fn query_range<'a, Q, S>(&'a self, query: Range<Q>, mut sink: S)
        where Q: PartialOrd<K>, S: Sink<&'a (K, V)>
    {
        let mut from = self.index_of(&query.start);
        if self.is_nil(from) {
            from = self.succ(from);
            if self.is_nil(from) {
                return;
            }
        }

        TreeRepr::traverse_inorder_from(self, from, 0, &mut sink, |this, sink, idx| {
            let node = this.node(idx);
            if &query.end <= node.key() && &query.start != node.key() {
                true
            } else {
                sink.consume(node.as_tuple());
                false
            }
        })
    }


    #[inline]
    fn work<D, Flt, F, R>(&mut self, driver: D, filter: Flt, mut f: F) -> R
        where D: TraversalDriver<K, V>,
              Flt: ItemFilter<K>,
              F: FnMut(&mut PlWorker<K,V,D,Flt>) -> R
    {
        // TODO: this can be sped up in several ways, e.g. having TreeRepr::filter of &Flt type, then we don't have to copy repr
        let repr: TreeRepr<PlNode<K, V>> = unsafe {
            ptr::read(self.repr.get())
        };

        let mut worker = PlWorker::new(repr, driver, filter);
        let result = f(&mut worker);

        // We do not reallocate the vecs inside repr, and the only thing that changes in its memory
        // is the size of the tree. So we can get away with only updating the size as opposed to
        // doing another expensive copy of the whole TreeRepr struct.
        //
        // This optimization results in a measurable speed-up to tiny/small range queries.
        self.repr_mut().size = worker.repr.size;
        mem::forget(worker.repr);

        result
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



pub struct NoUpdate<K, D, Flt> {
    _ph: PhantomData<(K, D, Flt)>
}

impl<K, V, D, Flt> ItemVisitor<PlNode<K, V>> for NoUpdate<K, D, Flt>
    where K: Key, D: TraversalDriver<K, V>, Flt: ItemFilter<K>
{
    type Tree = PlWorker<K,V,D,Flt>;

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



#[derive(new)]
pub struct PlWorker<K, V, D, Flt>
    where K: Key
{
    repr: TreeRepr<PlNode<K, V>>,
    drv: D,
    filter: Flt
}

impl<K, V, D, Flt> PlWorker<K, V, D, Flt>
    where K: Key, D: TraversalDriver<K, V>, Flt: ItemFilter<K>
{
    #[inline]
    fn filter(&mut self) {
        self.delete_range_loop(0);
        debug_assert!(self.slots_min().is_empty(), "slots_min={:?}", self.slots_min());
        debug_assert!(self.slots_max().is_empty());
    }

    #[inline]
    fn delete_range_loop(&mut self, mut idx: usize) {
        loop {
            if self.is_nil(idx) {
                return;
            }

            let decision = self.drv.decide(self.key(idx));

            if decision.left() && decision.right() {
                let item = self.filter_take(idx);
                let mut removed = item.is_some();

                removed = self.descend_delete_max_left(idx, removed);
                if let Some(item) = item {
                    self.drv.consume(item.into_tuple());
                }
                self.descend_delete_min_right(idx, removed);
                return;
            } else if decision.left() {
                idx = lefti(idx);
            } else {
                debug_assert!(decision.right());
                idx = righti(idx);
            }
        }
    }

    // The caller must make sure that `!is_nil(idx)`.
    #[inline(never)]
    fn delete_range_min(&mut self, idx: usize) {
        let decision = self.drv.decide(self.key(idx));
        debug_assert!(decision.left());

        if decision.right() {
            // the root and the whole left subtree are inside the range
            let item = self.filter_take(idx);
            let mut removed = item.is_some();
            removed = self.descend_consume_left(idx, removed);
            if let Some(item) = item {
                self.drv.consume(item.into_tuple())
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

            self.descend_delete_min_right(idx, removed);
        } else {
            // the root and the right subtree are outside the range
            self.descend_delete_min_left(idx, false);

            if self.slots_min().has_open() {
                self.fill_slot_min(idx);
                self.descend_fill_min_right(idx, true);
            }
        }
    }

    // The caller must make sure that `!is_nil(idx)`.
    #[inline(never)]
    fn delete_range_max(&mut self, idx: usize) {
        let decision = self.drv.decide(self.key(idx));
        debug_assert!(decision.right(), "idx={}", idx);

        if decision.left() {
            // the root and the whole right subtree are inside the range
            let item = self.filter_take(idx);
            let mut removed = self.descend_delete_max_left(idx, item.is_some());
            if let Some(item) = item {
                self.drv.consume(item.into_tuple())
            }
            removed = self.descend_consume_right(idx, removed);

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
            self.descend_delete_max_right(idx, false);

            if self.slots_max().has_open() {
                self.fill_slot_max(idx);
                self.descend_fill_max_left(idx, true);
            }
        }
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_min_left(&mut self, idx: usize, with_slot: bool) -> bool {
        self.descend_left(idx, with_slot,
                          |this: &mut Self, child_idx| this.delete_range_min(child_idx))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_max_left(&mut self, idx: usize, with_slot: bool) -> bool {
        if Flt::is_noop() {
            self.descend_left(idx, with_slot,
                              |this: &mut Self, child_idx| this.delete_range_max(child_idx))
        } else {
            self.descend_left_fresh_slots(idx, with_slot,
                                          |this: &mut Self, child_idx| this.delete_range_max(child_idx))
        }
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_min_right(&mut self, idx: usize, with_slot: bool) -> bool
        where D: TraversalDriver<K, V>
    {
        self.descend_right(idx, with_slot,
                           |this: &mut Self, child_idx| this.delete_range_min(child_idx))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_max_right(&mut self, idx: usize, with_slot: bool) -> bool
        where D: TraversalDriver<K, V>
    {
        self.descend_right(idx, with_slot,
                           |this: &mut Self, child_idx| this.delete_range_max(child_idx))
    }
}



impl<K, V, D, Flt> Deref for PlWorker<K, V, D, Flt>
    where K: Key, D: TraversalDriver<K, V>, Flt: ItemFilter<K>
{
    type Target = TreeRepr<PlNode<K, V>>;

    fn deref(&self) -> &Self::Target {
        &self.repr
    }
}

impl<K, V, D, Flt> DerefMut for PlWorker<K, V, D, Flt>
    where K: Key, D: TraversalDriver<K, V>, Flt: ItemFilter<K>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.repr
    }
}

impl<K, V, D, Flt> BulkDeleteCommon<PlNode<K, V>> for PlWorker<K, V, D, Flt>
    where K: Key, D: TraversalDriver<K, V>, Flt: ItemFilter<K>
{
    type Visitor = NoUpdate<K, D, Flt>;
    type Sink = D;
    type Filter = Flt;

    fn filter_mut(&mut self) -> &mut Self::Filter {
        &mut self.filter
    }

    fn sink_mut(&mut self) -> &mut Self::Sink {
        &mut self.drv
    }
}
