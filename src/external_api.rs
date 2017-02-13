use std::mem;
use std::marker::PhantomData;

pub use applied::interval::{Interval, KeyInterval};

pub use self::plain::{TeardownTreeMap, TeardownTreeSet};
pub use self::interval::{IntervalTeardownTreeMap, IntervalTeardownTreeSet};
pub use base::{TeardownTreeRefill, Sink};
pub use base::sink;


mod plain {
    use base::{TeardownTreeRefill, Sink, ItemFilter};
    use applied::plain_tree::{PlTree};
    use super::{SinkAdapter, RefSinkAdapter};

    use std::fmt;
    use std::fmt::{Debug, Display, Formatter};
    use std::ops::Range;

    #[cfg(test)] use base::{TreeRepr, Key};
    #[cfg(test)] use applied::plain_tree::PlNode;


    #[derive(Clone)]
    pub struct TeardownTreeMap<K: Ord+Clone, V> {
        internal: PlTree<K,V>
    }

    impl<K: Ord+Clone, V> TeardownTreeMap<K, V> {
        /// Creates a new `TeardownTreeMap` with the given set of items. The items can be given in
        /// any order. Duplicate keys are supported.
        #[inline]
        pub fn new(items: Vec<(K, V)>) -> TeardownTreeMap<K, V> {
            TeardownTreeMap { internal: PlTree::new(items) }
        }

        /// Creates a new `TeardownTreeMap` with the given set of items. Duplicate keys are supported.
        /// **Note**: the items are assumed to be sorted!
        #[inline]
        pub fn with_sorted(sorted: Vec<(K, V)>) -> TeardownTreeMap<K, V> {
            TeardownTreeMap { internal: PlTree::with_sorted(sorted) }
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
    }

    impl<K: Ord+Clone+Debug, V> Debug for TeardownTreeMap<K, V> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&self.internal, fmt)
        }
    }

    impl<K: Ord+Clone+Debug, V> Display for TeardownTreeMap<K, V> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.internal, fmt)
        }
    }

    impl<K: Ord+Clone+Copy, V: Copy> TeardownTreeRefill for TeardownTreeMap<K, V> {
        #[inline]
        fn refill(&mut self, master: &Self) {
            self.internal.refill(&master.internal)
        }
    }


    #[cfg(test)]
    impl<K: Ord+Clone, V> super::TreeWrapperAccess for TeardownTreeMap<K, V> {
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
            TeardownTreeMap { internal: wrapper }
        }

        fn from_repr(repr: Self::Repr) -> Self {
            Self::from_internal(PlTree::with_repr(repr))
        }
    }


    #[derive(Clone, Debug)]
    pub struct TeardownTreeSet<T: Ord+Clone> {
        map: TeardownTreeMap<T, ()>
    }

    impl<T: Ord+Clone> TeardownTreeSet<T> {
        /// Creates a new `TeardownTreeSet` with the given set of items. The items can be given in any
        /// order. Duplicates are supported.
        #[inline]
        pub fn new(items: Vec<T>) -> TeardownTreeSet<T> {
            let map_items = super::conv_to_tuple_vec(items);
            TeardownTreeSet { map: TeardownTreeMap::new(map_items) }
        }

        /// Creates a new `TeardownTreeSet` with the given set of items. Duplicates are supported.
        /// **Note**: the items are assumed to be sorted!
        #[inline]
        pub fn with_sorted(sorted: Vec<T>) -> TeardownTreeSet<T> {
            let map_items = super::conv_to_tuple_vec(sorted);
            TeardownTreeSet { map: TeardownTreeMap::with_sorted(map_items) }
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
    }

    impl<K: Ord+Clone+Copy> TeardownTreeRefill for TeardownTreeSet<K> {
        #[inline]
        fn refill(&mut self, master: &Self) {
            self.map.refill(&master.map)
        }
    }

    #[cfg(test)]
    impl<K: Key> super::TreeWrapperAccess for TeardownTreeSet<K> {
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
            TeardownTreeSet { map: TeardownTreeMap { internal: wrapper } }
        }

        fn from_repr(repr: Self::Repr) -> Self {
            Self::from_internal(PlTree::with_repr(repr))
        }
    }

    impl<T: Ord+Clone+Debug> Display for TeardownTreeSet<T> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.map, fmt)
        }
    }
}



mod interval {
    use std::fmt;
    use std::fmt::{Debug, Display, Formatter};

    use base::{TeardownTreeRefill, ItemFilter, Sink};
    use super::{SinkAdapter, RefSinkAdapter};

    use applied::AppliedTree;
    use applied::interval::{Interval};
    use applied::interval_tree::{IvTree};

    #[cfg(test)] use base::TreeRepr;
    #[cfg(test)] use applied::interval::IvNode;


    #[derive(Clone)]
    pub struct IntervalTeardownTreeMap<Iv: Interval, V> {
        internal: IvTree<Iv, V>
    }

