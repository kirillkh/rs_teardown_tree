use std::mem;

pub use applied::interval::{Interval, KeyInterval};

pub use self::plain::{TeardownMap, TeardownSet};
pub use self::interval::{IntervalTeardownMap, IntervalTeardownSet};
pub use base::{Refill, Sink};
pub use base::sink;


pub mod iter {
    pub use super::plain::{SetIter, MapIter, SetIntoIter, MapIntoIter};
    pub use super::interval::{IntervalSetIter, IntervalMapIter, IntervalSetIntoIter, IntervalMapIntoIter};
}



mod plain {
    use base::{Refill, Sink, ItemFilter};
    use applied::plain_tree::{PlTree, PlNode};
    use super::sink::{SinkAdapter, RefSinkAdapter};

    use std::fmt;
    use std::fmt::{Debug, Display, Formatter};
    use std::ops::Range;

    #[cfg(test)] use base::{TreeRepr, Key};


    #[derive(Clone)]
    pub struct TeardownMap<K: Ord+Clone, V> {
        internal: PlTree<K,V>
    }

    impl<K: Ord+Clone, V> TeardownMap<K, V> {
        /// Creates a new `TeardownMap` with the given set of items. The items can be given in
        /// any order. Duplicate keys are supported.
        #[inline]
        pub fn new(items: Vec<(K, V)>) -> TeardownMap<K, V> {
            TeardownMap { internal: PlTree::new(items) }
        }

        /// Creates a new `TeardownMap` with the given set of items. Duplicate keys are supported.
        /// **Note**: the items are assumed to be sorted!
        #[inline]
        pub fn with_sorted(sorted: Vec<(K, V)>) -> TeardownMap<K, V> {
            TeardownMap { internal: PlTree::with_sorted(sorted) }
        }

        /// Finds the item with the given key and returns it (or None).
        #[inline]
        pub fn find<'a, Q>(&'a self, query: &'a Q) -> Option<&'a V>
            where Q: PartialOrd<K>
        {
            self.internal.find(query)
        }

        /// Returns true if the map contains the given key.
        #[inline]
        pub fn contains_key<Q>(&self, query: &Q) -> bool
            where Q: PartialOrd<K>
        {
            self.internal.contains(query)
        }

        /// Executes a range query.
        #[inline]
        pub fn query_range<'a, Q, S>(&'a self, range: Range<Q>, sink: S)
            where Q: PartialOrd<K>,
                  S: Sink<&'a (K, V)>
        {
            self.internal.query_range(range, sink)
        }

        /// Deletes the item with the given key from the tree and returns it (or None).
        #[inline]
        pub fn delete<Q>(&mut self, query: &Q) -> Option<V>
            where Q: PartialOrd<K>
        {
            self.internal.delete(query)
        }

        /// Deletes all items inside `range` from the tree and feeds them into `sink`.
        /// The items are returned in order.
        #[inline]
        pub fn delete_range<Q, S>(&mut self, range: Range<Q>, sink: S)
            where Q: PartialOrd<K>, S: Sink<(K, V)>
        {
            self.internal.delete_range(range, sink)
        }

        /// Deletes all items inside `range` that match `filter` from the tree and feeds them into
        /// `sink`. The items are returned in order.
        #[inline]
        pub fn filter_range<Q, Flt, S>(&mut self, range: Range<Q>, filter: Flt, sink: S)
            where Q: PartialOrd<K>, Flt: ItemFilter<K>, S: Sink<(K, V)>
        {
            self.internal.filter_range(range, filter, sink)
        }

        /// Deletes all items inside `range` from the tree and feeds them into `sink`.
        #[inline]
        pub fn delete_range_ref<Q, S>(&mut self, range: Range<&Q>, sink: S)
            where Q: PartialOrd<K>, S: Sink<(K, V)>
        {
            self.internal.delete_range_ref(range, sink)
        }

