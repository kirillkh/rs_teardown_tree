use std::mem;

use applied::interval::{Interval, AugValue};
use base::TreeWrapper;

pub use self::plain::{TeardownTreeMap, TeardownTreeSet};
pub use self::interval::{IntervalTeardownTreeMap, IntervalTeardownTreeSet};
pub use base::TeardownTreeRefill;



pub trait PlainTreeWrapperAccess<K: Ord, V> {
    fn internal(&mut self) -> &mut TreeWrapper<K, V>;
}


pub trait IntervalTreeWrapperAccess<Iv: Interval, V> {
    fn internal(&mut self) -> &mut TreeWrapper<Iv, AugValue<Iv, V>>;
}


mod plain {
    use base::{TreeWrapper, TreeBase, TeardownTreeRefill};
    use applied::plain_tree::PlainDeleteInternal;

    use std::fmt;
    use std::fmt::{Debug, Display, Formatter};
    use std::ops::Range;
    use std::mem;


    #[derive(Clone)]
    pub struct TeardownTreeMap<K: Ord, V> {
        internal: TreeWrapper<K, V>
    }

    impl<K: Ord, V> TeardownTreeMap<K, V> {
        pub fn new(mut items: Vec<(K, V)>) -> TeardownTreeMap<K, V> {
            items.sort_by(|a, b| a.0.cmp(&b.0));
            Self::with_sorted(items)
        }

        /// Creates a new TeardownTree with the given set of items.
        /// **Note**: the items are assumed to be sorted!
        pub fn with_sorted(sorted: Vec<(K, V)>) -> TeardownTreeMap<K, V> {
            TeardownTreeMap { internal: TreeWrapper::with_sorted(sorted) }
        }

        /// Deletes the item with the given key from the tree and returns it (or None).
        pub fn delete(&mut self, search: &K) -> Option<V> {
            self.internal.delete(search)
        }

        /// Deletes all items inside the half-open `range` from the tree and stores them in the output
        /// Vec. The items are returned in order.
        pub fn delete_range(&mut self, range: Range<K>, output: &mut Vec<(K, V)>) {
            self.internal.delete_range(range, output)
        }

        /// Deletes all items inside the half-open `range` from the tree and stores them in the output Vec.
        pub fn delete_range_ref(&mut self, range: Range<&K>, output: &mut Vec<(K, V)>) {
            self.internal.delete_range_ref(range, output)
        }

        /// Returns the number of items in this tree.
        pub fn size(&self) -> usize { self.internal.size() }

        pub fn is_empty(&self) -> bool { self.size() == 0 }

        /// Removes all items from the tree (the items are dropped, but the internal storage is not).
        pub fn clear(&mut self) { self.internal.clear(); }
    }

    impl<K: Ord + Debug, V> Debug for TeardownTreeMap<K, V> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&self.internal, fmt)
        }
    }

    impl<K: Ord + Debug, V> Display for TeardownTreeMap<K, V> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.internal, fmt)
        }
    }

    impl<K: Ord+Copy, V> TeardownTreeRefill<K> for TeardownTreeMap<K, V> {
        fn refill(&mut self, master: &Self) {
            self.internal.refill(&master.internal)
        }
    }


    impl<K: Ord, V> super::PlainTreeWrapperAccess<K, V> for TeardownTreeMap<K, V> {
        fn internal(&mut self) -> &mut TreeWrapper<K, V> {
            &mut self.internal
        }
    }


    #[derive(Clone, Debug)]
    pub struct TeardownTreeSet<T: Ord> {
        map: TeardownTreeMap<T, ()>
    }

    impl<T: Ord> TeardownTreeSet<T> {
        pub fn new(items: Vec<T>) -> TeardownTreeSet<T> {
            let map_items = super::conv_to_tuple_vec(items);
            TeardownTreeSet { map: TeardownTreeMap::new(map_items) }
        }

        /// Creates a new TeardownTree with the given set of items.
        /// **Note**: the items are assumed to be sorted!
        pub fn with_sorted(sorted: Vec<T>) -> TeardownTreeSet<T> {
            let map_items = super::conv_to_tuple_vec(sorted);
            TeardownTreeSet { map: TeardownTreeMap::with_sorted(map_items) }
        }

        /// Deletes the item with the given key from the tree and returns it (or None).
        pub fn delete(&mut self, search: &T) -> bool {
            self.map.delete(search).is_some()
        }

        /// Deletes all items inside the half-open `range` from the tree and stores them in the output
        /// Vec. The items are returned in order.
        pub fn delete_range(&mut self, range: Range<T>, output: &mut Vec<T>) {
            let map_output = unsafe { mem::transmute(output) };
            self.map.delete_range(range, map_output)
        }

        /// Deletes all items inside the half-open `range` from the tree and stores them in the output Vec.
        pub fn delete_range_ref(&mut self, range: Range<&T>, output: &mut Vec<T>) {
            let map_output = unsafe { mem::transmute(output) };
            self.map.delete_range_ref(range, map_output)
        }

        /// Returns the number of items in this tree.
        pub fn size(&self) -> usize { self.map.size() }

        pub fn is_empty(&self) -> bool { self.map.is_empty() }

        /// Removes all items from the tree (the items are dropped, but the internal storage is not).
        pub fn clear(&mut self) { self.map.clear(); }
    }

    impl<K: Ord+Copy> TeardownTreeRefill<K> for TeardownTreeSet<K> {
        fn refill(&mut self, master: &Self) {
            self.map.refill(&master.map)
        }
    }

    impl<K: Ord> super::PlainTreeWrapperAccess<K, ()> for TeardownTreeSet<K> {
        fn internal(&mut self) -> &mut TreeWrapper<K, ()> {
            self.map.internal()
        }
    }
}



