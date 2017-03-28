use applied::AppliedTree;
use base::{Key, Node, TreeRepr, Traverse, Sink, BulkDeleteCommon, ItemVisitor, Entry, righti, lefti, parenti, depth_of};
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

type Nd<K,V> = PlNode<K, V>;


impl<K: Key, V> AppliedTree<Nd<K, V>> for PlTree<K, V> {
    fn with_repr(repr: TreeRepr<Nd<K, V>>) -> Self {
        PlTree { repr: UnsafeCell::new(repr) }
    }

    unsafe fn with_shape(items: Vec<Option<(K, V)>>) -> Self {
        let nodes = items.into_iter()
            .map(|opt| opt.map(|(k, v)| PlNode::new(k.clone(), v)))
            .collect::<Vec<_>>();
        Self::with_repr(TreeRepr::with_nodes(nodes))
    }
}




impl<K: Key, V> Deref for Nd<K, V> {
    type Target = Entry<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl<K: Key, V> DerefMut for Nd<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry
    }
}

impl<K: Key, V> Node for Nd<K, V> {
    type K = K;
    type V = V;

    #[inline] fn new(key: K, val: V) -> Self {
        PlNode { entry: Entry::new(key, val) }
    }

    #[inline] fn from_tuple(t: (K,V)) -> Self {
        PlNode { entry: Entry::from_tuple(t) }
    }

    #[inline] fn into_entry(self) -> Entry<K, V> {
        self.entry
    }

    #[inline] fn into_tuple(self) -> (K, V) {
        self.entry.into_tuple()
    }
}

impl<K: Key+fmt::Debug, V> fmt::Debug for Nd<K, V> {
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

    pub fn with_repr(repr: TreeRepr<Nd<K, V>>) -> PlTree<K, V> {
        PlTree { repr: UnsafeCell::new(repr) }
    }

    /// Constructs a new PlTree
    /// Note: the argument must be sorted!
    pub fn with_sorted(sorted: Vec<(K, V)>) -> PlTree<K, V> {
        PlTree::with_repr(TreeRepr::with_sorted(sorted))
    }

    pub fn with_nodes(nodes: Vec<Option<Nd<K, V>>>) -> PlTree<K, V> {
        PlTree::with_repr(TreeRepr::with_nodes(nodes))
    }


    pub fn repr(&self) -> &TreeRepr<Nd<K, V>> {
        // This is safe according to UnsafeCell::get(), because there are no mutable aliases to
        // self.repr possible at the time when &self is taken.
        unsafe { &*self.repr.get() }
    }

    pub fn repr_mut(&mut self) -> &mut TreeRepr<Nd<K, V>> {
        // This is safe according to UnsafeCell::get(), because the access to self.repr is unique at
        // the time when &mut self is taken.
        unsafe { &mut *self.repr.get() }
    }

    pub fn into_repr(self) -> TreeRepr<Nd<K, V>> {
        // This is safe according to UnsafeCell::into_inner(), because no thread can be inspecting
        // the inner value when self is passed by value.
        unsafe { self.repr.into_inner() }
    }
}


//---- single-item insert --------------------------------------------------------------------------
impl<K: Key, V> PlTree<K, V> {
    #[inline]
    pub fn insert(&mut self, item: (K, V)) -> Option<V> {
        let target_idx = self.index_of(&item.0);

        if target_idx >= self.capacity() {
            self.rebalance(target_idx, item);
            None
        } else {
            let old = self.place(target_idx, PlNode::from_tuple(item));
            
            old.map(|node| node.entry.into_tuple().1)
        }
    }

    
    fn rebalance(&mut self, idx: usize, item: (K, V)) {
        if 2*(self.size()+1) <= self.capacity() {
            // partial rebuild
            self.partial_rebuild(idx, item);
        } else {
            // full rebuild required
//            println!("double: size={}, capacity={}", self.size(), self.capacity());
            self.double_and_rebuild(item);
        }
    }

