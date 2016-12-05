use std::ptr;
use std::mem;
use std::cmp::{max, Ordering};
use std::fmt;
use std::fmt::{Debug, Formatter};
use delete_range::{DeleteRange, DeleteRangeCache, TraversalDriver};
use drivers::{DriverFromToRef, DriverFromTo};
use unsafe_stack::UnsafeStack;

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
    pub item: T,
}

impl<T: Item> Node<T> {
    fn new(item: T) -> Self {
        Node { item: item }
    }
}


/// A fast way to refill the tree from a master copy; adds the requirement for T to implement Copy.
pub trait TeardownTreeRefill<T: Copy+Item> {
    fn refill(&mut self, master: &TeardownTree<T>);
}


impl<T: Copy+Item> TeardownTreeRefill<T> for TeardownTree<T> {
    fn refill(&mut self, master: &TeardownTree<T>) {
        let len = self.data.len();
        debug_assert!(len == master.data.len());
        unsafe {
            ptr::copy_nonoverlapping(master.data.as_ptr(), self.data.as_mut_ptr(), len);
            ptr::copy_nonoverlapping(master.mask.as_ptr(), self.mask.as_mut_ptr(), len);
        }
        self.size = master.size;
    }
}


//impl<T: Clone+Item> TeardownTreeRefill<T> for TeardownTree<T> {
//    fn refill(&mut self, master: &TeardownTree<T>) {
//            let len = self.data.len();
//            debug_assert!(len == master.data.len());
//            self.drop_items();
//
//            for i in 0..master.size() {
//                if master.mask[i] {
//                    self.place(i, master.data[i].item.clone());
//                }
//            }
//    }
//}


pub struct TeardownTree<T: Item> {
    data: Vec<Node<T>>,
    mask: Vec<bool>,
    size: usize,

    pub traversal_stack: UnsafeStack<usize>,
    pub delete_range_cache: Option<DeleteRangeCache<T>>,
}

impl<T: Item> Clone for TeardownTree<T> {
    fn clone(&self) -> Self {
        debug_assert!(self.traversal_stack.is_empty());

        TeardownTree {
            data: self.data.clone(),
            mask: self.mask.clone(),
            size: self.size,
            traversal_stack: UnsafeStack::new(self.traversal_stack.capacity()),
            delete_range_cache: self.delete_range_cache.clone()
        }
    }
}


impl<T: Item> TeardownTree<T> {
    /// Constructs a new TeardownTree<T>
    /// Note: the argument must be sorted!
    pub fn new(mut sorted: Vec<T>) -> TeardownTree<T> {
        let size = sorted.len();

        let capacity = size;

        let mut data = Vec::with_capacity(capacity);
        unsafe { data.set_len(capacity); }

        let mask: Vec<bool> = vec![true; capacity];
        let height = Self::build(&mut sorted, 0, &mut data);
        unsafe { sorted.set_len(0); }
        let cache = DeleteRangeCache::new(height);
        TeardownTree { data: data, mask: mask, size: size, delete_range_cache: Some(cache), traversal_stack: UnsafeStack::new(capacity) }
    }

    fn calc_height(nodes: &Vec<Option<Node<T>>>, idx: usize) -> usize {
        if idx < nodes.len() && nodes[idx].is_some() {
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
    fn build(sorted: &mut [T], idx: usize, data: &mut [Node<T>]) -> usize {
        match sorted.len() {
            0 => 0,
            n => {
                let mid = Self::build_select_root(n);
                let (lefti, righti) = (Self::lefti(idx), Self::righti(idx));
                let lh = Self::build(&mut sorted[..mid], lefti, data);
                let rh = Self::build(&mut sorted[mid+1..], righti, data);

                let p = unsafe {
                    sorted.as_ptr().offset(mid as isize)
                };
                let item = unsafe { ptr::read(p) };

                let garbage = mem::replace(&mut data[idx], Node { item: item} );
                mem::forget(garbage);

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
                let ok = cache.slots_min.is_empty() && cache.slots_max.is_empty() && self.traversal_stack.is_empty();
                self.delete_range_cache = Some(cache);
                ok
            });
        }
        self.size -= output.len();
    }



    #[inline(always)]
    fn item_mut(&mut self, idx: usize) -> &mut T {
        &mut self.node_mut(idx).item
    }

    #[inline(always)]
    fn item(&self, idx: usize) -> &T {
        &self.node(idx).item
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
        self.index_of(search).map(|idx| self.item(idx))
    }

    fn index_of(&self, search: &T::Key) -> Option<usize> {
        if self.data.is_empty() {
            return None;
        }

        let mut idx = 0;
        let mut key =
            if self.mask[idx] {
                self.item(idx).key()
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

            if self.mask[idx] {
                key = self.item(idx).key();
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
                self.take(idx)
            },

            (true, false)  => {
                let left_max = self.delete_max(Self::lefti(idx));
                mem::replace(self.item_mut(idx), left_max)
            },

            (false, true)  => {
                let right_min = self.delete_min(Self::righti(idx));
                mem::replace(self.item_mut(idx), right_min)
            },

            (true, true)   => {
                let left_max = self.delete_max(Self::lefti(idx));
                mem::replace(self.item_mut(idx), left_max)
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
            mem::replace(self.item_mut(idx), left_max)
        } else {
            self.take(idx)
        }
    }

    #[inline]
    fn delete_min(&mut self, mut idx: usize) -> T {
        while self.has_left(idx) {
            idx = Self::lefti(idx);
        }

        if self.has_right(idx) {
            let right_min = self.delete_min(Self::righti(idx));
            mem::replace(self.item_mut(idx), right_min)
        } else {
            self.take(idx)
        }
    }


//    #[inline]
//    fn level_from(level: usize) -> usize {
//        (1 << level) - 1
//    }
//
//    #[inline]
//    fn level_of(idx: usize) -> usize {
//        mem::size_of::<usize>()*8 - ((idx+1).leading_zeros() as usize) - 1
//    }
//
//    #[inline]
//    fn row_start(idx: usize) -> usize {
//        Self::level_from(Self::level_of(idx))
//    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.size
    }
}

impl<T: Item> Drop for TeardownTree<T> {
    fn drop(&mut self) {
        self.drop_items();
        unsafe {
            self.data.set_len(0)
        }
    }
}

pub trait TeardownTreeInternal<T: Item> {
    fn with_nodes(nodes: Vec<Option<Node<T>>>) -> TeardownTree<T>;
//    fn into_node_vec(self) -> Vec<Option<Node<T>>>;

