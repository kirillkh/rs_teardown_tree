use base::{Node, lefti, righti, parenti, consume_unchecked, SlotStack};
use base::bulk_delete::DeleteRangeCache;
use std::cmp::{Ordering};
use std::fmt::{Debug, Formatter};
use std::fmt;
use std::mem;
use std::ptr;
use std::cmp::{max};
use std::ops::{Deref, DerefMut};


pub trait Key: Ord+Clone {}

impl<T: Ord+Clone> Key for T {}

pub trait TreeDeref<N: Node>: Deref<Target=TreeRepr<N>> {}
pub trait TreeDerefMut<N: Node>: TreeDeref<N> + DerefMut {}

impl<N: Node, T> TreeDeref<N> for T where T: Deref<Target=TreeRepr<N>> {}
impl<N: Node, T> TreeDerefMut<N> for T where T: Deref<Target=TreeRepr<N>> + DerefMut {}

#[derive(Clone)]
pub struct TreeRepr<N: Node> {
    pub data: Vec<N>,
    pub mask: Vec<bool>,
    pub size: usize,

    pub delete_range_cache: DeleteRangeCache,
}


impl<N: Node> TreeRepr<N> {
    pub fn new(mut items: Vec<(N::K, N::V)>) -> TreeRepr<N> {
        items.sort_by(|a, b| a.0.cmp(&b.0));
        Self::with_sorted(items)
    }


    /// Constructs a new TeardownTree<T>
    /// Note: the argument must be sorted!
    pub fn with_sorted(mut sorted: Vec<(N::K, N::V)>) -> TreeRepr<N> {
        let size = sorted.len();

        let capacity = size;

        let mut data = Vec::with_capacity(capacity);
        unsafe { data.set_len(capacity); }

        let mask: Vec<bool> = vec![true; capacity];
        let height = Self::build(&mut sorted, 0, &mut data);
        unsafe { sorted.set_len(0); }
        let cache = DeleteRangeCache::new(height);
        TreeRepr { data: data, mask: mask, size: size, delete_range_cache: cache }
    }

    pub fn calc_height(nodes: &Vec<Option<N>>, idx: usize) -> usize {
        if idx < nodes.len() && nodes[idx].is_some() {
            1 + max(Self::calc_height(nodes, lefti(idx)),
                    Self::calc_height(nodes, righti(idx)))
        } else {
            0
        }
    }

    /// Finds the point to partition n keys for a nearly-complete binary tree
    /// http://stackoverflow.com/a/26896494/3646645
    pub fn build_select_root(n: usize) -> usize {
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
    pub fn build(sorted: &mut [(N::K, N::V)], idx: usize, data: &mut [N]) -> usize {
        match sorted.len() {
            0 => 0,
            n => {
                let mid = Self::build_select_root(n);
                let (lefti, righti) = (lefti(idx), righti(idx));
                let lh = Self::build(&mut sorted[..mid], lefti, data);
                let rh = Self::build(&mut sorted[mid+1..], righti, data);

                unsafe {
                    let p = sorted.get_unchecked(mid);
                    let (k, v) = ptr::read(p);
                    ptr::write(data.get_unchecked_mut(idx), N::new(k, v));
                }

                debug_assert!(rh <= lh);
                1 + lh
            }
        }
    }

    /// Constructs a new TeardownTree<T> based on raw nodes vec.
    pub fn with_nodes(mut nodes: Vec<Option<N>>) -> TreeRepr<N> {
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
                let garbage = mem::replace(&mut data[i], node);
                mem::forget(garbage);
            }
        }

        let cache = DeleteRangeCache::new(height);
        TreeRepr { data: data, mask: mask, size: size, delete_range_cache: cache }
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


impl<N: Node> TreeRepr<N> {
    /// Finds the item with the given key and returns it (or None).
    pub fn lookup<'a>(&'a self, search: &'a N::K) -> Option<&'a N::V> where N: 'a {
        self.index_of(search).map(|idx| self.val(idx))
    }

    pub fn index_of(&self, search: &N::K) -> Option<usize> {
        if self.data.is_empty() {
            return None;
        }

        let mut idx = 0;
        let mut key =
        if self.mask[idx] {
            self.key(idx)
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

            if idx >= self.data.len() {
                return None;
            }

            if self.mask[idx] {
                key = self.key(idx);
            } else {
                return None;
            }
        }
    }

    #[inline(always)]
    pub fn node_unsafe<'b>(&self, idx: usize) -> &'b N {
        unsafe {
            mem::transmute(self.node(idx))
        }
    }

