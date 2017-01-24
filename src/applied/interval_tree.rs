use applied::interval::{Interval, IvNode};
use base::{TreeRepr, TeardownTreeRefill, NoopFilter, Node, KeyVal, BulkDeleteCommon, ItemVisitor, ItemFilter, lefti, righti, parenti};
use base::drivers::{consume_unchecked};
use std::ops::{Deref, DerefMut};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::cell::UnsafeCell;
use std::{cmp, fmt, ptr, mem};

pub struct IvTree<Iv: Interval, V> {
    pub repr: UnsafeCell<TreeRepr<IvNode<Iv, V>>>,
}

impl<Iv: Interval, V> IvTree<Iv, V> {
    /// Constructs a new IvTree
    pub fn new(items: Vec<(Iv, V)>) -> IvTree<Iv, V> {
        Self::with_repr(TreeRepr::new(items))
    }

    pub fn with_repr(repr: TreeRepr<IvNode<Iv, V>>) -> IvTree<Iv, V> {
        IvTree { repr: UnsafeCell::new(repr) }
    }

    /// Constructs a new IvTree
    /// Note: the argument must be sorted!
    pub fn with_sorted(sorted: Vec<(Iv, V)>) -> IvTree<Iv, V> {
        Self::with_repr(TreeRepr::with_sorted(sorted))
    }

    pub fn with_nodes(nodes: Vec<Option<IvNode<Iv, V>>>) -> IvTree<Iv, V> {
        Self::with_repr(TreeRepr::with_nodes(nodes))
    }

    /// Deletes the item with the given key from the tree and returns it (or None).
    #[inline]
    pub fn delete(&mut self, search: &Iv) -> Option<V> {
        self.work(NoopFilter, |tree| tree.delete(search))
    }

    #[inline]
    pub fn delete_intersecting(&mut self, search: &Iv, output: &mut Vec<(Iv, V)>) {
        self.filter_intersecting(search, NoopFilter, output)
    }

    #[inline]
    pub fn filter_intersecting<Flt>(&mut self, search: &Iv, filter: Flt, output: &mut Vec<(Iv, V)>)
        where Flt: ItemFilter<Iv>
    {
        self.work(filter, |worker: &mut IvWorker<Iv,V,Flt>| worker.filter_intersecting(search, output))
    }


    #[inline]
    fn work<Flt, F, R>(&mut self, filter: Flt, mut f: F) -> R where Flt: ItemFilter<Iv>,
                                                                    F: FnMut(&mut IvWorker<Iv,V,Flt>) -> R
    {
        let repr: TreeRepr<IvNode<Iv, V>> = unsafe {
            ptr::read(self.repr.get())
        };

        let mut worker = IvWorker::new(repr, filter);
        let result = f(&mut worker);

        unsafe {
            let x = mem::replace(&mut *self.repr.get(), worker.repr);
            mem::forget(x);
        }

        result
    }


    fn repr(&self) -> &TreeRepr<IvNode<Iv, V>> {
        unsafe { &*self.repr.get() }
    }