    fn node(&self, idx: usize) -> &Node<T>;
    fn node_mut(&mut self, idx: usize) -> &mut Node<T>;

    fn node_opt(&self, idx: usize) -> Option<&Node<T>>;

    fn parenti(idx: usize) -> usize;
    fn lefti(idx: usize) -> usize;
    fn righti(idx: usize) -> usize;

    fn parent_opt(&self, idx: usize) -> Option<&Node<T>>;
    fn left_opt(&self, idx: usize) -> Option<&Node<T>>;
    fn right_opt(&self, idx: usize) -> Option<&Node<T>>;

    fn parent(&self, idx: usize) -> &Node<T>;
    fn left(&self, idx: usize) -> &Node<T>;
    fn right(&self, idx: usize) -> &Node<T>;

    fn has_left(&self, idx: usize) -> bool;
    fn has_right(&self, idx: usize) -> bool;
    fn is_null(&self, idx: usize) -> bool;

    fn drop_items(&mut self);

    fn traverse_preorder<A, F>(&mut self, root: usize, a: &mut A, f: F) where F: FnMut(&mut Self, &mut A, usize);

    fn take(&mut self, idx: usize) -> T;
    fn place(&mut self, idx: usize, item: T);
    unsafe fn move_to(&mut self, idx: usize, dst: *mut T);
}

impl<T: Item> TeardownTreeInternal<T> for TeardownTree<T> {
    /// Constructs a new TeardownTree<T> based on raw nodes vec.
    fn with_nodes(mut nodes: Vec<Option<Node<T>>>) -> TeardownTree<T> {
        let size = nodes.iter().filter(|x| x.is_some()).count();
        let height = Self::calc_height(&nodes, 0);
        let capacity = nodes.len();

        let mut mask = vec![false; capacity];
        let mut data = Vec::with_capacity(capacity);
        unsafe {
            data.set_len(capacity);
        }

        for i in 0..capacity {
            if let Some(node) = nodes[i].take() {
                mask[i] = true;
                let garbage = mem::replace(&mut data[i], node );
                mem::forget(garbage);
            }
        }

        let cache = DeleteRangeCache::new(height);
        TeardownTree { data: data, mask: mask, size: size, delete_range_cache: Some(cache), traversal_stack: UnsafeStack::new(capacity) }
    }


//    fn into_node_vec(self) -> Vec<Option<Node<T>>> {
//        self.data
//            .into_iter()
//            .zip(self.mask.into_iter())
//            .map(|(node, flag)| if flag {
//                    Some(node)
//                } else {
//                    None
//                })
//            .collect::<Vec<Option<Node<T>>>>()
//    }

    #[inline(always)]
    fn node(&self, idx: usize) -> &Node<T> {
        &self.data[idx]
    }

    #[inline(always)]
    fn node_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[idx]
    }

