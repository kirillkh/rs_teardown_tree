use applied::AppliedTree;
use applied::interval::{Interval, IvNode};
use base::{TreeRepr, Sink, NoopFilter, Node, Entry, BulkDeleteCommon, ItemVisitor, ItemFilter, lefti, righti, parenti};

use std::ops::{Deref, DerefMut};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::cell::UnsafeCell;
use std::{cmp, fmt, ptr, mem};

pub struct IvTree<Iv: Interval, V> {
    pub repr: UnsafeCell<TreeRepr<IvNode<Iv, V>>>,
}

//---- constructors and helpers --------------------------------------------------------------------
impl<Iv: Interval, V> IvTree<Iv, V> {
    // assumes a contiguous layout of nodes (no holes)
    fn init_maxb(&mut self) {
        // initialize maxb values
        for i in (1..self.size()).rev() {
            let parent = self.node_mut_unsafe(parenti(i));
            let node = self.node(i);

            if node.maxb > parent.maxb {
                parent.maxb = node.maxb.clone()
            }
        }
    }

    fn repr(&self) -> &TreeRepr<IvNode<Iv, V>> {
        unsafe { &*self.repr.get() }
    }

    fn repr_mut(&mut self) -> &mut TreeRepr<IvNode<Iv, V>> {
        unsafe { &mut *self.repr.get() }
    }


    #[inline]
    fn update_maxb(&mut self, idx: usize) {
        let node = self.node_mut_unsafe(idx);

        let left_self_maxb =
            if self.has_left(idx) {
                cmp::max(&self.left(idx).maxb, node.key().b())
            } else {
                node.key().b()
            }.clone();
        node.maxb =
            if self.has_right(idx) {
                cmp::max(self.right(idx).maxb.clone(), left_self_maxb)
            } else {
                left_self_maxb
            };
    }

    #[inline]
    fn update_ancestors_after_delete(&mut self, mut idx: usize, idx_to: usize, removed_b: &Iv::K) {
        while idx != idx_to {
            idx = parenti(idx);
            if removed_b == &self.node(idx).maxb {
                self.update_maxb(idx);
            } else {
                break;
            }
        }
    }
}

//---- single-item queries -------------------------------------------------------------------------
impl<Iv: Interval, V> IvTree<Iv, V> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    #[inline]
    pub fn delete<Q>(&mut self, query: &Q) -> Option<V>
        where Q: PartialOrd<Iv>
    {
        let idx = self.index_of(query);
        if self.is_nil(idx) {
            None
        } else {
            let entry = self.delete_idx(idx);
            self.update_ancestors_after_delete(idx, 0, entry.key().b());
            Some(entry.into_tuple().1)
        }
    }

    #[inline]
    fn delete_idx(&mut self, idx: usize) -> Entry<Iv, V> {
        debug_assert!(!self.is_nil(idx));

        let repl_entry = if self.has_left(idx) {
            self.delete_max(lefti(idx))
        } else if self.has_right(idx) {
            self.delete_min(righti(idx))
        } else {
            let IvNode{entry, ..} = self.take(idx);
            return entry;
        };

        let entry = mem::replace(&mut self.node_mut(idx).entry, repl_entry);
        self.update_maxb(idx);
        entry
    }

    #[inline]
    fn delete_max(&mut self, idx: usize) -> Entry<Iv, V> {
        let max_idx = self.find_max(idx);
        let entry = if self.has_left(max_idx) {
            let repl_entry = self.delete_max(lefti(max_idx));
            let entry = mem::replace(&mut self.node_mut(max_idx).entry, repl_entry);
            self.update_maxb(max_idx);
            entry
        } else {
            let IvNode{entry, ..} = self.take(max_idx);
            entry
        };

        self.update_ancestors_after_delete(max_idx, idx, entry.key().b());
        entry
    }

    #[inline]
    fn delete_min(&mut self, idx: usize) -> Entry<Iv, V> {
        let min_idx = self.find_min(idx);
        let entry = if self.has_right(min_idx) {
            let repl_entry = self.delete_min(righti(min_idx));
            let entry = mem::replace(&mut self.node_mut(min_idx).entry, repl_entry);
            self.update_maxb(min_idx);
            entry
        } else {
            let IvNode{entry, ..} = self.take(min_idx);
            entry
        };

        self.update_ancestors_after_delete(min_idx, idx, entry.key().b());
        entry
    }
}

