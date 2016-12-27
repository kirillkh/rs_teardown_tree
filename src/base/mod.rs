mod slot_stack;
mod bulk_delete;
mod unsafe_stack;
pub mod drivers;

pub use self::slot_stack::*;
pub use self::bulk_delete::*;
pub use self::unsafe_stack::*;
pub use self::drivers::*;

use std::ptr;
use std::mem;
use std::cmp::{max, Ordering};
use std::fmt;
use std::fmt::{Debug, Formatter};
//use self::{DeleteRangeCache, DeleteRangeInternal};
//use self::{TraversalDriver, RangeRefDriver, RangeDriver, Sink};

//pub trait Item: Sized+Clone {
//    type Key: Ord;
//
//    #[inline(always)]
//    fn key(&self) -> &Self::Key;
//}
//
//
//impl Item for usize {
//    type Key = usize;
//
//    #[inline(always)]
//    fn key(&self) -> &Self::Key {
//        self
//    }
//}

#[derive(Debug, Clone)]
pub struct Node<T: Ord> {
    pub item: T,
}

impl<T: Ord> Node<T> {
    pub fn new(item: T) -> Self {
        Node { item: item }
    }
}

/// A fast way to refill the tree from a master copy; adds the requirement for T to implement Copy.
pub trait TeardownTreeRefill<T: Copy+Ord> {
    fn refill(&mut self, master: &Self);
}


impl<T: Copy+Ord> TeardownTreeRefill<T> for TeardownTree<T> {
    fn refill(&mut self, master: &TeardownTree<T>) {
        self.internal.refill(&master.internal)
    }
}