    #[inline(always)]
    fn node_opt(&self, idx: usize) -> Option<&Node<T>> {
        if self.is_null(idx) { None } else { Some(self.node(idx)) }
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
    fn parent_opt(&self, idx: usize) -> Option<&Node<T>> {
        if idx == 0 {
            None
        } else {
            Some(&self.data[Self::parenti(idx)])
        }
    }

    #[inline(always)]
    fn left_opt(&self, idx: usize) -> Option<&Node<T>> {
        let lefti = Self::lefti(idx);
        if self.is_null(lefti) {
            None
        } else {
            Some(&self.data[lefti])
        }
    }

    #[inline(always)]
    fn right_opt(&self, idx: usize) -> Option<&Node<T>> {
        let righti = Self::righti(idx);
        if self.is_null(righti) {
            None
        } else {
            Some(&self.data[righti])
        }
    }


    #[inline(always)]
    fn parent(&self, idx: usize) -> &Node<T> {
        let parenti = Self::parenti(idx);
        debug_assert!(idx > 0 && !self.is_null(idx));
        &self.data[parenti]
    }

    #[inline(always)]
    fn left(&self, idx: usize) -> &Node<T> {
        let lefti = Self::lefti(idx);
        debug_assert!(!self.is_null(lefti));
        &self.data[lefti]
    }

    #[inline(always)]
    fn right(&self, idx: usize) -> &Node<T> {
        let righti = Self::righti(idx);
        debug_assert!(!self.is_null(righti));
        &self.data[righti]
    }


    #[inline(always)]
    fn has_left(&self, idx: usize) -> bool {
        !self.is_null(Self::lefti(idx))
    }

    #[inline(always)]
    fn has_right(&self, idx: usize) -> bool {
        !self.is_null(Self::righti(idx))
    }

    #[inline(always)]
    fn is_null(&self, idx: usize) -> bool {
        idx >= self.data.len() || !self.mask[idx]
    }

    #[inline]
    fn traverse_preorder<A, F>(&mut self, root: usize, a: &mut A, mut f: F) where F: FnMut(&mut Self, &mut A, usize) {
        debug_assert!(self.traversal_stack.is_empty());

        if self.is_null(root) {
            return;
        }

        let mut next = root;

        loop {
            next = {
                f(self, a, next);

                match (self.has_left(next), self.has_right(next)) {
                    (false, false) => {
                        if self.traversal_stack.is_empty() {
                            break;
                        }

                        self.traversal_stack.pop()
                    },

                    (true, false)  => {
                        Self::lefti(next)
                    },

                    (false, true)  => {
                        Self::righti(next)
                    },

                    (true, true)   => {
                        debug_assert!(self.traversal_stack.size() < self.traversal_stack.capacity());

                        self.traversal_stack.push(Self::righti(next));
                        Self::lefti(next)
                    },
                }
            };
        }
    }


    fn drop_items(&mut self) {
        if self.size*2 <= self.data.len() {
            self.traverse_preorder(0, &mut 0, |this: &mut Self, _, idx| {
                unsafe {
                    let p = this.data.as_mut_ptr();
                    ptr::drop_in_place(p.offset(idx as isize));
                }

                this.mask[idx] = false;
            })
        } else {
            let p = self.data.as_mut_ptr();
            for i in 0..self.size() {
                if self.mask[i] {
                    unsafe {
                        ptr::drop_in_place(p.offset(i as isize));
                    }

                    self.mask[i] = false;
                }
            }
        }
    }

    #[inline(always)]
    fn take(&mut self, idx: usize) -> T {
        debug_assert!(!self.is_null(idx), "idx={}, mask[idx]={}", idx, self.mask[idx]);
        let p: *const Node<T> = unsafe {
            self.data.as_ptr().offset(idx as isize)
        };
        self.mask[idx] = false;
        unsafe { ptr::read(&(*p).item) }
    }

    #[inline(always)]
    unsafe fn move_to(&mut self, idx: usize, dst: *mut T) {
        debug_assert!(!self.is_null(idx), "idx={}, mask[idx]={}", idx, self.mask[idx]);
        self.mask[idx] = false;
        let p: *mut Node<T> = self.data.as_mut_ptr().offset(idx as isize);
        let x = ptr::read(&(*p).item);
        ptr::write(dst, x);
    }

    #[inline(always)]
    fn place(&mut self, idx: usize, item: T) {
        if self.mask[idx] {
            self.data[idx].item = item;
        } else {
            self.mask[idx] = true;
            unsafe {
                let p: *mut Node<T> = self.data.as_mut_ptr().offset(idx as isize);
                ptr::write(p, Node::new(item));
            };
        }
    }
}



impl<K: Ord+Debug, T: Item<Key=K>> Debug for TeardownTree<T> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let mut nz: Vec<_> = self.mask.iter().enumerate()
            .rev()
            .skip_while(|&(_, flag)| !flag)
            .map(|(i, &flag)| match (self.node(i), flag) {
                (_, false) => String::from("0"),
                (ref node, true) => format!("{:?}", node.item.key())
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
            writeln!(fmt, "{:?}", self.item(idx).key())?;

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