//---- range queries -------------------------------------------------------------------------------
impl<Iv: Interval, V> IvTree<Iv, V> {
    #[inline]
    pub fn delete_overlap<Q, S>(&mut self, query: &Q, sink: S)
        where Q: Interval<K=Iv::K>, S: Sink<(Iv, V)>
    {
        self.filter_overlap(query, sink, NoopFilter)
    }

    #[inline]
    pub fn filter_overlap<Q, Flt, S>(&mut self, query: &Q, sink: S, filter: Flt)
        where Q: Interval<K=Iv::K>,
              Flt: ItemFilter<Iv>,
              S: Sink<(Iv, V)>
    {
        self.work(sink, filter, |worker: &mut IvWorker<Iv,V,S,Flt>| worker.filter_overlap(query))
    }


    pub fn query_overlap<'a, Q, S>(&'a self, idx: usize, query: &Q, sink: S)
        where Q: Interval<K=Iv::K>,
              S: Sink<&'a Entry<Iv, V>>
    {
        self.work(sink, NoopFilter, |worker: &mut IvWorker<Iv,V,S,_>| worker.query_overlap_rec(idx, query))
    }

//    /// returns index of the first item in the tree that may overlap `query`
//    fn lower_bound<Q: Interval<K=Iv::K>>(&self, query: &Q) -> usize {
//        let mut parent = 0;
//        let mut next = 0;
//        while !self.is_nil(next) {
//            if &self.node(next).maxb <= query.a() {
//                // whole subtree outside the range
//                if next == parent {
//                    return self.size();
//                } else {
//                    return parent;
//                }
//            }
//
//            parent = next;
//            next = lefti(parent);
//        }
//
//        parent
//    }

    #[inline]
    fn work<S, Flt, F, R>(&self, sink: S, filter: Flt, mut f: F) -> R
        where Flt: ItemFilter<Iv>,
              F: FnMut(&mut IvWorker<Iv,V,S,Flt>) -> R
    {
        let repr: TreeRepr<IvNode<Iv, V>> = unsafe {
            ptr::read(self.repr.get())
        };

        let mut worker = IvWorker::new(repr, sink, filter);
        let result = f(&mut worker);

        unsafe {
            let x = mem::replace(&mut *self.repr.get(), worker.repr);
            mem::forget(x);
        }

        result
    }
}


impl<Iv: Interval, V> AppliedTree<IvNode<Iv, V>> for IvTree<Iv, V> {
    /// Constructs a new AppliedTree
    fn new(items: Vec<(Iv, V)>) -> Self {
        let mut tree = Self::with_repr(TreeRepr::new(items));
        tree.init_maxb();
        tree
    }

    /// Constructs a new IvTree
    /// Note: the argument must be sorted!
    fn with_sorted(sorted: Vec<(Iv, V)>) -> Self {
        let mut tree = Self::with_repr(TreeRepr::with_sorted(sorted));
        tree.init_maxb();
        tree
    }

    fn with_repr(repr: TreeRepr<IvNode<Iv, V>>) -> IvTree<Iv, V> {
        IvTree { repr: UnsafeCell::new(repr) }
    }

    unsafe fn with_shape(shape: Vec<Option<(Iv, V)>>) -> IvTree<Iv, V> {
        let nodes = shape.into_iter()
            .map(|opt| opt.map(|(k, v)| IvNode::new(k.clone(), v)))
            .collect::<Vec<_>>();
        let mut tree = Self::with_nodes(nodes);

        // initialize maxb values
        for i in (1..tree.data.len()).rev() {
            if !tree.is_nil(i) {
                let parent = tree.node_mut_unsafe(parenti(i));
                let node = tree.node(i);

                if node.maxb > parent.maxb {
                    parent.maxb = node.maxb.clone()
                }
            }
        }

        tree
    }


}



impl<Iv: Interval, V> Deref for IvTree<Iv, V> {
    type Target = TreeRepr<IvNode<Iv, V>>;

    fn deref(&self) -> &Self::Target {
        self.repr()
    }
}

