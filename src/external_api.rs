use applied::interval::{Interval, IntervalNode};
use base::TreeWrapper;

pub use self::plain::PlainTeardownTree;
pub use self::interval::IntervalTeardownTree;
pub use base::TeardownTreeRefill;



pub trait PlainTreeWrapperAccess<T: Ord> {
    fn internal(&mut self) -> &mut TreeWrapper<T>;
}


pub trait IntervalTreeWrapperAccess<Iv: Interval> {
    fn internal(&mut self) -> &mut TreeWrapper<IntervalNode<Iv>>;
}


mod plain {
    use base::{TreeWrapper, TreeBase, TeardownTreeRefill};
    use applied::plain_tree::PlainDeleteInternal;

    use std::fmt;
    use std::fmt::{Debug, Display, Formatter};


    #[derive(Clone)]
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

    impl<T: Ord + Debug> Debug for PlainTeardownTree<T> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&self.internal, fmt)
        }
    }

    impl<T: Ord + Debug> Display for PlainTeardownTree<T> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.internal, fmt)
        }
    }

    impl<T: Ord+Copy> TeardownTreeRefill<T> for PlainTeardownTree<T> {
        fn refill(&mut self, master: &Self) {
            self.internal.refill(&master.internal)
        }
    }


    impl<T: Ord> super::PlainTreeWrapperAccess<T> for PlainTeardownTree<T> {
        fn internal(&mut self) -> &mut TreeWrapper<T> {
            &mut self.internal
        }
    }
}



mod interval {
    use base::{TreeWrapper, TreeBase, parenti};

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
            {
                let internal = &mut tree.internal;

                // initialize maxb values
                for i in (1..internal.size()).rev() {
                    let parent = internal.item_mut_unsafe(parenti(i));
                    let item = internal.item(i);
                    if item.maxb > parent.maxb {
                        parent.maxb = item.maxb.clone()
                    }
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
        pub fn delete_intersecting(&mut self, search: &Iv, output: &mut Vec<Iv>) {
            self.internal.delete_intersecting(search, output)
        }

        pub fn size(&self) -> usize {
            self.internal.size()
        }
    }


    impl<Iv: Interval> super::IntervalTreeWrapperAccess<Iv> for IntervalTeardownTree<Iv> {
        fn internal(&mut self) -> &mut TreeWrapper<IntervalNode<Iv>> {
            &mut self.internal
        }
    }
}