impl<T: Copy+Ord> TeardownTreeRefill<T> for TeardownTreeInternal<T> {
    fn refill(&mut self, master: &TeardownTreeInternal<T>) {
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
//            let len = self.data().len();
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

#[derive(Clone)]
pub struct TeardownTree<T: Ord> {
    internal: TeardownTreeInternal<T>
}

#[derive(Clone)]
pub struct TeardownTreeInternal<T: Ord> {
    pub data: Vec<Node<T>>,
    pub mask: Vec<bool>,
    size: usize,

    pub delete_range_cache: DeleteRangeCache,
}

impl<T: Ord> TeardownTree<T> {
    /// Constructs a new TeardownTree<T>
    /// **Note:** the argument must be sorted!
    pub fn new(sorted: Vec<T>) -> TeardownTree<T> {
        TeardownTree { internal: TeardownTreeInternal::new(sorted) }
    }

    pub fn with_nodes(nodes: Vec<Option<Node<T>>>) -> TeardownTree<T> {
        TeardownTree { internal: TeardownTreeInternal::with_nodes(nodes) }
    }


    pub fn size(&self) -> usize {
        self.internal.size()
    }

    pub fn clear(&mut self) {
        self.internal.clear()
    }
}


impl<T: Ord> TeardownTreeInternal<T> {
    /// Constructs a new TeardownTree<T>
    /// Note: the argument must be sorted!
    pub fn new(mut sorted: Vec<T>) -> TeardownTreeInternal<T> {
        let size = sorted.len();

        let capacity = size;

        let mut data = Vec::with_capacity(capacity);
        unsafe { data.set_len(capacity); }

        let mask: Vec<bool> = vec![true; capacity];
        let height = Self::build(&mut sorted, 0, &mut data);
        unsafe { sorted.set_len(0); }
        let cache = DeleteRangeCache::new(height);
        TeardownTreeInternal { data: data, mask: mask, size: size, delete_range_cache: cache }
    }

    fn calc_height(nodes: &Vec<Option<Node<T>>>, idx: usize) -> usize {
        if idx < nodes.len() && nodes[idx].is_some() {
            1 + max(Self::calc_height(nodes, lefti(idx)),
                    Self::calc_height(nodes, righti(idx)))
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
            n - x/2
        }
    }

    /// Returns the height of the tree.
    fn build(sorted: &mut [T], idx: usize, data: &mut [Node<T>]) -> usize {
        match sorted.len() {
            0 => 0,
            n => {
                let mid = Self::build_select_root(n);
                let (lefti, righti) = (lefti(idx), righti(idx));
                let lh = Self::build(&mut sorted[..mid], lefti, data);
                let rh = Self::build(&mut sorted[mid+1..], righti, data);

                unsafe {
                    let p = sorted.get_unchecked(mid);
                    let item = ptr::read(p);
                    ptr::write(data.get_unchecked_mut(idx), Node { item: item });
                }

                debug_assert!(rh <= lh);
                1 + lh
            }
        }
    }
    /// Constructs a new TeardownTree<T> based on raw nodes vec.
    pub fn with_nodes(mut nodes: Vec<Option<Node<T>>>) -> TeardownTreeInternal<T> {
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
        TeardownTreeInternal { data: data, mask: mask, size: size, delete_range_cache: cache }
    }

//    fn into_node_vec(self) -> Vec<Option<Node<T>>> {
//        self.data()
//            .into_iter()
//            .zip(self.mask().into_iter())
//            .map(|(node, flag)| if flag {
//                    Some(node)
//                } else {
//                    None
//                })
//            .collect::<Vec<Option<Node<T>>>>()
//    }
}

impl<T: Ord> Drop for TeardownTreeInternal<T> {
    fn drop(&mut self) {
        self.drop_items();
        unsafe {
            self.data.set_len(0)
        }
    }
}

pub trait TreeInternalBase<T: Ord> {
    fn data(&self) -> &Vec<Node<T>>;
    fn data_mut(&mut self) -> &mut Vec<Node<T>>;

    fn mask(&self) -> &Vec<bool>;
    fn mask_mut(&mut self) -> &mut Vec<bool>;

    fn size(&self) -> usize;
    fn size_mut(&mut self) -> &mut usize;

    fn slots_min(&mut self) -> &mut SlotStack;
    fn slots_max(&mut self) -> &mut SlotStack;
}

pub trait TreeInternal<T: Ord>: TreeInternalBase<T> {
    #[inline(always)]
    fn item_mut_unsafe<'b>(&mut self, idx: usize) -> &'b mut T {
        unsafe {
            mem::transmute(&mut self.node_mut(idx).item)
        }
    }

    #[inline(always)]
    fn item_mut(&mut self, idx: usize) -> &mut T {
        &mut self.node_mut(idx).item
    }

    #[inline(always)]
    fn item(&self, idx: usize) -> &T {
        &self.node(idx).item
    }


    /// Finds the item with the given key and returns it (or None).
    fn lookup(&self, search: &T) -> Option<&T> {
        self.index_of(search).map(|idx| self.item(idx))
    }

    fn index_of(&self, search: &T) -> Option<usize> {
        if self.data().is_empty() {
            return None;
        }

        let mut idx = 0;
        let mut key =
        if self.mask()[idx] {
            self.item(idx)
        } else {
            return None;
        };

        loop {
            let ordering = search.cmp(&key);

            idx = match ordering {
                Ordering::Equal   => return Some(idx),
                Ordering::Less    => lefti(idx),
                Ordering::Greater => righti(idx),
            };

            if idx >= self.data().len() {
                return None;
            }

            if self.mask()[idx] {
                key = self.item(idx);
            } else {
                return None;
            }
        }
    }


    #[inline]
    fn find_max(&mut self, mut idx: usize) -> usize {
        while self.has_right(idx) {
            idx = righti(idx);
        }
        idx
    }

    #[inline]
    fn find_min(&mut self, mut idx: usize) -> usize {
        while self.has_left(idx) {
            idx = lefti(idx);
        }
        idx
    }

    #[inline(always)]
    fn node(&self, idx: usize) -> &Node<T> {
        &self.data()[idx]
    }

    #[inline(always)]
    fn node_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data_mut()[idx]
    }

    #[inline(always)]
    fn node_opt(&self, idx: usize) -> Option<&Node<T>> {
        if self.is_nil(idx) { None } else { Some(self.node(idx)) }
    }

    #[inline(always)]
    fn parent_opt(&self, idx: usize) -> Option<&Node<T>> {
        if idx == 0 {
            None
        } else {
            Some(&self.data()[parenti(idx)])
        }
    }

    #[inline(always)]
    fn left_opt(&self, idx: usize) -> Option<&Node<T>> {
        let lefti = lefti(idx);
        if self.is_nil(lefti) {
            None
        } else {
            Some(&self.data()[lefti])
        }
    }

    #[inline(always)]
    fn right_opt(&self, idx: usize) -> Option<&Node<T>> {
        let righti = righti(idx);
        if self.is_nil(righti) {
            None
        } else {
            Some(&self.data()[righti])
        }
    }


    #[inline(always)]
    fn parent(&self, idx: usize) -> &Node<T> {
        let parenti = parenti(idx);
        debug_assert!(idx > 0 && !self.is_nil(idx));
        &self.data()[parenti]
    }

    #[inline(always)]
    fn left(&self, idx: usize) -> &Node<T> {
        let lefti = lefti(idx);
        debug_assert!(!self.is_nil(lefti));
        &self.data()[lefti]
    }

    #[inline(always)]
    fn right(&self, idx: usize) -> &Node<T> {
        let righti = righti(idx);
        debug_assert!(!self.is_nil(righti));
        &self.data()[righti]
    }


    #[inline(always)]
    fn has_left(&self, idx: usize) -> bool {
        !self.is_nil(lefti(idx))
    }

    #[inline(always)]
    fn has_right(&self, idx: usize) -> bool {
        !self.is_nil(righti(idx))
    }

    #[inline(always)]
    fn is_nil(&self, idx: usize) -> bool {
        idx >= self.data().len() || !unsafe { *self.mask().get_unchecked(idx) }
    }


    /// Returns the closest subtree A enclosing `idx`, such that A is the left child (or 0 if no such
    /// node is found). `idx` is considered to enclose itself, so we return `idx` if it is the left
    /// child.
    #[inline(always)]
    fn left_enclosing(idx: usize) -> usize {
        debug_assert!((idx+2) & (idx+1) != 0, "idx={}", idx);

        if idx & 1 == 0 {
            if idx & 2 == 0 {
                parenti(idx)
            } else {
                let t = idx + 2;
                let shift = t.trailing_zeros();
                (idx >> shift) - 1
            }
        } else {
            idx
        }
    }

    /// Returns the closest subtree A enclosing `idx`, such that A is the right child (or 0 if no such
    /// node is found). `idx` is considered to enclose itself, so we return `idx` if it is the right
    /// child.
    #[inline(always)]
    fn right_enclosing(idx: usize) -> usize {
        if idx & 1 == 0 {
            idx
        } else {
            if idx & 2 == 0 {
                parenti(idx)
            } else {
                let t = idx + 1;
                let shift = t.trailing_zeros();
                (idx >> shift) - 1
            }
        }
    }

    #[inline]
    fn traverse_preorder<A, F>(&mut self, root: usize, a: &mut A, mut f: F)
        where F: FnMut(&mut Self, &mut A, usize) {
        if self.is_nil(root) {
            return;
        }

        let mut next = root;

        loop {
            next = {
                f(self, a, next);

                if self.has_left(next) {
                    lefti(next)
                } else if self.has_right(next) {
                    righti(next)
                } else {
                    loop {
                        let l_enclosing = {
                            let z = next + 2;

                            if z == z & (!z+1) {
                                0
                            } else {
                                Self::left_enclosing(next)
                            }
                        } ;

                        if l_enclosing <= root {
                            // done
                            return;
                        }

                        next = l_enclosing + 1; // right sibling
                        if !self.is_nil(next) {
                            break;
                        }
                    }
                    next
                }
            };
        }
    }

    #[inline(never)]
    fn traverse_inorder<A, F>(&mut self, root: usize, a: &mut A, mut on_next: F)
        where F: FnMut(&mut Self, &mut A, usize) -> bool {
        if self.is_nil(root) {
            return;
        }

        let mut next = self.find_min(root);

        loop {
            next = {
                let stop = on_next(self, a, next);
                if stop {
                    break;
                }

                if self.has_right(next) {
                    self.find_min(righti(next))
                } else {
                    // handle the case where we are on strictly right-hand path from the root.
                    // we don't need this in the current user code, but it can happen generally
//                    if (next+2) & (next+1) == 0 {
//                        break;
//                    }

                    let l_enclosing = Self::left_enclosing(next);

                    if l_enclosing <= root {
                        // done
                        break;
                    }

                    parenti(l_enclosing)
                }
            }
        }
    }


    #[inline]
    fn traverse_inorder_rev<A, F>(&mut self, root: usize, a: &mut A, mut f: F)
                                                        where F: FnMut(&mut Self, &mut A, usize) {
        if self.is_nil(root) {
            return;
        }

        let mut next = self.find_max(root);

        loop {
            next = {
                f(self, a, next);

                if self.has_left(next) {
                    self.find_max(lefti(next))
                } else {
                    let r_enclosing = Self::right_enclosing(next);

                    if r_enclosing <= root {
                        // done
                        return;
                    }

                    parenti(r_enclosing)
                }
            }
        }
    }

    #[inline(always)]
    fn take(&mut self, idx: usize) -> T {
        debug_assert!(!self.is_nil(idx), "idx={}, mask[idx]={}", idx, self.mask()[idx]);
        let p: *const Node<T> = unsafe {
            self.data().get_unchecked(idx)
        };
        self.mask_mut()[idx] = false;
        *self.size_mut() -= 1;
        unsafe { ptr::read(&(*p).item) }
    }

    #[inline(always)]
    unsafe fn move_to<S: Sink<T>>(&mut self, idx: usize, sink: &mut S) {
        debug_assert!(!self.is_nil(idx), "idx={}, mask[idx]={}", idx, self.mask()[idx]);
        *self.mask_mut().get_unchecked_mut(idx) = false;
        *self.size_mut() -= 1;
        let p: *const Node<T> = self.data().get_unchecked(idx);

        let item = ptr::read(&(*p).item);
        sink.consume_unchecked(item);
    }

    #[inline(always)]
    unsafe fn move_from_to(&mut self, src: usize, dst: usize) {
        debug_assert!(!self.is_nil(src) && self.is_nil(dst), "is_nil(src)={}, is_nil(dst)={}", self.is_nil(src), self.is_nil(dst));
        *self.mask_mut().get_unchecked_mut(src) = false;
        *self.mask_mut().get_unchecked_mut(dst) = true;
        let pdata = self.data_mut().as_mut_ptr();
        let psrc: *mut Node<T> = pdata.offset(src as isize);
        let pdst: *mut Node<T> = pdata.offset(dst as isize);
        let x = ptr::read(psrc);
        ptr::write(pdst, x);
    }

    #[inline(always)]
    fn place(&mut self, idx: usize, item: T) {
        if self.mask()[idx] {
            self.data_mut()[idx].item = item;
        } else {
            self.mask_mut()[idx] = true;
            *self.size_mut() += 1;
            unsafe {
                let p = self.data_mut().get_unchecked_mut(idx);
                ptr::write(p, Node::new(item));
            };
        }
    }

    fn clear(&mut self) {
        self.drop_items();
    }

    fn drop_items(&mut self) {
        if self.size()*2 <= self.data().len() {
            self.traverse_preorder(0, &mut 0, |this: &mut Self, _, idx| {
                unsafe {
                    let p = this.data_mut().get_unchecked_mut(idx);
                    ptr::drop_in_place(p);
                }

                this.mask_mut()[idx] = false;
            })
        } else {
            let p = self.data_mut().as_mut_ptr();
            for i in 0..self.size() {
                if self.mask()[i] {
                    unsafe {
                        ptr::drop_in_place(p.offset(i as isize));
                    }

                    self.mask_mut()[i] = false;
                }
            }
        }

        *self.size_mut() = 0;
    }
}
impl<T: Ord> TreeInternal<T> for TeardownTreeInternal<T> {}


impl<T: Ord> TreeInternalBase<T> for TeardownTreeInternal<T> {
    fn data(&self) -> &Vec<Node<T>> { &self.data }
    fn data_mut(&mut self) -> &mut Vec<Node<T>> { &mut self.data }
    fn mask(&self) -> &Vec<bool> { &self.mask }
    fn mask_mut(&mut self) -> &mut Vec<bool> { &mut self.mask }
    fn size(&self) -> usize { self.size }
    fn size_mut(&mut self) -> &mut usize { &mut self.size }
    fn slots_min(&mut self) -> &mut SlotStack { &mut self.delete_range_cache.slots_min}
    fn slots_max(&mut self) -> &mut SlotStack { &mut self.delete_range_cache.slots_max }
}


impl<T: Ord+Debug> Debug for TeardownTree<T> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.internal, fmt)
    }
}