impl<Iv: Interval, V> DerefMut for IvTree<Iv, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.repr_mut()
    }
}


impl<Iv: Interval, V> Debug for IvTree<Iv, V> where Iv::K: Debug {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Debug::fmt(self.repr(), fmt)
    }
}

impl<Iv: Interval, V> Display for IvTree<Iv, V> where Iv::K: Debug {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Display::fmt(self.repr(), fmt)
    }
}

impl<Iv: Interval, V: Clone> Clone for IvTree<Iv, V> {
    fn clone(&self) -> Self {
        IvTree { repr: UnsafeCell::new(self.repr().clone()) }
    }
}



#[derive(new)]
pub struct IvWorker<Iv, V, S, Flt>
    where Iv: Interval
{
    repr: TreeRepr<IvNode<Iv, V>>,
    sink: S,
    filter: Flt,
}



impl<'a, Iv: 'a, V: 'a, S, Flt> IvWorker<Iv, V, S, Flt>
    where Iv: Interval, S: Sink<&'a Entry<Iv, V>>, Flt: ItemFilter<Iv>
{
    fn query_overlap_rec<Q>(&mut self, idx: usize, query: &Q)
        where Q: Interval<K=Iv::K>
    {
        if self.is_nil(idx) {
            return;
        }

        // This is safe: it is guaranteed that the container (Sink) does not outlive content (node entry)
        // by IvTree::query_overlap()'s requirement that S: Sink<&'a Entry<Iv, V>>.
        let node = self.node_unsafe(idx);
        let k: &Iv = node.entry.key();

        if &node.maxb < query.a() {
            // whole subtree outside the range
        } else if query.b() <= k.a() && k.a() != query.a() {
            // root and right are outside the range
            self.query_overlap_rec(lefti(idx), query);
        } else {
            self.query_overlap_rec(lefti(idx), query);
            if query.overlaps(k) { self.sink.consume(node) }
            self.query_overlap_rec(righti(idx), query);
        }
    }

}