    #[inline(always)]
    pub fn key<'a>(&'a self, idx: usize) -> &'a N::K where N: 'a {
        &self.node(idx).key
    }

    #[inline(always)]
    pub fn val<'a>(&'a self, idx: usize) -> &'a N::V where N: 'a {
        &self.data[idx].val
    }

    #[inline(always)]
    pub fn node(&self, idx: usize) -> &N {
        &self.data[idx]
    }

    #[inline(always)]
    pub fn node_opt(&self, idx: usize) -> Option<&N> {
        if self.is_nil(idx) { None } else { Some(self.node(idx)) }
    }

    #[inline(always)]
    pub fn parent_opt(&self, idx: usize) -> Option<&N> {
        if idx == 0 {
            None
        } else {
            Some(&self.data[parenti(idx)])
        }
    }

    #[inline(always)]
    pub fn left_opt(&self, idx: usize) -> Option<&N> {
        let lefti = lefti(idx);
        if self.is_nil(lefti) {
            None
        } else {
            Some(&self.data[lefti])
        }
    }

    #[inline(always)]
    pub fn right_opt(&self, idx: usize) -> Option<&N> {
        let righti = righti(idx);
        if self.is_nil(righti) {
            None
        } else {
            Some(&self.data[righti])
        }
    }


    #[inline(always)]
    pub fn parent(&self, idx: usize) -> &N {
        let parenti = parenti(idx);
        debug_assert!(idx > 0 && !self.is_nil(idx));
        &self.data[parenti]
    }

    #[inline(always)]
    pub fn left(&self, idx: usize) -> &N {
        let lefti = lefti(idx);
        debug_assert!(!self.is_nil(lefti));
        &self.data[lefti]
    }

    #[inline(always)]
    pub fn right(&self, idx: usize) -> &N {
        let righti = righti(idx);
        debug_assert!(!self.is_nil(righti));
        &self.data[righti]
    }


    #[inline(always)]
    pub fn has_left(&self, idx: usize) -> bool {
        !self.is_nil(lefti(idx))
    }

    #[inline(always)]
    pub fn has_right(&self, idx: usize) -> bool {
        !self.is_nil(righti(idx))
    }

    #[inline(always)]
    pub fn is_nil(&self, idx: usize) -> bool {
        idx >= self.data.len() || !unsafe { *self.mask.get_unchecked(idx) }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    #[inline(always)]
    pub fn node_mut_unsafe<'b>(&mut self, idx: usize) -> &'b mut N {
        unsafe {
            mem::transmute(self.node_mut(idx))
        }
    }

    #[inline(always)]
    pub fn key_mut_unsafe<'b>(&mut self, idx: usize) -> &'b mut N::K {
        unsafe {
            mem::transmute(&mut self.node_mut(idx).key)
        }
    }

    #[inline(always)]
    pub fn key_mut<'a>(&'a mut self, idx: usize) -> &'a mut N::K where N: 'a {
        &mut self.node_mut(idx).key
    }

    #[inline]
    pub fn find_max(&mut self, mut idx: usize) -> usize {
        while self.has_right(idx) {
            idx = righti(idx);
        }
        idx
    }

    #[inline]
    pub fn find_min(&mut self, mut idx: usize) -> usize {
        while self.has_left(idx) {
            idx = lefti(idx);
        }
        idx
    }

    #[inline(always)]
    pub fn node_mut(&mut self, idx: usize) -> &mut N {
        &mut self.data[idx]
    }

    #[inline(always)]
    pub fn take(&mut self, idx: usize) -> N {
        debug_assert!(!self.is_nil(idx), "idx={}, mask[idx]={}", idx, self.mask[idx]);
        let p: *const N = unsafe {
            self.data.get_unchecked(idx)
        };
        self.mask[idx] = false;
        self.size -= 1;
        unsafe { ptr::read(&(*p)) }
    }

    #[inline(always)]
    pub unsafe fn move_to(&mut self, idx: usize, output: &mut Vec<(N::K, N::V)>) {
        debug_assert!(!self.is_nil(idx), "idx={}, mask[idx]={}", idx, self.mask[idx]);
        *self.mask.get_unchecked_mut(idx) = false;
        self.size -= 1;
        let p: *const N = self.data.get_unchecked(idx);

        let node = ptr::read(&*p);
        consume_unchecked(output, node.into_kv());
    }

    #[inline(always)]
    pub unsafe fn move_from_to(&mut self, src: usize, dst: usize) {
        debug_assert!(!self.is_nil(src) && self.is_nil(dst), "is_nil(src)={}, is_nil(dst)={}", self.is_nil(src), self.is_nil(dst));
        *self.mask.get_unchecked_mut(src) = false;
        *self.mask.get_unchecked_mut(dst) = true;
        let pdata = self.data.as_mut_ptr();
        let psrc: *mut N = pdata.offset(src as isize);
        let pdst: *mut N = pdata.offset(dst as isize);
        let x = ptr::read(psrc);
        ptr::write(pdst, x);
    }

    #[inline(always)]
    pub fn place(&mut self, idx: usize, node: N) {
        if self.mask[idx] {
            self.data[idx] = node;
        } else {
            self.mask[idx] = true;
            self.size += 1;
            unsafe {
                let p = self.data.get_unchecked_mut(idx);
                ptr::write(p, node);
            };
        }
    }

    pub fn clear(&mut self) {
        self.drop_items();
    }

    pub fn drop_items(&mut self) {
        if self.size*2 <= self.data.len() {
            let mut this: &mut TreeRepr<N> = &mut *self; // magic! doesn't compile without this
            this.traverse_preorder(0, &mut 0, |this, _, idx| {
                unsafe {
                    let p = this.data.get_unchecked_mut(idx);
                    ptr::drop_in_place(p);
                }

                this.mask[idx] = false;
            })
        } else {
            let p = self.data.as_mut_ptr();
            for i in 0..self.size {
                if self.mask[i] {
                    unsafe {
                        ptr::drop_in_place(p.offset(i as isize));
                    }

                    self.mask[i] = false;
                }
            }
        }

        self.size = 0;
    }


    pub fn slots_min<'a>(&'a mut self) -> &'a mut SlotStack where N: 'a {
        &mut self.delete_range_cache.slots_min
    }

    pub fn slots_max<'a>(&'a mut self) -> &'a mut SlotStack where N: 'a {
        &mut self.delete_range_cache.slots_max
    }
}