    fn repr_mut(&mut self) -> &mut TreeRepr<IvNode<Iv, V>> {
        unsafe { &mut *self.repr.get() }
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



pub struct IvWorker<Iv: Interval, V, Flt> where Flt: ItemFilter<Iv> {
    repr: TreeRepr<IvNode<Iv, V>>,
    filter: Flt
}

impl<Iv: Interval, V, Flt> IvWorker<Iv, V, Flt> where Flt: ItemFilter<Iv> {
    /// Constructs a new FilterTree
    pub fn new(repr: TreeRepr<IvNode<Iv, V>>, filter: Flt) -> Self {
        IvWorker { repr:repr, filter:filter }
    }

    /// Deletes the item with the given key from the tree and returns it (or None).
    #[inline]
    pub fn delete(&mut self, search: &Iv) -> Option<V> {
        self.index_of(search).map(|idx| {
            let kv = self.delete_idx(idx);
            self.update_ancestors_after_delete(idx, &kv.key.b());
            kv.val
        })
    }

    #[inline]
    pub fn filter_intersecting(&mut self, search: &Iv, output: &mut Vec<(Iv, V)>) {
        if self.size() != 0 {
            output.reserve(self.size());
            UpdateMax::visit(self, 0, move |this, _|
                this.filter_intersecting_ivl_rec(search, 0, false, output)
            )
        }
    }



    #[inline]
    fn update_maxb(&mut self, idx: usize) {
        let node = self.node_mut_unsafe(idx);

        let left_self_maxb =
            if self.has_left(idx) {
                cmp::max(&self.left(idx).maxb, node.key.b())
            } else {
                node.key.b()
            }.clone();
        node.maxb =
            if self.has_right(idx) {
                cmp::max(self.right(idx).maxb.clone(), left_self_maxb)
            } else {
                left_self_maxb
            };
    }

    #[inline]
    fn update_ancestors_after_delete(&mut self, mut idx: usize, removed_b: &Iv::K) {
        while idx != 0 {
            idx = parenti(idx);
            if removed_b == &self.node(idx).maxb {
                self.update_maxb(idx);
            } else {
                break;
            }
        }
    }

    #[inline]
    fn delete_idx(&mut self, idx: usize) -> KeyVal<Iv, V> {
        debug_assert!(!self.is_nil(idx));

        let node = self.node_mut_unsafe(idx);

        let repl_kv = match (self.has_left(idx), self.has_right(idx)) {
            (false, false) => {
                let IvNode{kv, ..} = self.take(idx);
                kv
            },

            (true, false)  => {
                let (kv, left_maxb) = self.delete_max(lefti(idx));
                node.maxb = left_maxb;
                kv
            },

            (false, true)  => {
//                let (removed, right_maxb) = self.delete_min(righti(idx));
//                item.maxb = right_maxb;
                let kv = self.delete_min(righti(idx));
                if &node.maxb == kv.key.b() {
                    self.update_maxb(idx)
                } else { // maxb remains the same
                    debug_assert!(kv.key.b() < &node.maxb);
                }
                kv
            },

            (true, true)   => {
                let (kv, left_maxb) = self.delete_max(lefti(idx));
                if &node.maxb == kv.key.b() {
                    node.maxb = cmp::max(left_maxb, self.right(idx).maxb.clone());
                } else { // maxb remains the same
                    debug_assert!(kv.key.b() < &node.maxb);
                }
                kv
            },
        };

        mem::replace(&mut node.kv, repl_kv)
    }


    /// returns the removed max-item of this subtree and the old maxb (before removal)
    #[inline]
    // we attempt to reduce the number of memory accesses as much as possible; might be overengineered
    fn delete_max(&mut self, idx: usize) -> (KeyVal<Iv,V>, Iv::K) {
        let max_idx = self.find_max(idx);

        let (kv, mut old_maxb, mut new_maxb) = if self.has_left(max_idx) {
            let node = self.node_mut_unsafe(max_idx);
            let (left_max_kv, left_maxb) = self.delete_max(lefti(max_idx));
            let kv = mem::replace(&mut node.kv, left_max_kv);

            let old_maxb = mem::replace(&mut node.maxb, left_maxb.clone());
            (kv, old_maxb, Some(left_maxb))
        } else {
            let IvNode { kv, maxb:old_maxb } = self.take(max_idx);
            (kv, old_maxb, None)
        };

        // update ancestors
        let mut upd_idx = max_idx;
        while upd_idx != idx {
            upd_idx = parenti(upd_idx);

            let node = self.node_mut_unsafe(upd_idx);
            old_maxb = node.maxb.clone();
            if &node.maxb == kv.key.b() {
                let mb = {
                    let self_left_maxb =
                        if self.has_left(upd_idx) {
                            cmp::max(&self.left(upd_idx).maxb, &node.maxb)
                        } else {
                            &node.maxb
                        };

                    new_maxb.map_or(self_left_maxb.clone(),
                                    |mb| cmp::max(mb, self_left_maxb.clone()))
                };
                node.maxb = mb.clone();
                new_maxb = Some(mb);
            } else {
                new_maxb = Some(old_maxb.clone());
            }
        }

        (kv, old_maxb)
    }

    // TODO: check whether optimizations similar to delete_max() are worth it
    #[inline]
    fn delete_min(&mut self, idx: usize) -> KeyVal<Iv,V> {
        let min_idx = self.find_min(idx);

        let replacement_kv = if self.has_right(min_idx) {
            let right_min_kv = self.delete_min(righti(min_idx));
            let node = self.node_mut_unsafe(min_idx);

            if self.has_right(min_idx) {
                let right_maxb = &self.right(min_idx).maxb;
                node.maxb = cmp::max(right_maxb, right_min_kv.key.b()).clone();
            } else {
                node.maxb = right_min_kv.key.b().clone();
            }

            mem::replace(&mut node.kv, right_min_kv)
        } else {
            let IvNode{kv, ..} = self.take(min_idx);
            kv
        };

        // update ancestors
        let mut upd_idx = min_idx;
        while upd_idx != idx {
            upd_idx = parenti(upd_idx);
            self.update_maxb(upd_idx);
        }

        replacement_kv
    }



    #[inline(never)]
    fn filter_intersecting_ivl_rec(&mut self, search: &Iv, idx: usize, min_included: bool, output: &mut Vec<(Iv, V)>) {
        let node = self.node_unsafe(idx);
        let k: &Iv = &node.key;

        if &node.maxb < search.a() {
            // whole subtree outside the range
            if self.slots_min().has_open() {
                self.fill_slots_min(idx);
            }
            if self.slots_max().has_open() && !self.is_nil(idx) {
                self.fill_slots_max(idx);
            }
        } else if search.b() <= k.a() && k.a() != search.a() {
            // root and right are outside the range
            self.descend_filter_intersecting_ivl_left(search, idx, false, min_included, output);

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
            let consumed = if search.intersects(k)
                { self.filter_take(idx) }
            else
                { None };

            // left subtree
            let mut removed = consumed.is_some();
            if let Some(item) = consumed {
                if min_included {
                    self.consume_subtree(lefti(idx), output)
                } else {
                    removed = self.descend_filter_intersecting_ivl_left(search, idx, true, false, output);
                }

                consume_unchecked(output, item.into_kv());
            } else {
                removed = self.descend_filter_intersecting_ivl_left(search, idx, false, min_included, output);
                if !removed && self.slots_min().has_open() {
                    removed = true;
                    self.fill_slot_min(idx);
                }
            }

            // right subtree
            let right_min_included = min_included || search.a() <= k.a();
            if right_min_included {
                let right_max_included = &node.maxb < search.b();
                if right_max_included {
                    self.consume_subtree(righti(idx), output);
                } else {
                    removed = self.descend_filter_intersecting_ivl_right(search, idx, removed, true, output);
                }
            } else {
                removed = self.descend_filter_intersecting_ivl_right(search, idx, removed, false, output);
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
    fn descend_filter_intersecting_ivl_left(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, output: &mut Vec<(Iv, V)>) -> bool {
        // this pinning business is asymmetric (we don't do it in descend_delete_intersecting_ivl_right) because of the program flow: we enter the left subtree first
        self.descend_left_fresh_slots(idx, with_slot,
                                      |this: &mut Self, child_idx| this.filter_intersecting_ivl_rec(search, child_idx, min_included, output))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_filter_intersecting_ivl_right(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, output: &mut Vec<(Iv, V)>) -> bool {
        self.descend_right(idx, with_slot,
                           |this: &mut Self, child_idx| this.filter_intersecting_ivl_rec(search, child_idx, min_included, output))
    }
}



impl<Iv: Interval, V, Flt: ItemFilter<Iv>> Deref for IvWorker<Iv, V, Flt> {
    type Target = TreeRepr<IvNode<Iv, V>>;

    fn deref(&self) -> &Self::Target {
        &self.repr
    }
}

impl<Iv: Interval, V, Flt: ItemFilter<Iv>> DerefMut for IvWorker<Iv, V, Flt> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.repr
    }
}

impl<Iv: Interval, V, Flt: ItemFilter<Iv>> BulkDeleteCommon<IvNode<Iv, V>> for IvWorker<Iv, V, Flt> {
    type Visitor = UpdateMax<Iv, Flt>;
    type Filter = Flt;

    fn filter_mut(&mut self) -> &mut Self::Filter {
        &mut self.filter
    }
}


impl<Iv: Interval, V, Flt: ItemFilter<Iv>> TeardownTreeRefill for IvWorker<Iv, V, Flt> where Iv: Copy, V: Copy {
    fn refill(&mut self, master: &IvWorker<Iv, V, Flt>) {
        self.repr.refill(&master.repr);
    }
}



pub struct UpdateMax<Iv: Interval, Flt: ItemFilter<Iv>> {
    _ph: PhantomData<(Iv, Flt)>
}

impl<Iv: Interval, V, Flt: ItemFilter<Iv>> ItemVisitor<IvNode<Iv, V>> for UpdateMax<Iv, Flt> {
    type Tree = IvWorker<Iv, V, Flt>;

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
                node.maxb = node.key.b().clone(),
            (false, true) =>
                node.maxb = cmp::max(node.key.b(), &tree.node(righti(idx)).maxb).clone(),
            (true, false) =>
                node.maxb = cmp::max(node.key.b(), &tree.node(lefti(idx)).maxb).clone(),
            (true, true) =>
                node.maxb = cmp::max(node.key.b(),
                                     cmp::max(&tree.node(lefti(idx)).maxb, &tree.node(righti(idx)).maxb))
                                    .clone(),
        }
    }
}