impl<Iv, V, S, Flt> IvWorker<Iv, V, S, Flt>
    where Iv: Interval, S: Sink<(Iv, V)>, Flt: ItemFilter<Iv>
{
    #[inline]
    pub fn filter_overlap<Q>(&mut self, query: &Q)
        where Q: Interval<K=Iv::K>
    {
        if self.size() != 0 {
            UpdateMax::visit(self, 0, move |this, _|
                this.filter_overlap_ivl_rec(query, 0, false)
            )
        }
    }

//    #[inline]
//    fn delete_idx(&mut self, idx: usize) -> Entry<Iv, V> {
//        debug_assert!(!self.is_nil(idx));
//
//        let node = self.node_mut_unsafe(idx);
//
//        let repl_entry = match (self.has_left(idx), self.has_right(idx)) {
//            (false, false) => {
//                let IvNode{entry, ..} = self.take(idx);
//                entry
//            },
//
//            (true, false)  => {
//                let (entry, left_maxb) = self.delete_max(lefti(idx));
//                node.maxb = left_maxb;
//                entry
//            },
//
//            (false, true)  => {
////                let (removed, right_maxb) = self.delete_min(righti(idx));
////                item.maxb = right_maxb;
//                let entry = self.delete_min(righti(idx));
//                if &node.maxb == entry.key.b() {
//                    self.update_maxb(idx)
//                } else { // maxb remains the same
//                    debug_assert!(entry.key.b() < &node.maxb);
//                }
//                entry
//            },
//
//            (true, true)   => {
//                let (entry, left_maxb) = self.delete_max(lefti(idx));
//                if &node.maxb == entry.key.b() {
//                    node.maxb = cmp::max(left_maxb, self.right(idx).maxb.clone());
//                } else { // maxb remains the same
//                    debug_assert!(entry.key.b() < &node.maxb);
//                }
//                entry
//            },
//        };
//
//        mem::replace(&mut node.entry, repl_entry)
//    }
//
//
//    /// returns the removed max-item of this subtree and the old maxb (before removal)
//    #[inline]
//    // we attempt to reduce the number of memory accesses as much as possible; might be overengineered
//    fn delete_max(&mut self, idx: usize) -> (Entry<Iv,V>, Iv::K) {
//        let max_idx = self.find_max(idx);
//
//        let (entry, mut old_maxb, mut new_maxb) = if self.has_left(max_idx) {
//            let node = self.node_mut_unsafe(max_idx);
//            let (left_max_entry, left_maxb) = self.delete_max(lefti(max_idx));
//            let entry = mem::replace(&mut node.entry, left_max_entry);
//
//            let old_maxb = mem::replace(&mut node.maxb, left_maxb.clone());
//            (entry, old_maxb, Some(left_maxb))
//        } else {
//            let IvNode { entry, maxb:old_maxb } = self.take(max_idx);
//            (entry, old_maxb, None)
//        };
//
//        // update ancestors
//        let mut upd_idx = max_idx;
//        while upd_idx != idx {
//            upd_idx = parenti(upd_idx);
//
//            let node = self.node_mut_unsafe(upd_idx);
//            old_maxb = node.maxb.clone();
//            if &node.maxb == entry.key.b() {
//                let mb = {
//                    let self_left_maxb =
//                        if self.has_left(upd_idx) {
//                            cmp::max(&self.left(upd_idx).maxb, &node.maxb)
//                        } else {
//                            &node.maxb
//                        };
//
//                    new_maxb.map_or(self_left_maxb.clone(),
//                                    |mb| cmp::max(mb, self_left_maxb.clone()))
//                };
//                node.maxb = mb.clone();
//                new_maxb = Some(mb);
//            } else {
//                new_maxb = Some(old_maxb.clone());
//            }
//        }
//
//        (entry, old_maxb)
//    }
//
//    // TODO: check whether optimizations similar to delete_max() are worth it
//    #[inline]
//    fn delete_min(&mut self, idx: usize) -> Entry<Iv,V> {
//        let min_idx = self.find_min(idx);
//
//        let replacement_entry = if self.has_right(min_idx) {
//            let right_min_entry = self.delete_min(righti(min_idx));
//            let node = self.node_mut_unsafe(min_idx);
//
//            if self.has_right(min_idx) {
//                let right_maxb = &self.right(min_idx).maxb;
//                node.maxb = cmp::max(right_maxb, right_min_entry.key.b()).clone();
//            } else {
//                node.maxb = right_min_entry.key.b().clone();
//            }
//
//            mem::replace(&mut node.entry, right_min_entry)
//        } else {
//            let IvNode{entry, ..} = self.take(min_idx);
//            entry
//        };
//
//        // update ancestors
//        let mut upd_idx = min_idx;
//        while upd_idx != idx {
//            upd_idx = parenti(upd_idx);
//            self.update_maxb(upd_idx);
//        }
//
//        replacement_entry
//    }

    #[inline(never)]
    fn filter_overlap_ivl_rec<Q>(&mut self, query: &Q, idx: usize, min_included: bool)
        where Q: Interval<K=Iv::K>
    {
        let node = self.node_mut_unsafe(idx);
        let k: &Iv = node.entry.key();

        if &node.maxb < query.a() {
            // whole subtree outside the range
            if self.slots_min().has_open() {
                self.fill_slots_min(idx);
            }
            if self.slots_max().has_open() && !self.is_nil(idx) {
                self.fill_slots_max(idx);
            }
        } else if query.b() <= k.a() && k.a() != query.a() {
            // root and right are outside the range
            self.descend_filter_overlap_ivl_left(query, idx, false, min_included);

            let removed = if self.slots_min().has_open() {
                self.fill_slot_min(idx);

                self.descend_fill_min_right(idx, true)
            } else {
                false
            };

            if self.slots_max().has_open() {
                self.descend_fill_max_left(idx, removed);
            }
        } else {
            // consume root if necessary
            let consumed = if query.overlaps(k)
                { self.filter_take(idx) }
            else
                { None };

            // left subtree
            let mut removed: bool;
            if let Some(consumed) = consumed {
                if min_included {
                    removed = self.descend_consume_left(idx, true);
                } else {
                    removed = self.descend_filter_overlap_ivl_left(query, idx, true, false);
                }
                node.maxb = consumed.maxb.clone();

                self.sink.consume(consumed.into_tuple())
            } else {
                self.descend_filter_overlap_ivl_left(query, idx, false, min_included);
                if self.slots_min().has_open() {
                    removed = true;
                    self.fill_slot_min(idx);
                } else {
                    removed = false;
                }
            }

            // right subtree
            let right_min_included = min_included || query.a() <= k.a();
            if right_min_included {
                let right_max_included = &node.maxb < query.b();
                if right_max_included {
                    removed = self.descend_consume_right(idx, removed);
                } else {
                    removed = self.descend_filter_overlap_ivl_right(query, idx, removed, true);
                }
            } else {
                removed = self.descend_filter_overlap_ivl_right(query, idx, removed, false);
            }

            if !removed && self.slots_max().has_open() {
                removed = true;
                self.fill_slot_max(idx);
            }

            // fill the remaining open slots_max from the left subtree
            if removed {
                self.descend_fill_max_left(idx, true);
            }
        }
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_filter_overlap_ivl_left<Q>(&mut self, query: &Q, idx: usize, with_slot: bool, min_included: bool) -> bool
        where Q: Interval<K=Iv::K>
    {
        // this pinning business is asymmetric (we don't do it in descend_delete_overlap_ivl_right) because of the program flow: we enter the left subtree first
        self.descend_left_fresh_slots(idx, with_slot,
                                      |this: &mut Self, child_idx| this.filter_overlap_ivl_rec(query, child_idx, min_included))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_filter_overlap_ivl_right<Q>(&mut self, query: &Q, idx: usize, with_slot: bool, min_included: bool) -> bool
        where Q: Interval<K=Iv::K>
    {
        self.descend_right(idx, with_slot,
                           |this: &mut Self, child_idx| this.filter_overlap_ivl_rec(query, child_idx, min_included))
    }
}



impl<Iv, V, S, Flt> Deref for IvWorker<Iv, V, S, Flt>
    where Iv: Interval, Flt: ItemFilter<Iv>
{
    type Target = TreeRepr<IvNode<Iv, V>>;

    fn deref(&self) -> &Self::Target {
        &self.repr
    }
}

impl<Iv, V, S, Flt> DerefMut for IvWorker<Iv, V, S, Flt>
    where Iv: Interval, Flt: ItemFilter<Iv>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.repr
    }
}