    fn partial_rebuild(&mut self, mut idx: usize, item: (K, V)) {
        let h = self.complete_height();
//        debug_assert!(h == depth_of(idx));
        let mut d = depth_of(idx);
        let full_rebuild_depth = h - depth_of(h);
        
        let mut count = 1; // 1 stands for the `item` that we pretend is already inserted
        let mut complete_count = 0;
        let mut insert_offs = 0;
        loop {
            debug_assert!(idx > 0);
            let sibling_count;
            let sibling;
            if idx & 1 == 0 {
                sibling = idx - 1;
                sibling_count = self.count_nodes(sibling);
                insert_offs += sibling_count + 1;
            } else {
                sibling = idx + 1;
                sibling_count = self.count_nodes(sibling);
            }

            idx = parenti(idx);
            count += 1 + sibling_count;
            complete_count = 2*complete_count + 1;
    
            d -= 1;
    
            if full_rebuild_depth <= d+1 {
                // This was supposed to bring the insert complexity down to O(log(n))??? Not sure where I
                // saw the idea, though! Experimentally, it does make sequential inserts a little faster.
                if count <= complete_count {
                    self.rebuild_subtree2(idx, insert_offs, count, item);
                    break;
                }
            } else {
                // TODO: we avoid floating-point division for numeric stability and speed, but it will cause overflow for extremely big trees
                if count * 2 * (h-1) <= complete_count * (h-1+d) {
                    self.rebuild_subtree2(idx, insert_offs, count, item);
                    break;
                }
            }
        }
    }

//    fn redistribute(&mut self, root: usize, insert_offs: usize, count: usize, item: (K, V)) {
//        let mut dst_offs = 0;
//        // TODO: figure out how much to allocate
//        let mut fifo: VecDeque<Nd<K,V>> = VecDeque::new();
//        let mut item_idx = 0;
//
//
//        let push = |this: &mut TreeRepr<_>, dst_offs: &mut usize, fifo: &mut VecDeque<_>, node| {
//            let local_idx = inorder_to_idx_n(*dst_offs, complete_count);
//
////            let local_idx = {
////                let k = complete_count + 1 + *dst_offs;
////                let shift = (!k).trailing_zeros();
////                (k >> (shift+1)) - 1
////            };
//            let global_idx = local_idx + root * (1 << depth_of(local_idx));
//
//
////            println!("place root={}, dst_offs={}, N={}, local={}, global={}", root, *dst_offs, complete_count, local_idx, global_idx);
//
//            let old = this.place(global_idx, node);
//            if let Some(node) = old {
//                fifo.push_back(node);
//            }
//            *dst_offs += 1;
//        };
//
//        let step = |this: &mut TreeRepr<_>, dst_offs: &mut usize, fifo: &mut VecDeque<Nd<K,V>>, idx| {
//            // push all relevant items from the fifo
//            while !fifo.is_empty() && fifo[0].key() <= this.key(idx) {
//                let node = fifo.pop_front().unwrap();
//                push(this, dst_offs, fifo, node);
//            }
//
//            // push the source item
//            let node = this.take(idx);
//            push(this, dst_offs, fifo, node);
//            false
//        };
//
//
//        TreeRepr::traverse_inorder_mut(self, root, &mut dst_offs, |this, dst_offs, idx| {
//            if *dst_offs == insert_offs {
//                item_idx = idx;
//                true
//            } else {
//                step(this, dst_offs, &mut fifo, idx)
//            }
//        });
//
//        fifo.push_back(PlNode::from_tuple(item));
//
//        TreeRepr::traverse_inorder_from_mut(self, item_idx, root, &mut dst_offs, |this, dst_offs, idx| {
//            step(this, dst_offs, &mut fifo, idx)
//        });
//
//        while let Some(node) = fifo.pop_front() {
//            push(self, &mut dst_offs, &mut fifo, node);
//        }
//    }
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
        // All 3 precondition of delete_max/min are satisfied.
        if self.has_left(idx) {
            self.delete_max(idx, lefti(idx));
        } else if self.has_right(idx) {
            self.delete_min(idx, righti(idx));
        }
        node.entry.into_tuple().1
    }


    // The caller must ensure that the following preconditions are satisfied:
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

    // The caller must ensure that the following preconditions are satisfied:
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
            if let Some(f) = self.successor(from) {
                from = f;
            } else {
                return;
            }
        }

        TreeRepr::traverse_inorder_from(self, from, 0, &mut sink, (), |this, sink, idx| {
            let node = this.node(idx);
            if &query.end <= node.key() && &query.start != node.key() {
                Some(())
            } else {
                sink.consume(node.as_tuple());
                None
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
        let repr: TreeRepr<Nd<K, V>> = unsafe {
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

impl<K, V, D, Flt> ItemVisitor<Nd<K, V>> for NoUpdate<K, D, Flt>
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
    type Target = TreeRepr<Nd<K, V>>;

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
    repr: TreeRepr<Nd<K, V>>,
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
    type Target = TreeRepr<Nd<K, V>>;

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

impl<K, V, D, Flt> BulkDeleteCommon<Nd<K, V>> for PlWorker<K, V, D, Flt>
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



#[cfg(test)]
pub mod tests {
    use rand::{Rng, XorShiftRng};

    use super::*;
    use util::make_permutation;
    use base::validation::to_vec;

    type Tree = PlTree<usize, usize>;

    pub fn ins(tree: &mut Tree, x: usize) {
//        println!("tree={}", tree);
        let old = tree.insert((x, x));
        assert_eq!(None, old);
    }

    fn repl(tree: &mut Tree, x: usize) {
        let old = tree.insert((x, x));
        assert_eq!(Some(x), old);
    }


    #[test]
    pub fn insert_iter() {
        let n = 1000;

        let tree = &mut PlTree::new(vec![]);
        for i in (0..n).rev() {
//            println!("i={}, tree={}", i, &tree);
            ins(tree, i);
            assert_eq!(to_vec(tree), (i..n).collect::<Vec<_>>());
        }

        let tree = &mut PlTree::new(vec![]);
        for i in 0..n {
//            println!("i={}, tree={}", i, &tree);
            ins(tree, i);
            assert_eq!(tree.size(), i+1);
            assert_eq!(to_vec(tree), (0..i+1).collect::<Vec<_>>());
        }

        let tree = &mut PlTree::new(vec![]);
        for i in (0..n).rev() {
            ins(tree, i);
            assert_eq!(to_vec(tree), (i..n).collect::<Vec<_>>());
        }
    }

    #[test]
    pub fn insert_rng() {
        let n = 30;
        let mut rng = XorShiftRng::new_unseeded();

        for j in 0..1000 {
            let seq = make_permutation(n, &mut rng);
            let tree = &mut PlTree::new(vec![]);
            for i in 0..n {
//            println!("i={}, seq[i]={}, tree={}", i, seq[i], &tree);
                ins(tree, seq[i]);
                assert_eq!(tree.size(), i + 1);
            }
            assert_eq!(to_vec(tree), (0..n).collect::<Vec<_>>());
        }
    }

    #[test]
    pub fn insert_predef() {
        let mut tree = &mut PlTree::new(vec![]);
        ins(tree, 9);
        ins(tree, 0);
        ins(tree, 8);
        ins(tree, 1);
        ins(tree, 2);
        ins(tree, 6);
        ins(tree, 7);
        ins(tree, 5);
        ins(tree, 3);
        ins(tree, 4);
        assert_eq!(tree.size(), 10);
        assert_eq!(to_vec(tree), (0..10).collect::<Vec<_>>());
        
    
        let items: Vec<_> = (0..255).map(|x| (x, x)).collect();

        let mut tree = &mut PlTree::new(items.clone());
        tree.delete_range(0..items.len(), vec![]);
        ins(tree, 9);
        ins(tree, 0);
        ins(tree, 8);
        ins(tree, 1);
        ins(tree, 2);
        ins(tree, 6);
        ins(tree, 7);
        ins(tree, 5);
        ins(tree, 3);
        ins(tree, 4);
        assert_eq!(tree.size(), 10);
        assert_eq!(to_vec(tree), (0..10).collect::<Vec<_>>());

        let mut tree = &mut PlTree::new(items.clone());
        tree.delete_range(0..items.len(), vec![]);
        ins(tree, 0);
        ins(tree, 1);
        ins(tree, 2);
        ins(tree, 4);
        ins(tree, 3);
        assert_eq!(tree.size(), 5);
        assert_eq!(to_vec(tree), (0..5).collect::<Vec<_>>());

        let mut tree = &mut PlTree::new(items.clone());
        tree.delete_range(0..items.len(), vec![]);
        ins(tree, 4);
        ins(tree, 3);
        ins(tree, 2);
        ins(tree, 0);
        ins(tree, 1);
        assert_eq!(tree.size(), 5);
        assert_eq!(to_vec(tree), (0..5).collect::<Vec<_>>());
    }
}


#[cfg(all(feature = "unstable", test))]
mod bench {
    extern crate test;

    use super::*;
    use super::tests::ins;
    use util::make_permutation;

    use self::test::Bencher;

    use rand::{Rng, SeedableRng, XorShiftRng, thread_rng};


    pub fn bench_inc(bencher: &mut Bencher, n: usize) {
        bencher.iter(|| {
            let tree = &mut PlTree::new(vec![]);
            for i in 0..n {
                ins(tree, i);
            }
            assert_eq!(tree.size(), n);
        });
    }

    pub fn bench_dec(bencher: &mut Bencher, n: usize) {
        bencher.iter(|| {
            let tree = &mut PlTree::new(vec![]);
            for i in (0..n).rev() {
                ins(tree, i);
            }
            assert_eq!(tree.size(), n);
        });
    }

    pub fn bench_rng(bencher: &mut Bencher, n: usize) {
        let mut trng = thread_rng();
        let seed = [trng.gen(), trng.gen(), trng.gen(), trng.gen()];
        let mut rng = XorShiftRng::from_seed(seed);

        let seq = make_permutation(n, &mut rng);
        bencher.iter(|| {
            let tree = &mut PlTree::new(vec![]);
            for i in 0..n {
                ins(tree, seq[i]);
            }
            assert_eq!(tree.size(), n);
        });
    }



    #[bench]
    pub fn bench_insert_inc_01_000(bencher: &mut Bencher) {
        bench_inc(bencher, 1_000);
    }

    #[bench]
    pub fn bench_insert_inc_02_000(bencher: &mut Bencher) {
        bench_inc(bencher, 2_000);
    }

    #[bench]
    pub fn bench_insert_inc_03_000(bencher: &mut Bencher) {
        bench_inc(bencher, 3_000);
    }

    #[bench]
    pub fn bench_insert_inc_04_000(bencher: &mut Bencher) {
        bench_inc(bencher, 4_000);
    }

    #[bench]
    pub fn bench_insert_inc_05_000(bencher: &mut Bencher) {
        bench_inc(bencher, 5_000);
    }

    #[bench]
    pub fn bench_insert_inc_06_000(bencher: &mut Bencher) {
        bench_inc(bencher, 6_000);
    }

    #[bench]
    pub fn bench_insert_inc_07_000(bencher: &mut Bencher) {
        bench_inc(bencher, 7_000);
    }

    #[bench]
    pub fn bench_insert_inc_08_000(bencher: &mut Bencher) {
        bench_inc(bencher, 8_000);
    }

    #[bench]
    pub fn bench_insert_inc_09_000(bencher: &mut Bencher) {
        bench_inc(bencher, 9_000);
    }


    #[bench]
    pub fn bench_insert_inc_10_000(bencher: &mut Bencher) {
        bench_inc(bencher, 10_000);
    }

    #[bench]
    pub fn bench_insert_inc_20_000(bencher: &mut Bencher) {
        bench_inc(bencher, 20_000);
    }

    #[bench]
    pub fn bench_insert_inc_30_000(bencher: &mut Bencher) {
        bench_inc(bencher, 30_000);
    }

    #[bench]
    pub fn bench_insert_inc_40_000(bencher: &mut Bencher) {
        bench_inc(bencher, 40_000);
    }

    #[bench]
    pub fn bench_insert_inc_50_000(bencher: &mut Bencher) {
        bench_inc(bencher, 50_000);
    }




    #[bench]
    pub fn bench_insert_dec_01_000(bencher: &mut Bencher) {
        bench_dec(bencher, 1_000);
    }

    #[bench]
    pub fn bench_insert_dec_02_000(bencher: &mut Bencher) {
        bench_dec(bencher, 2_000);
    }

    #[bench]
    pub fn bench_insert_dec_03_000(bencher: &mut Bencher) {
        bench_dec(bencher, 3_000);
    }

    #[bench]
    pub fn bench_insert_dec_04_000(bencher: &mut Bencher) {
        bench_dec(bencher, 4_000);
    }

    #[bench]
    pub fn bench_insert_dec_05_000(bencher: &mut Bencher) {
        bench_dec(bencher, 5_000);
    }

    #[bench]
    pub fn bench_insert_dec_06_000(bencher: &mut Bencher) {
        bench_dec(bencher, 6_000);
    }

    #[bench]
    pub fn bench_insert_dec_07_000(bencher: &mut Bencher) {
        bench_dec(bencher, 7_000);
    }

    #[bench]
    pub fn bench_insert_dec_08_000(bencher: &mut Bencher) {
        bench_dec(bencher, 8_000);
    }

    #[bench]
    pub fn bench_insert_dec_09_000(bencher: &mut Bencher) {
        bench_dec(bencher, 9_000);
    }

    #[bench]
    pub fn bench_insert_dec_10_000(bencher: &mut Bencher) {
        bench_dec(bencher, 10_000);
    }




    #[bench]
    pub fn bench_insert_rng_01_000(bencher: &mut Bencher) {
        bench_rng(bencher, 1_000);
    }

    #[bench]
    pub fn bench_insert_rng_02_000(bencher: &mut Bencher) {
        bench_rng(bencher, 2_000);
    }

    #[bench]
    pub fn bench_insert_rng_03_000(bencher: &mut Bencher) {
        bench_rng(bencher, 3_000);
    }

    #[bench]
    pub fn bench_insert_rng_04_000(bencher: &mut Bencher) {
        bench_rng(bencher, 4_000);
    }

    #[bench]
    pub fn bench_insert_rng_05_000(bencher: &mut Bencher) {
        bench_rng(bencher, 5_000);
    }

    #[bench]
    pub fn bench_insert_rng_06_000(bencher: &mut Bencher) {
        bench_rng(bencher, 6_000);
    }

    #[bench]
    pub fn bench_insert_rng_07_000(bencher: &mut Bencher) {
        bench_rng(bencher, 7_000);
    }

    #[bench]
    pub fn bench_insert_rng_08_000(bencher: &mut Bencher) {
        bench_rng(bencher, 8_000);
    }

    #[bench]
    pub fn bench_insert_rng_09_000(bencher: &mut Bencher) {
        bench_rng(bencher, 9_000);
    }

    #[bench]
    pub fn bench_insert_rng_10_000(bencher: &mut Bencher) {
        bench_rng(bencher, 10_000);
    }



//    #[bench]
//    pub fn bench_insert_dec_10_000(bencher: &mut Bencher) {
//        bench_dec(bencher, 10_000);
//    }
//
//    #[bench]
//    pub fn bench_insert_dec_20_000(bencher: &mut Bencher) {
//        bench_dec(bencher, 20_000);
//    }
//
//    #[bench]
//    pub fn bench_insert_dec_30_000(bencher: &mut Bencher) {
//        bench_dec(bencher, 30_000);
//    }
//
//    #[bench]
//    pub fn bench_insert_dec_40_000(bencher: &mut Bencher) {
//        bench_dec(bencher, 40_000);
//    }
//
//    #[bench]
//    pub fn bench_insert_dec_50_000(bencher: &mut Bencher) {
//        bench_dec(bencher, 50_000);
//    }
}



#[cfg(all(feature = "unstable", test))]
mod bench_btree {
    extern crate test;

    use std::collections::BTreeMap;
    use rand::{Rng, SeedableRng, XorShiftRng, thread_rng};

//    use applied::plain_tree::{PlTree, PlNode};
    use self::test::Bencher;
    use util::make_permutation;


    pub fn ins(tree: &mut BTreeMap<usize,usize>, x: usize) {
        let old = tree.insert(x, x);
        assert_eq!(None, old);
    }


    pub fn bench_rng(bencher: &mut Bencher, n: usize) {
        let mut trng = thread_rng();
        let seed = [trng.gen(), trng.gen(), trng.gen(), trng.gen()];
        let mut rng = XorShiftRng::from_seed(seed);

        let seq = make_permutation(n, &mut rng);
        bencher.iter(|| {
            let mut tree = &mut BTreeMap::new();
            for i in 0..n {
                ins(tree, seq[i]);
            }
            assert_eq!(tree.len(), n);
        });
    }


    pub fn bench_inc(bencher: &mut Bencher, n: usize) {
        bencher.iter(|| {
            let tree = &mut BTreeMap::new();
            for i in 0..n {
                ins(tree, i);
            }
            assert_eq!(tree.len(), n);
        });
    }

    pub fn bench_dec(bencher: &mut Bencher, n: usize) {
        bencher.iter(|| {
            let tree = &mut BTreeMap::new();
            for i in (0..n).rev() {
                ins(tree, i);
            }
            assert_eq!(tree.len(), n);
        });
    }


    #[bench]
    pub fn bench_insert_rng_01_000(bencher: &mut Bencher) {
        bench_rng(bencher, 1_000);
    }

    #[bench]
    pub fn bench_insert_rng_02_000(bencher: &mut Bencher) {
        bench_rng(bencher, 2_000);
    }

    #[bench]
    pub fn bench_insert_rng_03_000(bencher: &mut Bencher) {
        bench_rng(bencher, 3_000);
    }

    #[bench]
    pub fn bench_insert_rng_04_000(bencher: &mut Bencher) {
        bench_rng(bencher, 4_000);
    }

    #[bench]
    pub fn bench_insert_rng_05_000(bencher: &mut Bencher) {
        bench_rng(bencher, 5_000);
    }

    #[bench]
    pub fn bench_insert_rng_06_000(bencher: &mut Bencher) {
        bench_rng(bencher, 6_000);
    }

    #[bench]
    pub fn bench_insert_rng_07_000(bencher: &mut Bencher) {
        bench_rng(bencher, 7_000);
    }

    #[bench]
    pub fn bench_insert_rng_08_000(bencher: &mut Bencher) {
        bench_rng(bencher, 8_000);
    }

    #[bench]
    pub fn bench_insert_rng_09_000(bencher: &mut Bencher) {
        bench_rng(bencher, 9_000);
    }

    #[bench]
    pub fn bench_insert_rng_10_000(bencher: &mut Bencher) {
        bench_rng(bencher, 10_000);
    }


    #[bench]
    pub fn bench_insert_inc_01_000(bencher: &mut Bencher) {
        bench_inc(bencher, 1_000);
    }

    #[bench]
    pub fn bench_insert_inc_02_000(bencher: &mut Bencher) {
        bench_inc(bencher, 2_000);
    }

    #[bench]
    pub fn bench_insert_inc_03_000(bencher: &mut Bencher) {
        bench_inc(bencher, 3_000);
    }

    #[bench]
    pub fn bench_insert_inc_04_000(bencher: &mut Bencher) {
        bench_inc(bencher, 4_000);
    }

    #[bench]
    pub fn bench_insert_inc_05_000(bencher: &mut Bencher) {
        bench_inc(bencher, 5_000);
    }

    #[bench]
    pub fn bench_insert_inc_06_000(bencher: &mut Bencher) {
        bench_inc(bencher, 6_000);
    }

    #[bench]
    pub fn bench_insert_inc_07_000(bencher: &mut Bencher) {
        bench_inc(bencher, 7_000);
    }

    #[bench]
    pub fn bench_insert_inc_08_000(bencher: &mut Bencher) {
        bench_inc(bencher, 8_000);
    }

    #[bench]
    pub fn bench_insert_inc_09_000(bencher: &mut Bencher) {
        bench_inc(bencher, 9_000);
    }


    #[bench]
    pub fn bench_insert_inc_10_000(bencher: &mut Bencher) {
        bench_inc(bencher, 10_000);
    }

    #[bench]
    pub fn bench_insert_inc_20_000(bencher: &mut Bencher) {
        bench_inc(bencher, 20_000);
    }

    #[bench]
    pub fn bench_insert_inc_30_000(bencher: &mut Bencher) {
        bench_inc(bencher, 30_000);
    }

    #[bench]
    pub fn bench_insert_inc_40_000(bencher: &mut Bencher) {
        bench_inc(bencher, 40_000);
    }

    #[bench]
    pub fn bench_insert_inc_50_000(bencher: &mut Bencher) {
        bench_inc(bencher, 50_000);
    }




    #[bench]
    pub fn bench_insert_dec_01_000(bencher: &mut Bencher) {
        bench_dec(bencher, 1_000);
    }

    #[bench]
    pub fn bench_insert_dec_02_000(bencher: &mut Bencher) {
        bench_dec(bencher, 2_000);
    }

    #[bench]
    pub fn bench_insert_dec_03_000(bencher: &mut Bencher) {
        bench_dec(bencher, 3_000);
    }

    #[bench]
    pub fn bench_insert_dec_04_000(bencher: &mut Bencher) {
        bench_dec(bencher, 4_000);
    }

    #[bench]
    pub fn bench_insert_dec_05_000(bencher: &mut Bencher) {
        bench_dec(bencher, 5_000);
    }

    #[bench]
    pub fn bench_insert_dec_06_000(bencher: &mut Bencher) {
        bench_dec(bencher, 6_000);
    }

    #[bench]
    pub fn bench_insert_dec_07_000(bencher: &mut Bencher) {
        bench_dec(bencher, 7_000);
    }

    #[bench]
    pub fn bench_insert_dec_08_000(bencher: &mut Bencher) {
        bench_dec(bencher, 8_000);
    }

    #[bench]
    pub fn bench_insert_dec_09_000(bencher: &mut Bencher) {
        bench_dec(bencher, 9_000);
    }

    #[bench]
    pub fn bench_insert_dec_10_000(bencher: &mut Bencher) {
        bench_dec(bencher, 10_000);
    }
}