mod interval {
    use std::mem;

    use base::{TreeWrapper, TreeBase, TeardownTreeRefill, parenti};

    use applied::interval::{Interval, AugValue};
    use applied::interval_tree::IntervalTreeInternal;

    #[derive(Clone)]
    pub struct IntervalTeardownTreeMap<Iv: Interval, V> {
        internal: TreeWrapper<Iv, AugValue<Iv, V>>
    }

    impl<Iv: Interval, V> IntervalTeardownTreeMap<Iv, V> {
        pub fn new(mut items: Vec<(Iv, V)>) -> IntervalTeardownTreeMap<Iv, V> {
            items.sort_by(|a, b| a.0.cmp(&b.0));
            Self::with_sorted(items)
        }

        /// Creates a new `IntervalTeardownTree` with the given set of intervals.
        /// **Note**: the items are assumed to be sorted with respect to `Interval::cmp()`!
        pub fn with_sorted(sorted: Vec<(Iv, V)>) -> IntervalTeardownTreeMap<Iv, V> {
            let items = sorted.into_iter()
                              .map(|(ivl, val)| {
                                  let val = AugValue::new(ivl.b().clone(), val);
                                  (ivl, val)
                              })
                              .collect();
            let mut tree = IntervalTeardownTreeMap { internal: TreeWrapper::with_sorted(items) };
            {
                let internal = &mut tree.internal;

                // initialize maxb values
                for i in (1..internal.size()).rev() {
                    let parent = internal.node_mut_unsafe(parenti(i));
                    let node = internal.node(i);
                    if node.val.maxb > parent.val.maxb {
                        parent.val.maxb = node.val.maxb.clone()
                    }
                }
            }

            tree
        }

        /// Deletes the item with the given key from the tree and returns it (or None).
        #[inline]
        pub fn delete(&mut self, search: &Iv) -> Option<V> {
            self.internal.delete(search)
        }

        /// Deletes all intervals intersecting with the `search` interval from the tree and stores them
        /// in the output Vec. The items are returned in order.
        #[inline]
        pub fn delete_intersecting(&mut self, search: &Iv, output: &mut Vec<(Iv, V)>) {
            self.internal.delete_intersecting(search, output)
        }

        /// Returns the number of items in this tree.
        pub fn size(&self) -> usize {
            self.internal.size()
        }

        pub fn is_empty(&self) -> bool { self.size() == 0 }

        /// Removes all items from the tree (the items are dropped, but the internal storage is not).
        pub fn clear(&mut self) { self.internal.clear(); }
    }


    impl<Iv: Interval, V> super::IntervalTreeWrapperAccess<Iv, V> for IntervalTeardownTreeMap<Iv, V> {
        fn internal(&mut self) -> &mut TreeWrapper<Iv, AugValue<Iv, V>> {
            &mut self.internal
        }
    }

    impl<Iv: Interval+Copy, V> TeardownTreeRefill<Iv> for IntervalTeardownTreeMap<Iv, V> {
        fn refill(&mut self, master: &Self) {
            self.internal.refill(&master.internal)
        }
    }



    pub struct IntervalTeardownTreeSet<Iv: Interval> {
        map: IntervalTeardownTreeMap<Iv, ()>
    }

    impl<Iv: Interval> IntervalTeardownTreeSet<Iv> {
        pub fn new(items: Vec<Iv>) -> IntervalTeardownTreeSet<Iv> {
            let map_items = super::conv_to_tuple_vec(items);
            IntervalTeardownTreeSet { map: IntervalTeardownTreeMap::new(map_items) }
        }

        /// Creates a new TeardownTree with the given set of items.
        /// **Note**: the items are assumed to be sorted!
        pub fn with_sorted(sorted: Vec<Iv>) -> IntervalTeardownTreeSet<Iv> {
            let map_items = super::conv_to_tuple_vec(sorted);
            IntervalTeardownTreeSet { map: IntervalTeardownTreeMap::with_sorted(map_items) }
        }

        /// Deletes the item with the given key from the tree and returns it (or None).
        pub fn delete(&mut self, search: &Iv) -> bool {
            self.map.delete(search).is_some()
        }

        /// Deletes all items inside the half-open `range` from the tree and stores them in the output
        /// Vec. The items are returned in order.
        pub fn delete_intersecting(&mut self, search: &Iv, output: &mut Vec<Iv>) {
            let map_output = unsafe { mem::transmute(output) };
            self.map.delete_intersecting(search, map_output)
        }

        /// Returns the number of items in this tree.
        pub fn size(&self) -> usize { self.map.size() }

        pub fn is_empty(&self) -> bool { self.map.is_empty() }

        /// Removes all items from the tree (the items are dropped, but the internal storage is not).
        pub fn clear(&mut self) { self.map.clear(); }
    }

    impl<Iv: Interval+Copy> TeardownTreeRefill<Iv> for IntervalTeardownTreeSet<Iv> {
        fn refill(&mut self, master: &Self) {
            self.map.refill(&master.map)
        }
    }
}

#[inline(always)]
fn conv_to_tuple_vec<K>(items: Vec<K>) -> Vec<(K, ())> {
    unsafe { mem::transmute(items) }
}