impl<Iv, V, S, Flt> BulkDeleteCommon<IvNode<Iv, V>> for IvWorker<Iv, V, S, Flt>
    where Iv: Interval, S: Sink<(Iv, V)>, Flt: ItemFilter<Iv>
{
    type Visitor = UpdateMax<Iv, S, Flt>;
    type Sink = S;
    type Filter = Flt;

    fn filter_mut(&mut self) -> &mut Self::Filter {
        &mut self.filter
    }

    fn sink_mut(&mut self) -> &mut Self::Sink {
        &mut self.sink
    }
}



pub struct UpdateMax<Iv, S, Flt> {
    _ph: PhantomData<(Iv, S, Flt)>
}

impl<Iv, V, S, Flt> ItemVisitor<IvNode<Iv, V>> for UpdateMax<Iv, S, Flt>
    where Iv: Interval, S: Sink<(Iv, V)>, Flt: ItemFilter<Iv>
{
    type Tree = IvWorker<Iv, V, S, Flt>;

    #[inline]
    fn visit<F>(tree: &mut Self::Tree, idx: usize, mut f: F)
        where F: FnMut(&mut Self::Tree, usize)
    {
        f(tree, idx);

        if tree.is_nil(idx) {
            return;
        }

        let node = &mut tree.node_mut_unsafe(idx);
        match (tree.has_left(idx), tree.has_right(idx)) {
            (false, false) =>
                node.maxb = node.key().b().clone(),
            (false, true) =>
                node.maxb = cmp::max(node.key().b(), &tree.node(righti(idx)).maxb).clone(),
            (true, false) =>
                node.maxb = cmp::max(node.key().b(), &tree.node(lefti(idx)).maxb).clone(),
            (true, true) =>
                node.maxb = cmp::max(node.key().b(),
                                     cmp::max(&tree.node(lefti(idx)).maxb, &tree.node(righti(idx)).maxb))
                                    .clone(),
        }
    }
}