pub trait Traverse<N: Node>: TreeDerefMut<N> {
    /// Returns the closest subtree A enclosing `idx`, such that A is the left child (or 0 if no such
    /// node is found). `idx` is considered to enclose itself, so we return `idx` if it is the left
    /// child.
    /// **Attention!** For efficiency reasons, idx and return value are both **1-based**.
    #[inline(always)]
    fn left_enclosing(idx: usize) -> usize {
        if idx & 1 == 0 {
            idx
        } else if idx & 2 == 0 {
            idx >> 1
        } else {
            // optimizaion: the two lines below could be the sole body of this function; the 2 branches
            // above are special cases
            let shift = (idx + 1).trailing_zeros();
            idx >> shift
        }
    }

    /// Returns the closest subtree A enclosing `idx`, such that A is the right child (or 0 if no such
    /// node is found). `idx` is considered to enclose itself, so we return `idx` if it is the right
    /// child.
    /// **Attention!** For efficiency reasons, idx and return value are both **1-based**.
    #[inline(always)]
    fn right_enclosing(idx: usize) -> usize {
        if idx & 1 == 1 {
            idx
        } else if idx & 2 == 1 {
            idx >> 1
        } else {
            // optimizaion: the two lines below could be the sole body of this function; the 2 branches
            // above are special cases
            let shift = idx.trailing_zeros();
            idx >> shift
        }
    }


    #[inline]
    fn traverse_preorder<A, F>(&mut self, root: usize, a: &mut A, mut f: F)
        where F: FnMut(&mut Self, &mut A, usize)
    {
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
                                Self::left_enclosing(next+1)-1
                            }
                        };

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
                    let l_enclosing = Self::left_enclosing(next+1);

                    if l_enclosing <= root+1 {
                        // done
                        break;
                    }

                    parenti(l_enclosing-1)
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
}


impl<N: Node, T> Traverse<N> for T where T: TreeDerefMut<N> {}



impl<N: Node> Drop for TreeRepr<N> {
    fn drop(&mut self) {
        let mut this = &mut *self;
        this.drop_items();
        unsafe {
            this.data.set_len(0)
        }
    }
}

impl<N: Node> Debug for TreeRepr<N> where N: Debug {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let mut nz: Vec<_> = self.mask.iter().enumerate()
            .rev()
            .skip_while(|&(_, flag)| !flag)
            .map(|(i, &flag)| match (self.node(i), flag) {
                (_, false) => String::from("0"),
                (ref node, true) => format!("{:?}", node)
            })
            .collect();
        nz.reverse();

        let _ = write!(fmt, "[size={}: ", self.size);
        let mut sep = "";
        for ref node in nz.iter() {
            let _ = write!(fmt, "{}", sep);
            sep = ", ";
            let _ = write!(fmt, "{}", node);
        }
        let _ = write!(fmt, "]");
        Ok(())
    }
}

impl<N: Node> fmt::Display for TreeRepr<N> where N: fmt::Debug {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        writeln!(fmt, "")?;
        let mut ancestors = vec![];
        self.fmt_subtree(fmt, 0, &mut ancestors)
    }
}


impl<N: Node> TreeRepr<N> where N: fmt::Debug {
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
            writeln!(fmt, "{:?}", self.node(idx))?;

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