impl<T: Ord+Debug> Debug for TeardownTreeInternal<T> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let mut nz: Vec<_> = self.mask().iter().enumerate()
            .rev()
            .skip_while(|&(_, flag)| !flag)
            .map(|(i, &flag)| match (self.node(i), flag) {
                (_, false) => String::from("0"),
                (ref node, true) => format!("{:?}", node.item)
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


impl<T: Ord+Debug> fmt::Display for TeardownTree<T> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.internal, fmt)
    }
}


impl<T: Ord+Debug> fmt::Display for TeardownTreeInternal<T> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        writeln!(fmt, "")?;
        let mut ancestors = vec![];
        self.fmt_subtree(fmt, 0, &mut ancestors)
    }
}

impl<T: Ord+Debug> TeardownTreeInternal<T> {
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

        if !self.is_nil(idx) {
            writeln!(fmt, "{:?}", self.item(idx))?;

            if idx%2 == 0 && !ancestors.is_empty() {
                *ancestors.last_mut().unwrap() = false;
            }

            if self.has_left(idx) || self.has_right(idx) {
                ancestors.push(true);
                self.fmt_subtree(fmt, lefti(idx), ancestors)?;
                self.fmt_subtree(fmt, righti(idx), ancestors)?;
                ancestors.pop();
            }
        } else {
            writeln!(fmt, "X")?;
        }

        Ok(())
    }
}


#[inline(always)]
pub fn parenti(idx: usize) -> usize {
    (idx-1) >> 1
}

#[inline(always)]
pub fn lefti(idx: usize) -> usize {
    (idx<<1) + 1
}

#[inline(always)]
pub fn righti(idx: usize) -> usize {
    (idx<<1) + 2
}


pub trait InternalAccess<T: Ord> {
    fn internal(&mut self) -> &mut TeardownTreeInternal<T>;
}

impl<T: Ord> InternalAccess<T> for TeardownTree<T> {
    fn internal(&mut self) -> &mut TeardownTreeInternal<T> {
        &mut self.internal
    }
}