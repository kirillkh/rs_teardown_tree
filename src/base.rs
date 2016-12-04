use std::ptr;
use std::mem;
use std::cmp::{max, Ordering};
use std::fmt;
use std::fmt::{Debug, Formatter};
use delete_range::{DeleteRange, DeleteRangeCache, TraversalDriver};
use drivers::{DriverFromToRef, DriverFromTo};

pub trait Item: Sized+Clone {
    type Key: Ord;

    #[inline(always)]
    fn key(&self) -> &Self::Key;
}


impl Item for usize {
    type Key = usize;

    #[inline(always)]
    fn key(&self) -> &Self::Key {
        self
    }
}


#[derive(Debug, Clone)]
pub struct Node<T: Item> {
    pub item: Option<T>,
}


/// A fast way to refill the tree from a master copy; adds the requirement for T to implement Copy.
pub trait TeardownTreeRefill<T: Copy+Item> {
    fn refill(&mut self, master: &TeardownTree<T>);
}


impl<T: Copy+Item> TeardownTreeRefill<T> for TeardownTree<T> {
    fn refill(&mut self, master: &TeardownTree<T>) {
        let len = self.data.len();
        debug_assert!(len == master.data.len());
        self.data.truncate(0);
        unsafe {
            ptr::copy_nonoverlapping(master.data.as_ptr(), self.data.as_mut_ptr(), len);
            self.data.set_len(len);
        }
        self.size = master.size;
    }
}



#[derive(Clone)]
pub struct TeardownTree<T: Item> {
    data: Vec<Node<T>>,
    size: usize,

    pub delete_range_cache: Option<DeleteRangeCache<T>>,
}

impl<T: Item> TeardownTree<T> {
    /// Constructs a new TeardownTree<T>
    /// Note: the argument must be sorted!
    pub fn new(sorted: Vec<T>) -> TeardownTree<T> {
        let size = sorted.len();

        let capacity = size;

        let mut data = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            data.push(Node{item: None});
        }