        /// Deletes all items inside `range` that match `filter` from the tree and feeds them into
        /// `sink`. The items are returned in order.
        #[inline]
        pub fn filter_range_ref<Q, Flt, S>(&mut self, range: Range<&Q>, filter: Flt, sink: S)
            where Q: PartialOrd<K>,
                  Flt: ItemFilter<K>,
                  S: Sink<(K, V)>
        {
            self.internal.filter_range_ref(range, filter, sink)
        }

        /// Returns the number of items in this tree.
        #[inline] pub fn size(&self) -> usize { self.internal.size() }

        #[inline] pub fn is_empty(&self) -> bool { self.size() == 0 }

        /// Removes all items from the tree (the items are dropped, but the internal storage is not).
        #[inline] pub fn clear(&mut self) { self.internal.clear(); }

        /// Creates an iterator into the map.
        #[inline]
        pub fn iter<'a>(&'a self) -> MapIter<'a, K, V> {
            MapIter::new(self.internal.iter())
        }
    }

    impl<K: Ord+Clone+Debug, V> Debug for TeardownMap<K, V> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&self.internal, fmt)
        }
    }

    impl<K: Ord+Clone+Debug, V> Display for TeardownMap<K, V> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.internal, fmt)
        }
    }

    impl<K: Ord+Clone+Copy, V: Copy> Refill for TeardownMap<K, V> {
        #[inline]
        fn refill(&mut self, master: &Self) {
            self.internal.refill(&master.internal)
        }
    }


    #[cfg(test)]
    impl<K: Ord+Clone, V> super::TreeWrapperAccess for TeardownMap<K, V> {
        type Repr = TreeRepr<PlNode<K,V>>;
        type Wrapper = PlTree<K,V>;

        fn internal(&self) -> &PlTree<K,V> {
            &self.internal
        }

        fn internal_mut(&mut self) -> &mut Self::Wrapper {
            &mut self.internal
        }

        fn into_internal(self) -> PlTree<K, V> {
            self.internal
        }

        fn from_internal(wrapper: PlTree<K, V>) -> Self {
            TeardownMap { internal: wrapper }
        }

        fn from_repr(repr: Self::Repr) -> Self {
            Self::from_internal(PlTree::with_repr(repr))
        }
    }


    #[derive(Clone, Debug)]
    pub struct TeardownSet<T: Ord+Clone> {
        map: TeardownMap<T, ()>
    }

    impl<T: Ord+Clone> TeardownSet<T> {
        /// Creates a new `TeardownSet` with the given set of items. The items can be given in any
        /// order. Duplicates are supported.
        #[inline]
        pub fn new(items: Vec<T>) -> TeardownSet<T> {
            let map_items = super::conv_to_tuple_vec(items);
            TeardownSet { map: TeardownMap::new(map_items) }
        }

        /// Creates a new `TeardownSet` with the given set of items. Duplicates are supported.
        /// **Note**: the items are assumed to be sorted!
        #[inline]
        pub fn with_sorted(sorted: Vec<T>) -> TeardownSet<T> {
            let map_items = super::conv_to_tuple_vec(sorted);
            TeardownSet { map: TeardownMap::with_sorted(map_items) }
        }

        /// Returns true if the set contains the given item.
        #[inline]
        pub fn contains<Q: PartialOrd<T>>(&self, query: &Q) -> bool {
            self.map.contains_key(query)
        }

        /// Executes a range query and feeds references to the matching items into `sink`.
        #[inline]
        pub fn query_range<'a, Q, S>(&'a self, query: Range<Q>, sink: S)
            where Q: PartialOrd<T>,
                  S: Sink<&'a T>
        {
            self.map.query_range(query, RefSinkAdapter::new(sink))
        }

        /// Deletes the item with the given key from the tree and returns it (or None).
        #[inline]
        pub fn delete<Q: PartialOrd<T>>(&mut self, query: &Q) -> bool {
            self.map.delete(query).is_some()
        }

        /// Deletes all items inside `range` from the tree and feeds them into `sink`.
        /// The items are returned in order.
        #[inline]
        pub fn delete_range<Q, S>(&mut self, query: Range<Q>, sink: S)
            where Q: PartialOrd<T>, S: Sink<T>
        {
            let map_sink = SinkAdapter::new(sink);
            self.map.delete_range(query, map_sink)
        }

        /// Deletes all items inside `range` that match `filter` from the tree and feeds them into
        /// `sink`. The items are returned in order.
        #[inline]
        pub fn filter_range<Q, Flt, S>(&mut self, range: Range<Q>, filter: Flt, sink: S)
            where Q: PartialOrd<T>,
                  Flt: ItemFilter<T>,
                  S: Sink<T>
        {
            let map_sink = SinkAdapter::new(sink);
            self.map.filter_range(range, filter, map_sink)
        }

        /// Deletes all items inside `range` from the tree and feeds them into `sink`.
        #[inline]
        pub fn delete_range_ref<Q, S>(&mut self, range: Range<&Q>, sink: S)
            where Q: PartialOrd<T>, S: Sink<T>
        {
            let map_sink = SinkAdapter::new(sink);
            self.map.delete_range_ref(range, map_sink)
        }

        /// Deletes all items inside `range` that match `filter` from the tree and feeds them into
        /// `sink`. The items are returned in order.
        #[inline]
        pub fn filter_range_ref<Q, Flt, S>(&mut self, range: Range<&Q>, filter: Flt, sink: S)
            where Q: PartialOrd<T>,
                  Flt: ItemFilter<T>,
                  S: Sink<T>
        {
            let map_sink = SinkAdapter::new(sink);
            self.map.filter_range_ref(range, filter, map_sink)
        }

        /// Returns the number of items in this tree.
        #[inline] pub fn size(&self) -> usize { self.map.size() }

        #[inline] pub fn is_empty(&self) -> bool { self.map.is_empty() }

        /// Removes all items from the tree (the items are dropped, but the internal storage is not).
        #[inline] pub fn clear(&mut self) { self.map.clear(); }

        /// Creates an iterator into the set.
        #[inline] pub fn iter<'a>(&'a self) -> SetIter<'a, T> {
            SetIter::new(self.map.internal.iter())
        }
    }

    impl<K: Ord+Clone+Copy> Refill for TeardownSet<K> {
        #[inline]
        fn refill(&mut self, master: &Self) {
            self.map.refill(&master.map)
        }
    }

    #[cfg(test)]
    impl<K: Key> super::TreeWrapperAccess for TeardownSet<K> {
        type Repr = TreeRepr<PlNode<K, ()>>;
        type Wrapper = PlTree<K, ()>;

        fn internal(&self) -> &PlTree<K,()> {
            &self.map.internal
        }

        fn internal_mut(&mut self) -> &mut PlTree<K,()> {
            &mut self.map.internal
        }

        fn into_internal(self) -> PlTree<K, ()> {
            self.map.internal
        }

        fn from_internal(wrapper: PlTree<K, ()>) -> Self {
            TeardownSet { map: TeardownMap { internal: wrapper } }
        }

        fn from_repr(repr: Self::Repr) -> Self {
            Self::from_internal(PlTree::with_repr(repr))
        }
    }

    impl<T: Ord+Clone+Debug> Display for TeardownSet<T> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.map, fmt)
        }
    }


    #[derive(new)]
    pub struct MapIter<'a, K: Ord+Clone+'a, V: 'a> {
        inner: ::base::Iter<'a, PlNode<K, V>>
    }

    impl<'a, K: Ord+Clone+'a, V: 'a> Iterator for MapIter<'a, K, V> {
        type Item = &'a (K, V);

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next().map(|entry| entry.as_tuple())
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<'a, K: Ord+Clone+'a, V: 'a> ExactSizeIterator for MapIter<'a, K, V> {}


    #[derive(new)]
    pub struct SetIter<'a, T: Ord+Clone+'a> {
        inner: ::base::Iter<'a, PlNode<T, ()>>
    }

    impl<'a, T: Ord+Clone+'a> Iterator for SetIter<'a, T> {
        type Item = &'a T;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next().map(|entry| entry.key())
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<'a, T: Ord+Clone+'a> ExactSizeIterator for SetIter<'a, T> {}


    impl<K: Ord+Clone, V> IntoIterator for TeardownMap<K, V> {
        type Item = (K, V);
        type IntoIter = MapIntoIter<K, V>;

        fn into_iter(self) -> Self::IntoIter {
            MapIntoIter::new(::base::IntoIter::new(self.internal.into_repr()))
        }
    }

    // this is just a wrapper for ::base::IntoIter<Node> to avoid leaking the Node type
    #[derive(new)]
    pub struct MapIntoIter<K: Ord+Clone, V> {
        inner: ::base::IntoIter<PlNode<K, V>>
    }

    impl<K: Ord+Clone, V> Iterator for MapIntoIter<K, V> {
        type Item = (K, V);
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<K: Ord+Clone, V> ExactSizeIterator for MapIntoIter<K, V> {}



    impl<T: Ord+Clone> IntoIterator for TeardownSet<T> {
        type Item = T;
        type IntoIter = SetIntoIter<T>;

        fn into_iter(self) -> Self::IntoIter {
            SetIntoIter::new(::base::IntoIter::new(self.map.internal.into_repr()))
        }
    }

    // this is just a wrapper for ::base::IntoIter<Node> to avoid leaking the Node type
    #[derive(new)]
    pub struct SetIntoIter<T: Ord+Clone> {
        inner: ::base::IntoIter<PlNode<T, ()>>
    }

    impl<T: Ord+Clone> Iterator for SetIntoIter<T> {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next().map(|(item, _)| item)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<T: Ord+Clone> ExactSizeIterator for SetIntoIter<T> {}
}



mod interval {
    use std::fmt;
    use std::fmt::{Debug, Display, Formatter};

    use base::{Refill, ItemFilter, Sink};
    use super::sink::{SinkAdapter, RefSinkAdapter};

    use applied::AppliedTree;
    use applied::interval::{Interval, IvNode};
    use applied::interval_tree::{IvTree};

    #[cfg(test)] use base::TreeRepr;


    #[derive(Clone)]
    pub struct IntervalTeardownMap<Iv: Interval, V> {
        internal: IvTree<Iv, V>
    }

    impl<Iv: Interval, V> IntervalTeardownMap<Iv, V> {
        /// Creates a new `IntervalTeardownMap` with the given set of intervals. The items can be
        /// given in any order. Duplicates are supported.
        #[inline]
        pub fn new(mut items: Vec<(Iv, V)>) -> IntervalTeardownMap<Iv, V> {
            items.sort_by(|a, b| a.0.cmp(&b.0));
            Self::with_sorted(items)
        }

        /// Creates a new `IntervalTeardownMap` with the given set of intervals. Duplicates are
        /// supported.
        /// **Note**: the items are assumed to be sorted with respect to `Interval::cmp()`!
        #[inline]
        pub fn with_sorted(sorted: Vec<(Iv, V)>) -> IntervalTeardownMap<Iv, V> {
            IntervalTeardownMap { internal: IvTree::with_sorted(sorted) }
        }

        /// Finds the item with the given key and returns it (or None).
        #[inline]
        pub fn find<'a, Q>(&'a self, query: &'a Q) -> Option<&'a V>
            where Q: Interval<K=Iv::K> + PartialOrd<Iv> // TODO: requiring PartialOrd is redundant, we could get rid of it using a wrapper
        {
            self.internal.find(query)
        }

        /// Returns true if the map contains the given key.
        #[inline]
        pub fn contains_key<Q>(&self, query: &Q) -> bool
            where Q: Interval<K=Iv::K> + PartialOrd<Iv> // TODO: requiring PartialOrd is redundant, we could get rid of it using a wrapper
        {
            self.internal.contains(query)
        }

        /// Executes an overlap query.
        #[inline]
        pub fn query_overlap<'a, Q, S>(&'a self, query: &Q, sink: S)
            where Q: Interval<K=Iv::K>,
                  S: Sink<&'a (Iv, V)>
        {
            self.internal.query_overlap(0, query, sink)
        }

        /// Deletes the item with the given key from the tree and returns it (or None).
        #[inline]
        pub fn delete<Q>(&mut self, query: &Q) -> Option<V>
            where Q: PartialEq<Iv> + PartialOrd<Iv>
        {
            self.internal.delete(query)
        }

        /// Deletes all intervals that overlap with `query` from the tree and feeds them into `sink`.
        /// The items are returned in order.
        #[inline]
        pub fn delete_overlap<Q, S>(&mut self, query: &Q, sink: S)
            where Q: Interval<K=Iv::K>, S: Sink<(Iv, V)>
        {
            self.internal.delete_overlap(query, sink)
        }

        /// Deletes all intervals that overlap with `query` and match the filter from the tree and
        /// feeds them into `sink`. The items are returned in order.
        #[inline]
        pub fn filter_overlap<Q, Flt, S>(&mut self, query: &Q, f: Flt, sink: S)
            where Q: Interval<K=Iv::K>,
                  Flt: ItemFilter<Iv>,
                  S: Sink<(Iv, V)>
        {
            self.internal.filter_overlap(query, sink, f)
        }

        /// Returns the number of items in this tree.
        #[inline]
        pub fn size(&self) -> usize {
            self.internal.size()
        }

        #[inline] pub fn is_empty(&self) -> bool { self.size() == 0 }

        /// Removes all items from the tree (the items are dropped, but the internal storage is not).
        #[inline] pub fn clear(&mut self) { self.internal.clear(); }

        /// Creates an iterator into the map.
        #[inline]
        pub fn iter<'a>(&'a self) -> IntervalMapIter<'a, Iv, V> {
            IntervalMapIter::new(self.internal.iter())
        }
    }


    #[cfg(test)]
    impl<Iv: Interval, V> super::TreeWrapperAccess for IntervalTeardownMap<Iv, V> {
        type Repr = TreeRepr<IvNode<Iv,V>>;
        type Wrapper = IvTree<Iv,V>;

        fn internal(&self) -> &IvTree<Iv, V> {
            &self.internal
        }

        fn internal_mut(&mut self) -> &mut IvTree<Iv, V> {
            &mut self.internal
        }

        fn into_internal(self) -> IvTree<Iv, V> {
            self.internal
        }

        fn from_internal(wrapper: IvTree<Iv, V>) -> Self {
            IntervalTeardownMap { internal: wrapper }
        }

        fn from_repr(repr: Self::Repr) -> Self {
            Self::from_internal(IvTree::with_repr(repr))
        }
    }

    impl<Iv: Interval+Copy, V: Copy> Refill for IntervalTeardownMap<Iv, V> {
        #[inline]
        fn refill(&mut self, master: &Self) {
            self.internal.refill(&master.internal)
        }
    }


    #[derive(Clone)]
    pub struct IntervalTeardownSet<Iv: Interval> {
        map: IntervalTeardownMap<Iv, ()>
    }

    impl<Iv: Interval> IntervalTeardownSet<Iv> {
        /// Creates a new `IntervalTeardownSet` with the given set of intervals. The items can be
        /// given in any order. Duplicates are supported.
        #[inline]
        pub fn new(items: Vec<Iv>) -> IntervalTeardownSet<Iv> {
            let map_items = super::conv_to_tuple_vec(items);
            IntervalTeardownSet { map: IntervalTeardownMap::new(map_items) }
        }

        /// Creates a new `IntervalTeardownSet` with the given set of intervals. Duplicates are
        /// supported.
        /// **Note**: the items are assumed to be sorted!
        #[inline]
        pub fn with_sorted(sorted: Vec<Iv>) -> IntervalTeardownSet<Iv> {
            let map_items = super::conv_to_tuple_vec(sorted);
            IntervalTeardownSet { map: IntervalTeardownMap::with_sorted(map_items) }
        }

        /// Returns true if the set contains the given item.
        #[inline]
        pub fn contains<Q>(&self, query: &Q) -> bool
            where Q: Interval<K=Iv::K> + PartialOrd<Iv> // TODO: requiring PartialOrd is redundant, we could get rid of it using a wrapper
        {
            self.map.contains_key(query)
        }

        /// Executes an overlap query.
        #[inline]
        pub fn query_overlap<'a, Q, S>(&'a self, query: &Q, sink: S)
            where Q: Interval<K=Iv::K>,
                  S: Sink<&'a Iv>
        {

            self.map.query_overlap(query, RefSinkAdapter::new(sink))
        }

        /// Deletes the given interval from the tree and returns true (or false if it was not found).
        #[inline]
        pub fn delete<Q>(&mut self, query: &Q) -> bool
            where Q: Interval<K=Iv::K> + PartialOrd<Iv> // TODO: requiring PartialOrd is redundant, we could get rid of it using a wrapper
        {
            self.map.delete(query).is_some()
        }

        /// Deletes all intervals that overlap with `query` from the tree and and feeds them into
        /// `sink`. The items are returned in order.
        #[inline]
        pub fn delete_overlap<Q, S>(&mut self, query: &Q, sink: S)
            where Q: Interval<K=Iv::K>, S: Sink<Iv>
        {
            let map_sink = SinkAdapter::new(sink);
            self.map.delete_overlap(query, map_sink)
        }

        /// Deletes all intervals that overlap with `query` and match the filter from the tree and
        /// feeds them into `sink`. The items are returned in order.
        #[inline]
        pub fn filter_overlap<Q, Flt, S>(&mut self, query: &Q, f: Flt, sink: S)
            where Q: Interval<K=Iv::K>,
                  Flt: ItemFilter<Iv>,
                  S: Sink<Iv>
        {
            let map_sink = SinkAdapter::new(sink);
            self.map.filter_overlap(query, f, map_sink)
        }


        /// Returns the number of items in this tree.
        #[inline] pub fn size(&self) -> usize { self.map.size() }

        #[inline] pub fn is_empty(&self) -> bool { self.map.is_empty() }

        /// Removes all items from the tree (the items are dropped, but the internal storage is not).
        #[inline] pub fn clear(&mut self) { self.map.clear(); }

        /// Creates an iterator into the set.
        #[inline]
        pub fn iter<'a>(&'a self) -> IntervalSetIter<'a, Iv> {
            IntervalSetIter::new(self.map.internal.iter())
        }
    }

    #[cfg(test)]
    impl<Iv: Interval> super::TreeWrapperAccess for IntervalTeardownSet<Iv> {
        type Repr = TreeRepr<IvNode<Iv, ()>>;
        type Wrapper = IvTree<Iv, ()>;

        fn internal(&self) -> &IvTree<Iv, ()> {
            &self.map.internal
        }

        fn internal_mut(&mut self) -> &mut IvTree<Iv, ()> {
            &mut self.map.internal
        }

        fn into_internal(self) -> IvTree<Iv, ()> {
            self.map.internal
        }

        fn from_internal(wrapper: IvTree<Iv, ()>) -> Self {
            IntervalTeardownSet { map: IntervalTeardownMap { internal: wrapper } }
        }

        fn from_repr(repr: Self::Repr) -> Self {
            Self::from_internal(IvTree::with_repr(repr))
        }
    }

    impl<Iv: Interval+Copy> Refill for IntervalTeardownSet<Iv> {
        #[inline] fn refill(&mut self, master: &Self) {
            self.map.refill(&master.map)
        }
    }


    impl<Iv: Interval+Debug, V> Debug for IntervalTeardownMap<Iv, V> where Iv::K: Debug {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&self.internal, fmt)
        }
    }

    impl<Iv: Interval, V> Display for IntervalTeardownMap<Iv, V> where Iv::K: Debug {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.internal, fmt)
        }
    }

    impl<Iv: Interval+Debug> Debug for IntervalTeardownSet<Iv> where Iv::K: Debug {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&self.map, fmt)
        }
    }

    impl<Iv: Interval> Display for IntervalTeardownSet<Iv> where Iv::K: Debug {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.map, fmt)
        }
    }


    #[derive(new)]
    pub struct IntervalMapIter<'a, Iv: Interval+'a, V: 'a> {
        inner: ::base::Iter<'a, IvNode<Iv, V>>
    }

    impl<'a, Iv: Interval+'a, V: 'a> Iterator for IntervalMapIter<'a, Iv, V> {
        type Item = &'a (Iv, V);

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next().map(|entry| entry.as_tuple())
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<'a, Iv: Interval+'a, V: 'a> ExactSizeIterator for IntervalMapIter<'a, Iv, V> {}


    #[derive(new)]
    pub struct IntervalSetIter<'a, Iv: Interval+'a> {
        inner: ::base::Iter<'a, IvNode<Iv, ()>>
    }

    impl<'a, Iv: Interval+'a> Iterator for IntervalSetIter<'a, Iv> {
        type Item = &'a Iv;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next().map(|entry| entry.key())
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<'a, Iv: Interval+'a> ExactSizeIterator for IntervalSetIter<'a, Iv> {}


    impl<Iv: Interval, V> IntoIterator for IntervalTeardownMap<Iv, V> {
        type Item = (Iv, V);
        type IntoIter = IntervalMapIntoIter<Iv, V>;

        fn into_iter(self) -> Self::IntoIter {
            IntervalMapIntoIter::new(::base::IntoIter::new(self.internal.into_repr()))
        }
    }

    // this is just a wrapper for ::base::IntoIter<Node> to avoid leaking the Node type
    #[derive(new)]
    pub struct IntervalMapIntoIter<Iv: Interval, V> {
        inner: ::base::IntoIter<IvNode<Iv, V>>
    }

    impl<Iv: Interval, V> Iterator for IntervalMapIntoIter<Iv, V> {
        type Item = (Iv, V);
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<Iv: Interval, V> ExactSizeIterator for IntervalMapIntoIter<Iv, V> {}



    impl<Iv: Interval> IntoIterator for IntervalTeardownSet<Iv> {
        type Item = Iv;
        type IntoIter = IntervalSetIntoIter<Iv>;

        fn into_iter(self) -> Self::IntoIter {
            IntervalSetIntoIter::new(::base::IntoIter::new(self.map.internal.into_repr()))
        }
    }

    // this is just a wrapper for ::base::IntoIter<Node> to avoid leaking the Node type
    #[derive(new)]
    pub struct IntervalSetIntoIter<Iv: Interval> {
        inner: ::base::IntoIter<IvNode<Iv, ()>>
    }

    impl<Iv: Interval> Iterator for IntervalSetIntoIter<Iv> {
        type Item = Iv;
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next().map(|(item, _)| item)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<Iv: Interval> ExactSizeIterator for IntervalSetIntoIter<Iv> {}
}

#[inline(always)]
fn conv_to_tuple_vec<K>(items: Vec<K>) -> Vec<(K, ())> {
    unsafe { mem::transmute(items) }
}


#[cfg(test)]
pub trait TreeWrapperAccess {
    type Repr;
    type Wrapper;

    fn internal(&self) -> &Self::Wrapper;
    fn internal_mut(&mut self) -> &mut Self::Wrapper;
    fn into_internal(self) -> Self::Wrapper;
    fn from_internal(wrapper: Self::Wrapper) -> Self;
    fn from_repr(repr: Self::Repr) -> Self;
}
