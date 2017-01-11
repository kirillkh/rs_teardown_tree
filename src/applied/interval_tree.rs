use applied::interval::{Interval, AugValue};
use base::{TreeWrapper, TreeBase, Node, BulkDeleteCommon, ItemVisitor, lefti, righti, parenti, Sink};
use base::drivers::{consume_ptr, consume_unchecked};
use std::{mem, cmp};
use std::marker::PhantomData;

type IvTree<Iv, V> = TreeWrapper<Iv, AugValue<Iv, V>>;

pub trait IntervalTreeInternal<Iv: Interval, V> {
    #[inline] fn delete(&mut self, search: &Iv) -> Option<V>;
    #[inline] fn delete_intersecting(&mut self, search: &Iv, output: &mut Vec<(Iv, V)>);
}


impl<Iv: Interval, V> IntervalTreeInternal<Iv, V> for IvTree<Iv, V> {
    /// Deletes the item with the given key from the tree and returns it (or None).
    // TODO: accepting IntervalNode is super ugly, temporary solution only
    #[inline]
    fn delete(&mut self, search: &Iv) -> Option<V> {
        self.index_of(search).map(|idx| {
            let (ivl, val) = self.delete_idx(idx);
            self.update_ancestors_after_delete(idx, &ivl.b());
            val
        })
    }

    #[inline]
    fn delete_intersecting(&mut self, search: &Iv, output: &mut Vec<(Iv, V)>) {
        if self.size() != 0 {
            UpdateMax::visit(self, 0, move |this, _|
                this.delete_intersecting_ivl_rec(search, 0, false, &mut self::IntervalSink { output: output })
            )
        }
    }
}


trait IntervalDelete<Iv: Interval, V>: TreeBase<Iv, AugValue<Iv, V>> {
    #[inline]
    fn update_maxb(&mut self, idx: usize) {
        let node = self.node_mut_unsafe(idx);

        let left_self_maxb =
            if self.has_left(idx) {
                cmp::max(&self.left(idx).val.maxb, node.key.b())
            } else {
                node.key.b()
            };
        node.val.maxb =
            if self.has_right(idx) {
                cmp::max(&self.right(idx).val.maxb, left_self_maxb)
            } else {
                left_self_maxb
            }.clone();
    }

    #[inline]
    fn update_ancestors_after_delete(&mut self, mut idx: usize, removed_b: &Iv::K) {
        while idx != 0 {
            idx = parenti(idx);
            if removed_b == &self.val(idx).maxb {
                self.update_maxb(idx);
            } else {
                break;
            }
        }
    }

    #[inline]
    fn delete_idx(&mut self, idx: usize) -> (Iv, V) {
        debug_assert!(!self.is_nil(idx));

        let node = self.node_mut_unsafe(idx);

        let (repl_ivl, repl_val): (Iv, V) = match (self.has_left(idx), self.has_right(idx)) {
            (false, false) => {
                let item = self.take(idx);
                return (item.key, item.val.val)
            },

            (true, false)  => {
                let (ivl, val, left_maxb) = self.delete_max(lefti(idx));
                node.val.maxb = left_maxb;
                (ivl, val)
            },

            (false, true)  => {
//                let (removed, right_maxb) = self.delete_min(righti(idx));
//                item.maxb = right_maxb;
                let (ivl, val) = self.delete_min(righti(idx));
                if &node.val.maxb == ivl.b() {
                    self.update_maxb(idx)
                } else { // maxb remains the same
                    debug_assert!(ivl.b() < &node.val.maxb);
                }
                (ivl, val)
            },

            (true, true)   => {
                let (ivl, val, left_maxb) = self.delete_max(lefti(idx));
                if &node.val.maxb == ivl.b() {
                    node.val.maxb = cmp::max(left_maxb, self.right(idx).val.maxb.clone());
                } else { // maxb remains the same
                    debug_assert!(ivl.b() < &node.val.maxb);
                }
                (ivl, val)
            },
        };

        let ivl = mem::replace(&mut node.key, repl_ivl);
        let key = mem::replace(&mut node.val.val, repl_val);
        (ivl, key)
    }


