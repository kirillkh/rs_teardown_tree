use applied::interval::{Interval, IntervalNode};
use base::{TreeWrapper, TreeBase, BulkDeleteCommon, EnterItem, lefti, righti, parenti, Sink};
use base::drivers::{consume_ptr, consume_unchecked};
use std::{mem, cmp};
use std::marker::PhantomData;


pub trait IntervalTreeInternal<Iv: Interval> {
    #[inline] fn delete(&mut self, search: &IntervalNode<Iv>) -> Option<Iv>;
    #[inline] fn delete_intersecting(&mut self, search: &Iv, idx: usize, output: &mut Vec<Iv>);
}

impl<Iv: Interval> IntervalTreeInternal<Iv> for TreeWrapper<IntervalNode<Iv>> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    // TODO: accepting IntervalNode is super ugly, temporary solution only
    #[inline]
    fn delete(&mut self, search: &IntervalNode<Iv>) -> Option<Iv> {
        self.index_of(search).map(|idx| {
            let removed = self.delete_idx(idx);
            let mut upd_idx = idx;
            while upd_idx != 0 {
                upd_idx = parenti(upd_idx);
                if removed.b() == &self.item(idx).maxb {
                    self.update_maxb(idx);
                } else {
                    break;
                }
            }
            removed
        })
    }

    #[inline]
    fn delete_intersecting(&mut self, search: &Iv, idx: usize, output: &mut Vec<Iv>) {
        UpdateMax::enter(self, 0, move |this, idx|
            this.delete_intersecting_ivl_rec(search, idx, false, &mut self::IntervalSink { output: output })
        )
    }
}


trait IntervalDelete<Iv: Interval>: TreeBase<IntervalNode<Iv>> {
    #[inline]
    fn update_maxb(&mut self, idx: usize) {
        let item = self.item_mut_unsafe(idx);

        let left_self_maxb =
            if self.has_left(idx) {
                cmp::max(&self.left(idx).item.maxb, item.ivl.b())
            } else {
                item.ivl.b()
            };
        item.maxb =
            if self.has_right(idx) {
                cmp::max(&self.right(idx).item.maxb, left_self_maxb)
            } else {
                left_self_maxb
            }.clone();
    }

    #[inline]
    fn delete_idx(&mut self, idx: usize) -> Iv {
        debug_assert!(!self.is_nil(idx));

        let item = self.item_mut_unsafe(idx);

        let replacement: Iv = match (self.has_left(idx), self.has_right(idx)) {
            (false, false) => {
                let item = self.take(idx);
                return item.ivl
            },

            (true, false)  => {
                let (removed, left_maxb) = self.delete_max(lefti(idx));
                item.maxb = left_maxb;
                removed
            },

            (false, true)  => {
//                let (removed, right_maxb) = self.delete_min(righti(idx));
//                item.maxb = right_maxb;
                let removed = self.delete_min(righti(idx));
                if &item.maxb == removed.b() {
                    self.update_maxb(idx)
                } else { // maxb remains the same
                    debug_assert!(removed.b() < &item.maxb);
                }
                removed
            },

            (true, true)   => {
                let (removed, left_maxb) = self.delete_max(lefti(idx));
                if &item.maxb == removed.b() {
                    item.maxb = cmp::max(left_maxb, self.right(idx).item.maxb.clone());
                } else { // maxb remains the same
                    debug_assert!(removed.b() < &item.maxb);
                }
                removed
            },
        };

        mem::replace(&mut item.ivl, replacement)
    }


    /// returns the removed max-item of this subtree and the old maxb (before removal)
    #[inline]
    // we attempt to reduce the number of memory accesses as much as possible; might be overengineered
    fn delete_max(&mut self, idx: usize) -> (Iv, Iv::K) {
        let max_idx = self.find_max(idx);

        let (removed, mut old_maxb, mut new_maxb) = if self.has_left(max_idx) {
            let item = self.item_mut_unsafe(max_idx);
            let (left_max, left_maxb) = self.delete_max(lefti(max_idx));
            let removed = mem::replace(&mut item.ivl, left_max);

            let old_maxb = mem::replace(&mut item.maxb, left_maxb.clone());
            (removed, old_maxb, Some(left_maxb))
        } else {
            let IntervalNode { ivl, maxb:old_maxb } = self.take(max_idx);
            (ivl, old_maxb, None)
        };

        // update ancestors
        let mut upd_idx = max_idx;
        while upd_idx != idx {
            upd_idx = parenti(upd_idx);

            let item = self.item_mut_unsafe(upd_idx);
            old_maxb = item.maxb.clone();
            if &item.maxb == removed.b() {
                let mb = {
                    let self_left_maxb =
                        if self.has_left(upd_idx) {
                            cmp::max(&self.left(upd_idx).item.maxb, &item.maxb)
                        } else {
                            &item.maxb
                        };

                    new_maxb.map_or(self_left_maxb.clone(),
                                    |mb| cmp::max(mb, self_left_maxb.clone()))
                };
                item.maxb = mb.clone();
                new_maxb = Some(mb);
            } else {
                new_maxb = Some(old_maxb.clone());
            }
        }

        (removed, old_maxb)
    }