        let mut sorted: Vec<Option<T>> = sorted.into_iter().map(|x| Some(x)).collect();
        let height = Self::build(&mut sorted, 0, &mut data);
        debug_assert!({ // we assert that the tree is nearly complete
            data.iter().take(sorted.len()).filter(|x| x.item.is_none()).count() == 0
        });
        let cache = DeleteRangeCache::new(height);
        TeardownTree { data: data, size: size, delete_range_cache: Some(cache) }
    }

    fn calc_height(nodes: &Vec<Node<T>>, idx: usize) -> usize {
        if idx < nodes.len() && nodes[idx].item.is_some() {
            1 + max(Self::calc_height(nodes, Self::lefti(idx)),
                    Self::calc_height(nodes, Self::righti(idx)))
        } else {
            0
        }
    }

    /// Finds the point to partition n keys for a nearly-complete binary tree
    /// http://stackoverflow.com/a/26896494/3646645
    fn build_select_root(n: usize) -> usize {
        // the highest power of two <= n
        let x = if n.is_power_of_two() { n }
                else { n.next_power_of_two() / 2 };

        if x/2 <= (n-x) + 1 {
            debug_assert!(x >= 1, "x={}, n={}", x, n);
            x - 1
        } else {
            n - x / 2
        }
    }

    /// Returns the height of the tree.
    fn build(sorted: &mut [Option<T>], idx: usize, data: &mut [Node<T>]) -> usize {
        match sorted.len() {
            0 => 0,
            n => {
                let mid = Self::build_select_root(n);
                let (lefti, righti) = (Self::lefti(idx), Self::righti(idx));
                let lh = Self::build(&mut sorted[..mid], lefti, data);
                let rh = Self::build(&mut sorted[mid+1..], righti, data);

                data[idx] = Node { item: sorted[mid].take() };
                debug_assert!(rh <= lh);
                1 + lh
            }
        }
    }

    /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output Vec.
    pub fn delete_range(&mut self, from: T::Key, to: T::Key, output: &mut Vec<T>) {
        self.delete_with_driver(&mut DriverFromTo::new(from, to), output)
    }

    /// Deletes all items inside the closed [from,to] range from the tree and stores them in the output Vec.
    pub fn delete_range_ref(&mut self, from: &T::Key, to: &T::Key, output: &mut Vec<T>) {
        self.delete_with_driver(&mut DriverFromToRef::new(from, to), output)
    }


    /// Delete based on driver decisions.
    pub fn delete_with_driver<'a, D: TraversalDriver<T>>(&mut self, drv: &mut D, output: &mut Vec<T>) {
        debug_assert!(output.is_empty());
        output.truncate(0);
        {
            DeleteRange::new(self, output).delete_range(drv);
            debug_assert!({
                let cache: DeleteRangeCache<T> = self.delete_range_cache.take().unwrap();
                let ok = cache.slots_min.is_empty() && cache.slots_max.is_empty() && cache.delete_subtree_stack.is_empty();
                self.delete_range_cache = Some(cache);
                ok
            });
        }
        self.size -= output.len();
    }



    fn item_mut_unwrap(&mut self, idx: usize) -> &mut T {
        self.node_mut(idx).item.as_mut().unwrap()
    }

    fn item_unwrap(&self, idx: usize) -> &T {
        self.node(idx).item.as_ref().unwrap()
    }


    /// Deletes the item with the given key from the tree and returns it (or None).
    pub fn delete(&mut self, search: &T::Key) -> Option<T> {
        self.index_of(search).map(|idx| {
            self.size -= 1;
            self.delete_idx(idx)
        })
    }

    /// Finds the item with the given key and returns it (or None).
    pub fn lookup(&self, search: &T::Key) -> Option<&T> {
        self.index_of(search).map(|idx| self.item_unwrap(idx))
    }

    fn index_of(&self, search: &T::Key) -> Option<usize> {
        if self.data.is_empty() {
            return None;
        }

        let mut idx = 0;
        let mut key =
            if let Some(ref it) = self.node(idx).item {
                it.key()
            } else {
                return None;
            };

        loop {
            let ordering = search.cmp(&key);

            idx = match ordering {
                Ordering::Equal   => return Some(idx),
                Ordering::Less    => Self::lefti(idx),
                Ordering::Greater => Self::righti(idx),
            };

            if idx >= self.data.len() {
                return None;
            }

            if let Some(ref it) = self.node(idx).item {
                key = it.key();
            } else {
                return None;
            }
        }
    }



    #[inline]
    fn delete_idx(&mut self, idx: usize) -> T {
        debug_assert!(!self.is_null(idx));

        match (self.has_left(idx), self.has_right(idx)) {
            (false, false) => {
                let root = self.node_mut(idx);
                root.item.take().unwrap()
            },

            (true, false)  => {
                let left_max = self.delete_max(Self::lefti(idx));
                mem::replace(self.item_mut_unwrap(idx), left_max)
            },

            (false, true)  => {
                let right_min = self.delete_min(Self::righti(idx));
                mem::replace(self.item_mut_unwrap(idx), right_min)
            },

            (true, true)   => {
                let left_max = self.delete_max(Self::lefti(idx));
                mem::replace(self.item_mut_unwrap(idx), left_max)
            },
        }
    }

    #[inline]
    fn delete_max(&mut self, mut idx: usize) -> T {
        while self.has_right(idx) {
            idx = Self::righti(idx);
        }

        if self.has_left(idx) {
            let left_max = self.delete_max(Self::lefti(idx));
            mem::replace(self.item_mut_unwrap(idx), left_max)
        } else {
            let root = self.node_mut(idx);
            root.item.take().unwrap()
        }
    }

    #[inline]
    fn delete_min(&mut self, mut idx: usize) -> T {
        while self.has_left(idx) {
            idx = Self::lefti(idx);
        }

        if self.has_right(idx) {
            let right_min = self.delete_min(Self::righti(idx));
            mem::replace(self.item_mut_unwrap(idx), right_min)
        } else {
            let root = self.node_mut(idx);
            root.item.take().unwrap()
        }
    }


    #[inline]
    fn level_from(level: usize) -> usize {
        (1 << level) - 1
    }

    #[inline]
    fn level_of(idx: usize) -> usize {
        mem::size_of::<usize>()*8 - ((idx+1).leading_zeros() as usize) - 1
    }

    #[inline]
    fn row_start(idx: usize) -> usize {
        Self::level_from(Self::level_of(idx))
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.size
    }
}

pub trait TeardownTreeInternal<T: Item> {
    fn with_nodes(nodes: Vec<Node<T>>) -> TeardownTree<T>;
    fn into_node_vec(self) -> Vec<Node<T>>;

    fn node(&self, idx: usize) -> &Node<T>;
    fn node_mut(&mut self, idx: usize) -> &mut Node<T>;

    fn parenti(idx: usize) -> usize;
    fn lefti(idx: usize) -> usize;
    fn righti(idx: usize) -> usize;

    fn parent(&self, idx: usize) -> &Node<T>;
    fn left(&self, idx: usize) -> Option<&Node<T>>;
    fn right(&self, idx: usize) -> Option<&Node<T>>;

    fn parent_mut(&mut self, idx: usize) -> &mut Node<T>;
    fn left_mut(&mut self, idx: usize) -> &mut Node<T>;
    fn right_mut(&mut self, idx: usize) -> &mut Node<T>;

    fn has_left(&self, idx: usize) -> bool;
    fn has_right(&self, idx: usize) -> bool;
    fn is_null(&self, idx: usize) -> bool;

}