    /// returns the removed max-item of this subtree and the old maxb (before removal)
    #[inline]
    // we attempt to reduce the number of memory accesses as much as possible; might be overengineered
    fn delete_max(&mut self, idx: usize) -> (Iv, V, Iv::K) {
        let max_idx = self.find_max(idx);

        let (ivl, val, mut old_maxb, mut new_maxb) = if self.has_left(max_idx) {
            let node = self.node_mut_unsafe(max_idx);
            let (left_max, left_maxv, left_maxb) = self.delete_max(lefti(max_idx));
            let ivl = mem::replace(&mut node.key, left_max);
            let val = mem::replace(&mut node.val.val, left_maxv);

            let old_maxb = mem::replace(&mut node.val.maxb, left_maxb.clone());
            (ivl, val, old_maxb, Some(left_maxb))
        } else {
            let Node { key: ivl, val: AugValue{maxb:old_maxb, val} } = self.take(max_idx);
            (ivl, val, old_maxb, None)
        };

        // update ancestors
        let mut upd_idx = max_idx;
        while upd_idx != idx {
            upd_idx = parenti(upd_idx);

            let node = self.node_mut_unsafe(upd_idx);
            old_maxb = node.val.maxb.clone();
            if &node.val.maxb == ivl.b() {
                let mb = {
                    let self_left_maxb =
                        if self.has_left(upd_idx) {
                            cmp::max(&self.left(upd_idx).val.maxb, &node.val.maxb)
                        } else {
                            &node.val.maxb
                        };

                    new_maxb.map_or(self_left_maxb.clone(),
                                    |mb| cmp::max(mb, self_left_maxb.clone()))
                };
                node.val.maxb = mb.clone();
                new_maxb = Some(mb);
            } else {
                new_maxb = Some(old_maxb.clone());
            }
        }

        (ivl, val, old_maxb)
    }

    // TODO: check whether optimizations similar to delete_max() are worth it
    #[inline]
    fn delete_min(&mut self, idx: usize) -> (Iv, V) {
        let min_idx = self.find_min(idx);

        let (ivl, val) = if self.has_right(min_idx) {
            let (right_min, right_minv) = self.delete_min(righti(min_idx));
            let node = self.node_mut_unsafe(min_idx);

            if self.has_right(min_idx) {
                let right_maxb = &self.right(min_idx).val.maxb;
                node.val.maxb = cmp::max(right_maxb, right_min.b()).clone();
            } else {
                node.val.maxb = right_min.b().clone();
            }

            let ivl = mem::replace(&mut node.key, right_min);
            let val = mem::replace(&mut node.val.val, right_minv);
            (ivl, val)
        } else {
            let item = self.take(min_idx);
            (item.key, item.val.val)
        };

        // update ancestors
        let mut upd_idx = min_idx;
        while upd_idx != idx {
            upd_idx = parenti(upd_idx);
            self.update_maxb(upd_idx);
        }

        (ivl, val)
    }
}