    impl<Iv: Interval, V> IntervalTeardownTreeMap<Iv, V> {
        /// Creates a new `IntervalTeardownTreeMap` with the given set of intervals. The items can be
        /// given in any order. Duplicates are supported.
        #[inline]
        pub fn new(mut items: Vec<(Iv, V)>) -> IntervalTeardownTreeMap<Iv, V> {
            items.sort_by(|a, b| a.0.cmp(&b.0));
            Self::with_sorted(items)
        }

        /// Creates a new `IntervalTeardownTreeMap` with the given set of intervals. Duplicates are
        /// supported.
        /// **Note**: the items are assumed to be sorted with respect to `Interval::cmp()`!
        #[inline]
        pub fn with_sorted(sorted: Vec<(Iv, V)>) -> IntervalTeardownTreeMap<Iv, V> {
            IntervalTeardownTreeMap { internal: IvTree::with_sorted(sorted) }
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
    }


    #[cfg(test)]
    impl<Iv: Interval, V> super::TreeWrapperAccess for IntervalTeardownTreeMap<Iv, V> {
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
            IntervalTeardownTreeMap { internal: wrapper }
        }

        fn from_repr(repr: Self::Repr) -> Self {
            Self::from_internal(IvTree::with_repr(repr))
        }
    }

    impl<Iv: Interval+Copy, V: Copy> TeardownTreeRefill for IntervalTeardownTreeMap<Iv, V> {
        #[inline]
        fn refill(&mut self, master: &Self) {
            self.internal.refill(&master.internal)
        }
    }


    #[derive(Clone)]
    pub struct IntervalTeardownTreeSet<Iv: Interval> {
        map: IntervalTeardownTreeMap<Iv, ()>
    }

    impl<Iv: Interval> IntervalTeardownTreeSet<Iv> {
        /// Creates a new `IntervalTeardownTreeSet` with the given set of intervals. The items can be
        /// given in any order. Duplicates are supported.
        #[inline]
        pub fn new(items: Vec<Iv>) -> IntervalTeardownTreeSet<Iv> {
            let map_items = super::conv_to_tuple_vec(items);
            IntervalTeardownTreeSet { map: IntervalTeardownTreeMap::new(map_items) }
        }

        /// Creates a new `IntervalTeardownTreeSet` with the given set of intervals. Duplicates are
        /// supported.
        /// **Note**: the items are assumed to be sorted!
        #[inline]
        pub fn with_sorted(sorted: Vec<Iv>) -> IntervalTeardownTreeSet<Iv> {
            let map_items = super::conv_to_tuple_vec(sorted);
            IntervalTeardownTreeSet { map: IntervalTeardownTreeMap::with_sorted(map_items) }
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
    }

    #[cfg(test)]
    impl<Iv: Interval> super::TreeWrapperAccess for IntervalTeardownTreeSet<Iv> {
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
            IntervalTeardownTreeSet { map: IntervalTeardownTreeMap { internal: wrapper } }
        }

        fn from_repr(repr: Self::Repr) -> Self {
            Self::from_internal(IvTree::with_repr(repr))
        }
    }

    impl<Iv: Interval+Copy> TeardownTreeRefill for IntervalTeardownTreeSet<Iv> {
        #[inline] fn refill(&mut self, master: &Self) {
            self.map.refill(&master.map)
        }
    }


    impl<Iv: Interval+Debug, V> Debug for IntervalTeardownTreeMap<Iv, V> where Iv::K: Debug {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&self.internal, fmt)
        }
    }

    impl<Iv: Interval, V> Display for IntervalTeardownTreeMap<Iv, V> where Iv::K: Debug {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.internal, fmt)
        }
    }

    impl<Iv: Interval+Debug> Debug for IntervalTeardownTreeSet<Iv> where Iv::K: Debug {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&self.map, fmt)
        }
    }

    impl<Iv: Interval> Display for IntervalTeardownTreeSet<Iv> where Iv::K: Debug {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.map, fmt)
        }
    }
}

#[inline(always)]
fn conv_to_tuple_vec<K>(items: Vec<K>) -> Vec<(K, ())> {
    unsafe { mem::transmute(items) }
}



struct SinkAdapter<T, S: Sink<T>> {
    sink: S,
    _ph: PhantomData<T>
}

impl<T, S: Sink<T>> SinkAdapter<T, S> {
    #[inline]
    fn new(sink: S) -> Self {
        SinkAdapter { sink: sink, _ph: PhantomData }
    }
}

impl<T, S: Sink<T>> Sink<(T, ())> for SinkAdapter<T, S> {
    #[inline(always)]
    fn consume(&mut self, entry: (T, ())) {
        self.sink.consume(entry.0)
    }
}



struct RefSinkAdapter<'a, T: 'a, S: Sink<&'a T>> {
    sink: S,
    _ph: PhantomData<&'a T>
}

impl<'a, T: 'a, S: Sink<&'a T>> RefSinkAdapter<'a, T, S> {
    #[inline]
    fn new(sink: S) -> Self {
        RefSinkAdapter { sink: sink, _ph: PhantomData }
    }
}

impl<'a, T: 'a, S: Sink<&'a T>> Sink<&'a (T, ())> for RefSinkAdapter<'a, T, S> {
    #[inline]
    fn consume(&mut self, entry: &'a (T, ())) {
        self.sink.consume(&entry.0)
    }
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