    // TODO: check whether optimizations similar to delete_max() are worth it
    #[inline]
    fn delete_min(&mut self, idx: usize) -> Iv {
        let min_idx = self.find_min(idx);

        let removed = if self.has_right(min_idx) {
            let right_min = self.delete_min(righti(min_idx));
            let item = self.item_mut_unsafe(min_idx);

            if self.has_right(min_idx) {
                let right_maxb = &self.right(min_idx).item.maxb;
                item.maxb = cmp::max(right_maxb, right_min.b()).clone();
            } else {
                item.maxb = right_min.b().clone();
            }

            mem::replace(&mut item.ivl, right_min)
        } else {
            self.take(min_idx).ivl
        };

        // update ancestors
        let mut upd_idx = min_idx;
        while upd_idx != idx {
            upd_idx = parenti(upd_idx);
            self.update_maxb(upd_idx);
        }

        removed
    }
}


trait IntervalDeleteRange<Iv: Interval>: BulkDeleteCommon<IntervalNode<Iv>,
//                                                          UpdateMax<TreeWrapper<IntervalNode<Iv>>>
                                                          UpdateMax<Iv, Self>
                                                         > + IntervalDelete<Iv> {
    fn delete_intersecting_ivl_rec<S: Sink<IntervalNode<Iv>>>(&mut self, search: &Iv, idx: usize, mut min_included: bool, sink: &mut S) {
        let k: &IntervalNode<Iv> = &self.node_unsafe(idx).item;

        if k.max() <= search.a() {
            // whole subtree outside the range
            if self.slots_min().has_open() {
                self.fill_slots_min(idx);
            }
            if self.slots_max().has_open() && !self.is_nil(idx) {
                self.fill_slots_max(idx);
            }
        } else if search.b() <= k.a() {
            // root and right are outside the range
            self.descend_delete_intersecting_ivl_left(search, idx, false, min_included, sink);

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
            let consume = search.a() < k.b() && k.a() < search.b();
            let item = if consume
                { Some(self.take(idx)) }
            else
                { None };

            // left subtree
            let mut removed = consume;
            if consume {
                if min_included {
                    self.consume_subtree(lefti(idx), sink)
                } else {
                    removed = self.descend_delete_intersecting_ivl_left(search, idx, true, false, sink);
                }

                sink.consume_unchecked(item.unwrap());
            } else {
                removed = self.descend_delete_intersecting_ivl_left(search, idx, false, min_included, sink);
            }

            // right subtree
            min_included = min_included || search.a() <= k.a();
            if min_included {
                let max_included = k.max() <= search.b();
                if max_included {
                    self.consume_subtree(righti(idx), sink);
                } else {
                    removed = self.descend_delete_intersecting_ivl_right(search, idx, removed, min_included, sink);
                }
            } else {
                removed = self.descend_delete_intersecting_ivl_right(search, idx, removed, false, sink);
            }

            // fill the remaining open slots_max from the left subtree
            if removed && self.slots_max().has_open() {
                self.descend_fill_max_left(righti(idx), true);
            }
        }
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_intersecting_ivl_left<S: Sink<IntervalNode<Iv>>>(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, sink: &mut S) -> bool {
        self.descend_left(idx, with_slot,
                          |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included, sink))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_intersecting_ivl_right<S: Sink<IntervalNode<Iv>>>(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, sink: &mut S) -> bool {
        self.descend_right(idx, with_slot,
                           |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included, sink))
    }
}


struct IntervalSink<'a, Iv: Interval+'a> {
    output: &'a mut Vec<Iv>
}

impl<'a, Iv: Interval> Sink<IntervalNode<Iv>> for IntervalSink<'a, Iv> {
    fn consume(&mut self, item: IntervalNode<Iv>) {
        self.output.push(item.ivl)
    }

    fn consume_unchecked(&mut self, item: IntervalNode<Iv>) {
        consume_unchecked(&mut self.output, item.ivl);
    }

    fn consume_ptr(&mut self, src: *const IntervalNode<Iv>) {
        let p = unsafe { &(*src).ivl };
        consume_ptr(&mut self.output, p)
    }
}


struct UpdateMax<Iv: Interval, Tree: TreeBase<IntervalNode<Iv>>> {
    _ph: PhantomData<(Iv, Tree)>
}

impl<Iv: Interval, Tree> EnterItem<IntervalNode<Iv>> for UpdateMax<Iv, Tree>
                                            where Tree: BulkDeleteCommon<IntervalNode<Iv>,
                                                                         UpdateMax<Iv, Tree>> {
    type Tree = Tree;

    #[inline]
    fn enter<F>(tree: &mut Self::Tree, idx: usize, mut f: F)
                                                    where F: FnMut(&mut Self::Tree, usize) {
        f(tree, idx);

        if tree.is_nil(idx) {
            return;
        }

        let item = tree.item_mut_unsafe(idx);
        match (tree.has_left(idx), tree.has_right(idx)) {
            (false, false) => {},
            (false, true) =>
                item.maxb = cmp::max(&item.maxb, &tree.item(righti(idx)).maxb).clone(),
            (true, false) =>
                item.maxb = cmp::max(&item.maxb, &tree.item(lefti(idx)).maxb).clone(),
            (true, true) =>
                item.maxb = cmp::max(&item.maxb,
                                     cmp::max(&tree.item(lefti(idx)).maxb, &tree.item(righti(idx)).maxb))
                                    .clone(),
        }
    }
}

impl<Iv: Interval> BulkDeleteCommon<IntervalNode<Iv>,
                                    UpdateMax<Iv, TreeWrapper<IntervalNode<Iv>>>
                                   > for TreeWrapper<IntervalNode<Iv>> {
//    type Update = UpdateMax;
}



impl<Iv: Interval> IntervalDelete<Iv> for TreeWrapper<IntervalNode<Iv>> {}
impl<Iv: Interval> IntervalDeleteRange<Iv> for TreeWrapper<IntervalNode<Iv>> {}