trait IntervalDeleteRange<Iv: Interval, V>: BulkDeleteCommon<Iv, AugValue<Iv, V>, UpdateMax<Iv, V, Self>> + IntervalDelete<Iv, V> {
    fn delete_intersecting_ivl_rec<S: Sink<Iv, AugValue<Iv, V>>>(&mut self, search: &Iv, idx: usize, min_included: bool, sink: &mut S) {
        let node = self.node_unsafe(idx);
        let k: &Iv = &node.key;

        if node.val.maxb() <= search.a() && k.a() != search.a() {
            // whole subtree outside the range
            if self.slots_min().has_open() {
                self.fill_slots_min(idx);
            }
            if self.slots_max().has_open() && !self.is_nil(idx) {
                self.fill_slots_max(idx);
            }
        } else if search.b() <= k.a() && k.a() != search.a() {
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
            let consumed = if search.intersects(k)
                { Some(self.take(idx)) }
            else
                { None };

            // left subtree
            let mut removed = consumed.is_some();
            if removed {
                if min_included {
                    self.consume_subtree(lefti(idx), sink)
                } else {
                    removed = self.descend_delete_intersecting_ivl_left(search, idx, true, false, sink);
                }

                sink.consume_unchecked(consumed.unwrap());
            } else {
                removed = self.descend_delete_intersecting_ivl_left(search, idx, false, min_included, sink);
                if !removed && self.slots_min().has_open() {
                    removed = true;
                    self.fill_slot_min(idx);
                }
            }

            // right subtree
            let right_min_included = min_included || search.a() <= k.a();
            if right_min_included {
                let right_max_included = node.val.maxb() < search.b();
                if right_max_included {
                    self.consume_subtree(righti(idx), sink);
                } else {
                    removed = self.descend_delete_intersecting_ivl_right(search, idx, removed, true, sink);
                }
            } else {
                removed = self.descend_delete_intersecting_ivl_right(search, idx, removed, false, sink);
            }

            if !removed && self.slots_max().has_open() {
                removed = true;
                self.fill_slot_max(idx);
            }

            // fill the remaining open slots_max from the left subtree
            if removed && self.slots_max().has_open() {
                self.descend_fill_max_left(idx, true);
            }
        }
    }


    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_intersecting_ivl_left<S: Sink<Iv, AugValue<Iv, V>>>(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, sink: &mut S) -> bool {
        self.descend_left(idx, with_slot,
                          |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included, sink))
    }

    /// Returns true if the item is removed after recursive call, false otherwise.
    #[inline(always)]
    fn descend_delete_intersecting_ivl_right<S: Sink<Iv, AugValue<Iv, V>>>(&mut self, search: &Iv, idx: usize, with_slot: bool, min_included: bool, sink: &mut S) -> bool {
        self.descend_right(idx, with_slot,
                           |this: &mut Self, child_idx| this.delete_intersecting_ivl_rec(search, child_idx, min_included, sink))
    }
}


struct IntervalSink<'a, Iv: Interval+'a, V: 'a> {
    output: &'a mut Vec<(Iv, V)>
}

impl<'a, Iv: Interval, V> Sink<Iv, AugValue<Iv, V>> for IntervalSink<'a, Iv, V> {
    fn consume(&mut self, item: Node<Iv, AugValue<Iv, V>>) {
        self.output.push((item.key, item.val.val));
    }

    fn consume_unchecked(&mut self, node: Node<Iv, AugValue<Iv, V>>) {
        // TODO
        consume_unchecked(&mut self.output, Node::new(node.key, node.val.val));
    }

    fn consume_ptr(&mut self, src: *const Node<Iv, AugValue<Iv, V>>) {
        // TODO
        unimplemented!();
//        consume_ptr(&mut self.output, src)
    }
}


struct UpdateMax<Iv: Interval, V, Tree: TreeBase<Iv, AugValue<Iv, V>>> {
    _ph: PhantomData<(Iv, V, Tree)>
}

impl<Iv: Interval, V, Tree> ItemVisitor<Iv, AugValue<Iv, V>> for UpdateMax<Iv, V, Tree>
                               where Tree: BulkDeleteCommon<Iv, AugValue<Iv, V>, UpdateMax<Iv, V, Tree>> {
    type Tree = Tree;

    #[inline]
    fn visit<F>(tree: &mut Self::Tree, idx: usize, mut f: F)
                                                    where F: FnMut(&mut Self::Tree, usize) {
        f(tree, idx);

        if tree.is_nil(idx) {
            return;
        }

        let val = &mut tree.node_mut_unsafe(idx).val;
        match (tree.has_left(idx), tree.has_right(idx)) {
            (false, false) => {},
            (false, true) =>
                val.maxb = cmp::max(&val.maxb, &tree.node(righti(idx)).val.maxb).clone(),
            (true, false) =>
                val.maxb = cmp::max(&val.maxb, &tree.node(lefti(idx)).val.maxb).clone(),
            (true, true) =>
                val.maxb = cmp::max(&val.maxb,
                                     cmp::max(&tree.node(lefti(idx)).val.maxb, &tree.node(righti(idx)).val.maxb))
                                    .clone(),
        }
    }
}

impl<Iv: Interval, V> BulkDeleteCommon<Iv, AugValue<Iv, V>,
                                    UpdateMax<Iv, V, IvTree<Iv, V>>> for IvTree<Iv, V> {
//    type Update = UpdateMax;
}


impl<Iv: Interval, V> IntervalDelete<Iv, V> for IvTree<Iv, V> {}
impl<Iv: Interval, V> IntervalDeleteRange<Iv, V> for IvTree<Iv, V> {}