impl<T: Item> TeardownTreeInternal<T> for TeardownTree<T> {
    /// Constructs a new TeardownTree<T> based on raw nodes vec.
    fn with_nodes(nodes: Vec<Node<T>>) -> TeardownTree<T> {
        let size = nodes.iter().filter(|x| x.item.is_some()).count();
        let height = Self::calc_height(&nodes, 0);
        let capacity = Self::row_start(nodes.len())*4 + 3; // allocate enough nodes that righti() is never out of bounds

        let mut data = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            data.push(Node{item: None});
        }

        unsafe {
            ptr::copy_nonoverlapping(nodes.as_ptr(), data.as_mut_ptr(), nodes.len());
        }
        ::std::mem::forget(nodes);

        let cache = DeleteRangeCache::new(height);
        TeardownTree { data: data, size: size, delete_range_cache: Some(cache) }
    }


    fn into_node_vec(self) -> Vec<Node<T>> {
        self.data
    }

    #[inline(always)]
    fn node(&self, idx: usize) -> &Node<T> {
        &self.data[idx]
    }

    #[inline(always)]
    fn node_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[idx]
    }

    #[inline(always)]
    fn parenti(idx: usize) -> usize {
        (idx-1) >> 1
    }

    #[inline(always)]
    fn lefti(idx: usize) -> usize {
        (idx<<1) + 1
    }

    #[inline(always)]
    fn righti(idx: usize) -> usize {
        (idx<<1) + 2
    }


    #[inline(always)]
    fn parent(&self, idx: usize) -> &Node<T> {
        &self.data[Self::parenti(idx)]
    }

    #[inline(always)]
    fn left(&self, idx: usize) -> Option<&Node<T>> {
        let lefti = Self::lefti(idx);
        if lefti < self.data.len() {
            Some(&self.data[lefti])
        } else {
            None
        }
    }

    #[inline(always)]
    fn right(&self, idx: usize) -> Option<&Node<T>> {
        let righti = Self::righti(idx);
        if righti < self.data.len() {
            Some(&self.data[righti])
        } else {
            None
        }
    }


    #[inline]
    fn parent_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[Self::parenti(idx)]
    }

    #[inline]
    fn left_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[Self::lefti(idx)]
    }

    #[inline]
    fn right_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[Self::righti(idx)]
    }



    #[inline(always)]
    fn has_left(&self, idx: usize) -> bool {
        self.left(idx).and_then(|nd| nd.item.as_ref()).is_some()
    }

    #[inline(always)]
    fn has_right(&self, idx: usize) -> bool {
        self.right(idx).and_then(|nd| nd.item.as_ref()).is_some()
    }

    #[inline(always)]
    fn is_null(&self, idx: usize) -> bool {
        idx >= self.data.len() || self.data[idx].item.is_none()
    }
}



impl<K: Ord+Debug, T: Item<Key=K>> Debug for TeardownTree<T> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let mut nz: Vec<_> = self.data.iter()
            .rev()
            .skip_while(|node| node.item.is_none())
            .map(|node| match node.item {
                None => String::from("0"),
                Some(ref x) => format!("{:?}", x.key())
            })
            .collect();
        nz.reverse();

        let _ = write!(fmt, "[");
        let mut sep = "";
        for ref key in nz.iter() {
            let _ = write!(fmt, "{}", sep);
            sep = ", ";
            let _ = write!(fmt, "{}", key);
        }
        let _ = write!(fmt, "]");
        Ok(())
    }
}



impl<K: Ord+Debug, T: Item<Key=K>> fmt::Display for TeardownTree<T> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        writeln!(fmt, "")?;
        let mut ancestors = vec![];
        self.fmt_subtree(fmt, 0, &mut ancestors)
    }
}

impl<K: Ord+Debug, T: Item<Key=K>> TeardownTree<T> {
    fn fmt_branch(&self, fmt: &mut Formatter, ancestors: &Vec<bool>) -> fmt::Result {
        for (i, c) in ancestors.iter().enumerate() {
            if i == ancestors.len() - 1 {
                write!(fmt, "|--")?;
            } else {
                if *c {
                    write!(fmt, "|")?;
                } else {
                    write!(fmt, " ")?;
                }
                write!(fmt, "  ")?;
            }
        }

        Ok(())
    }

    fn fmt_subtree(&self, fmt: &mut Formatter, idx: usize, ancestors: &mut Vec<bool>) -> fmt::Result {
        self.fmt_branch(fmt, ancestors)?;

        if !self.is_null(idx) {
            writeln!(fmt, "{:?}", self.item_unwrap(idx).key())?;

            if idx%2 == 0 && !ancestors.is_empty() {
                *ancestors.last_mut().unwrap() = false;
            }

            if self.has_left(idx) || self.has_right(idx) {
                ancestors.push(true);
                self.fmt_subtree(fmt, Self::lefti(idx), ancestors)?;
                self.fmt_subtree(fmt, Self::righti(idx), ancestors)?;
                ancestors.pop();
            }
        } else {
            writeln!(fmt, "X")?;
        }

        Ok(())
    }
}
