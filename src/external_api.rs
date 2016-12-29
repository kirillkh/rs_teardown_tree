use base::TreeWrapper;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};


pub use self::plain::PlainTeardownTree;
pub use self::interval::IntervalTeardownTree;
pub use base::TeardownTreeRefill;



trait TreeWrapperAccess<T: Ord>: Deref<Target=TreeWrapper<T>>+DerefMut<Target=TreeWrapper<T>> {}



impl<T: Ord+Copy> TeardownTreeRefill<T> for TreeWrapperAccess<T, Target = TreeWrapper<T>> {
    fn refill(&mut self, master: &Self) {
        self.deref_mut().refill(&master.deref())
    }
}


impl<T: Ord + Debug> Debug for TreeWrapperAccess<T, Target = TreeWrapper<T>> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Debug::fmt(self.deref(), fmt)
    }
}

impl<T: Ord + Debug> Display for TreeWrapperAccess<T, Target = TreeWrapper<T>> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Display::fmt(self.deref(), fmt)
    }
}



mod plain {
    use base::{TreeBase, TreeWrapper};
    use std::ops::{Deref, DerefMut};

    use applied::plain_tree::{PlainDeleteInternal};

    #[derive(Debug, Clone)]
    pub struct PlainTeardownTree<T: Ord> {
        internal: TreeWrapper<T>
    }

    impl<T: Ord> PlainTeardownTree<T> {
        pub fn new(sorted: Vec<T>) -> PlainTeardownTree<T> {
            PlainTeardownTree { internal: TreeWrapper::new(sorted) }
        }

        /// Deletes the item with the given key from the tree and returns it (or None).
        pub fn delete(&mut self, search: &T) -> Option<T> {
            self.internal.delete(search)
        }

        /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output
        /// Vec. The items are returned in order.
        pub fn delete_range(&mut self, from: T, to: T, output: &mut Vec<T>) {
            self.internal.delete_range(from, to, output)
        }

        /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output Vec.
        pub fn delete_range_ref(&mut self, from: &T, to: &T, output: &mut Vec<T>) {
            self.internal.delete_range_ref(from, to, output)
        }

        pub fn size(&self) -> usize { self.internal.size() }

        pub fn clear(&mut self) { self.internal.clear(); }
    }

    impl<T: Ord> Deref for PlainTeardownTree<T> {
        type Target = TreeWrapper<T>;

        fn deref(&self) -> &Self::Target {
            &self.internal
        }
    }

    impl<T: Ord> DerefMut for PlainTeardownTree<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.internal
        }
    }

    impl<T: Ord> super::TreeWrapperAccess<T> for PlainTeardownTree<T> {}
}



mod interval {
    use base::{TreeBase, TreeWrapper, parenti};
    use std::ops::{Deref, DerefMut};

    use applied::interval::{Interval, IntervalNode};
    use applied::interval_tree::IntervalTreeInternal;

    #[derive(Clone)]
    pub struct IntervalTeardownTree<Iv: Interval> {
        internal: TreeWrapper<IntervalNode<Iv>>
    }

    impl<Iv: Interval> IntervalTeardownTree<Iv> {
        pub fn new(sorted: Vec<Iv>) -> IntervalTeardownTree<Iv> {
            let items = sorted.into_iter()
                              .map(|ivl| IntervalNode{ maxb: ivl.b().clone(), ivl: ivl })
                              .collect();
            let mut tree = IntervalTeardownTree { internal: TreeWrapper::new(items) };

            // initialize maxb values
            for i in (1..tree.size()).rev() {
                let parent = tree.item_mut_unsafe(parenti(i));
                let item = tree.item(i);
                if item.maxb > parent.maxb {
                    parent.maxb = item.maxb.clone()
                }
            }

            tree
        }

        /// Deletes the item with the given key from the tree and returns it (or None).
        // TODO: accepting IntervalNode is super ugly, temporary solution only
        #[inline]
        pub fn delete(&mut self, search: &IntervalNode<Iv>) -> Option<Iv> {
            self.internal.delete(search)
        }

        #[inline]
        pub fn delete_intersecting(&mut self, search: &Iv, idx: usize, output: &mut Vec<Iv>) {
            self.internal.delete_intersecting(search, idx, output)
        }

        pub fn size(&self) -> usize {
            self.internal.size()
        }
    }

    impl<Iv: Interval> Deref for IntervalTeardownTree<Iv> {
        type Target = TreeWrapper<IntervalNode<Iv>>;

        fn deref(&self) -> &Self::Target {
            &self.internal
        }
    }

    impl<Iv: Interval> DerefMut for IntervalTeardownTree<Iv> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.internal
        }
    }

    impl<Iv: Interval> super::TreeWrapperAccess<IntervalNode<Iv>> for IntervalTeardownTree<Iv> {}
}
