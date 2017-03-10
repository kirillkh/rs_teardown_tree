use base::{Node, Entry, Sink, lefti, righti, parenti, SlotStack, Refill, depth_of};
use base::bulk_delete::DeleteRangeCache;
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
    data: Vec<N>,
    mask: Vec<bool>,
    pub size: usize,

    delete_range_cache: DeleteRangeCache,
}


//---- Entry points --------------------------------------------------------------------------------
impl<N: Node> TreeRepr<N> {
    pub fn new(mut items: Vec<(N::K, N::V)>) -> TreeRepr<N> {
        items.sort_by(|a, b| a.0.cmp(&b.0));
        Self::with_sorted(items)
    }


    /// Constructs a new TeardownTree<T>
    /// Note: the argument must be sorted!
    pub fn with_sorted(mut sorted: Vec<(N::K, N::V)>) -> TreeRepr<N> {
        let size = sorted.len();

        let mut data = Vec::with_capacity(size);
        let mut mask = Vec::with_capacity(size);

        // We use manual management of `data`'s  and `mask`'s memory. To ensure nothing bad is going on, we
        // analyze each access to `data`.
        unsafe {
            data.set_len(size);
            mask.set_len(size);
        }

        let height = Self::build_complete(&mut sorted, 0, &mut data, &mut mask, &|item| N::from_tuple(item));
        // As per contract with `build()`, we safely dispose of the contents of `sorted` without dropping them.
        unsafe { sorted.set_len(0); }
        let cache = DeleteRangeCache::new(height);
        TreeRepr { data: data, mask: mask, size: size, delete_range_cache: cache }
    }

    /// Constructs a new TreeRepr<T> based on raw nodes vec.
    pub fn with_nodes(mut nodes: Vec<Option<N>>) -> TreeRepr<N> {
        let size = nodes.iter().filter(|x| x.is_some()).count();
        let height = Self::calc_height(&nodes, 0);
        let capacity = nodes.len();

        let mut mask = vec![false; capacity];
        let mut data = Vec::with_capacity(capacity);
        // We use manual management of the memory inside `data`. To ensure nothing bad is going on,
        // we analyze each access to `data`.
        unsafe {
            data.set_len(capacity);
        }

        for i in 0..capacity {
            if let Some(node) = nodes[i].take() {
                mask[i] = true;
                // This is safe: data[i] contains garbage, therefore we must not drop its content.
                unsafe {
                    ptr::write(&mut data[i], node);
                }
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


    /// Finds the item with the given key and returns it (or None).
    pub fn find<'a, Q>(&'a self, query: &'a Q) -> Option<&'a N::V>
        where N: 'a, Q: PartialOrd<N::K>
    {
        let idx = self.index_of(query);
        if self.is_nil(idx) {
            None
        } else {
            Some(self.val(idx))
        }
    }

    pub fn contains<Q: PartialOrd<N::K>>(&self, query: &Q) -> bool {
        self.find(query).is_some()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn clear(&mut self) {
        self.drop_items();
    }

    pub fn capacity(&self) -> usize {
        self.data.len()
    }
}


//---- Accessors -----------------------------------------------------------------------------------
impl<N: Node> TreeRepr<N> {
    // The caller must make sure idx is inside bounds.
    #[inline(always)]
    pub fn mask(&self, idx: usize) -> bool {
        debug_assert!(idx < self.data.len());
        unsafe {
            *self.mask.get_unchecked(idx)
        }
    }

    // The caller must make sure idx is inside bounds.
    #[inline(always)]
    pub fn mask_mut(&mut self, idx: usize) -> &mut bool {
        debug_assert!(idx < self.data.len());
        unsafe {
            self.mask.get_unchecked_mut(idx)
        }
    }


    #[inline(always)]
    pub fn key<'a>(&'a self, idx: usize) -> &'a N::K where N: 'a {
        self.node(idx).key()
    }

    #[inline(always)]
    pub fn key_mut<'a>(&'a mut self, idx: usize) -> &'a mut N::K where N: 'a {
        self.node_mut(idx).key_mut()
    }

    // Spoofs the lifetime of the reference to self.key(idx), which is required to work around the
    // borrow checker on some occasions. The caller must ensure the reference does not outlive the
    // content.
    #[inline(always)]
    pub fn key_unsafe<'a>(&self, idx: usize) -> &'a N::K where N: 'a {
        self.node_unsafe(idx).key()
    }

    // Spoofs the lifetime of the reference to self.key_mut(idx), which is required to work around
    // the borrow checker on some occasions. The caller must ensure the reference does not outlive
    // the content and there is no race condition in access to the content.
    #[inline(always)]
    pub fn key_mut_unsafe<'a>(&mut self, idx: usize) -> &'a mut N::K where N: 'a {
        self.node_mut_unsafe(idx).key_mut()
    }


    #[inline(always)]
    pub fn val<'a>(&'a self, idx: usize) -> &'a N::V where N: 'a {
        self.node(idx).val()
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
    pub fn node_mut(&mut self, idx: usize) -> &mut N {
        &mut self.data[idx]
    }

    // Spoofs the lifetime of the reference to self.node(idx), which is required to work around the
    // borrow checker on some occasions. The caller must ensure the reference does not outlive the
    // content.
    #[inline(always)]
    pub fn node_unsafe<'a>(&self, idx: usize) -> &'a N where N: 'a {
        debug_assert!(idx < self.data.len());
        unsafe {
            mem::transmute(self.data.get_unchecked(idx))
        }
    }

    // Spoofs the lifetime of the reference to self.node_mut(idx), which is required to work around
    // the borrow checker on some occasions. The caller must ensure the reference does not outlive
    // the content and there is no race condition in access to the content.
    #[inline(always)]
    pub fn node_mut_unsafe<'a>(&mut self, idx: usize) -> &'a mut N where N: 'a {
        unsafe {
            mem::transmute(self.data.get_unchecked_mut(idx))
        }
    }


    #[inline(always)]
    pub fn parent(&self, idx: usize) -> &N {
        debug_assert!(idx > 0 && !self.is_nil(idx));
        self.node(parenti(idx))
    }

    #[inline(always)]
    pub fn parent_opt(&self, idx: usize) -> Option<&N> {
        if idx == 0 {
            None
        } else {
            Some(self.parent(idx))
        }
    }


    #[inline(always)]
    pub fn left(&self, idx: usize) -> &N {
        let lefti = lefti(idx);
        debug_assert!(!self.is_nil(lefti));
        self.node(lefti)
    }

    #[inline(always)]
    pub fn left_opt(&self, idx: usize) -> Option<&N> {
        let lefti = lefti(idx);
        if self.is_nil(lefti) {
            None
        } else {
            Some(self.node(lefti))
        }
    }


    #[inline(always)]
    pub fn right(&self, idx: usize) -> &N {
        let righti = righti(idx);
        debug_assert!(!self.is_nil(righti));
        self.node(righti)
    }

    #[inline(always)]
    pub fn right_opt(&self, idx: usize) -> Option<&N> {
        let righti = righti(idx);
        if self.is_nil(righti) {
            None
        } else {
            Some(self.node(righti))
        }
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
        // This is safe, as we check that idx is in bounds just before reading from it.
        idx >= self.data.len() || !self.mask(idx)
    }
}


//---- Helpers -------------------------------------------------------------------------------------
impl<N: Node> TreeRepr<N> {
    fn calc_height(nodes: &Vec<Option<N>>, idx: usize) -> usize {
        if idx < nodes.len() && nodes[idx].is_some() {
            1 + max(Self::calc_height(nodes, lefti(idx)),
                    Self::calc_height(nodes, righti(idx)))
        } else {
            0
        }
    }


    /// Returns the height of the tree. This consumes the contents of `data`, so the caller must
    /// make sure the contents are never reused or dropped after this call returns.
    pub fn build_complete<T, U, G>(sorted: &[T], root: usize, data: &mut [U], mask: &mut [bool], convert: G) -> usize
        where G: Fn(T) -> U
    {
        let len = sorted.len();
        if len  == 0 {
            return 0;
        }

        let height = depth_of(len-1) + 1;
        let complete_count = (1 << height) - 1;

        let psrc = sorted.as_ptr();
        let pdst = data.as_mut_ptr();

        let mut skipped = 0;
        let mut src_offs = 0;
        while src_offs < len {
//            let local_idx = inorder_to_idx_n(src_offs+skipped, complete_count);
            let inorder = src_offs+skipped;
            let local_idx = a025480(complete_count+1+inorder) - 1;


            if local_idx < len {
                let local_depth = depth_of(local_idx);
                let global_idx = local_idx + (root << local_depth);

                unsafe {
                    let item = ptr::read(psrc.offset(src_offs as isize));
                    ptr::write(pdst.offset(global_idx as isize), convert(item));
                    *mask.get_unchecked_mut(global_idx) = true;
                }
                src_offs += 1;
            } else {
                skipped += 1;
            }
        }

        height
    }

    /// Returns the height of the tree. This consumes the contents of `data`, so the caller must
    /// make sure the contents are never reused or dropped after this call returns.
    pub fn build_dispersed<T, U, G>(sorted: &[T], root: usize, data: &mut [U], mask: &mut [bool], convert: G) -> usize
        where G: Fn(T) -> U
    {
        let len = sorted.len();
        if len  == 0 {
            return 0;
        }

        let height = depth_of(len-1) + 1;
        let complete_count = (1 << height) - 1;

        let complete_leaves = (complete_count+1) >> 1;
        let internal_nodes = complete_count - complete_leaves;
        let leaves = len - internal_nodes;
        let nils = complete_leaves - leaves;

        let full_gap = nils / leaves;
        let mut remainder = nils % leaves;
        let mut gap = 0;

        let psrc = sorted.as_ptr();
        let pdst = data.as_mut_ptr();

        let mut skipped = 0;
        let mut src_offs = 0;
        while src_offs < len {
            let inorder = src_offs+skipped;
            let local_idx = a025480(complete_count+1+inorder) - 1;

            let mut skip = local_idx >= internal_nodes;
            if skip {
                skip = if gap == 0 {
                    gap = full_gap +
                        if remainder != 0 {
                            remainder -= 1;
                            1
                        } else {
                            0
                        };
                    false
                } else {
                    gap -= 1;
                    true
                }
            }

            if !skip {
                let local_depth = depth_of(local_idx);
                let global_idx = local_idx + (root << local_depth);

                unsafe {
                    let item = ptr::read(psrc.offset(src_offs as isize));
                    ptr::write(pdst.offset(global_idx as isize), convert(item));
                    *mask.get_unchecked_mut(global_idx) = true;
                }
                src_offs += 1;
            } else {
                skipped += 1;
            }
        }

        height
    }

    #[inline]
    pub fn successor(&self, curr: usize) -> Option<usize> {
        let next = if self.has_right(curr) {
            self.find_min(righti(curr))
        } else {
            let l_enclosing = left_enclosing(curr+1);

            if l_enclosing <= 1 {
                // done
                return None
            }

            parenti(l_enclosing-1)
        };

        Some(next)
    }



//    pub fn double_and_rebuild(&mut self, item: (N::K, N::V)) {
//        let base = self.capacity();
//        let capacity = base * 2 + 1;
//        let mut data = Vec::with_capacity(capacity);
//        let mut mask = vec![false; capacity];
//
//        // copy the nodes in-order into the leaves of the new tree
//
//        let pdst: *mut N = unsafe { data.as_mut_ptr().offset(base as isize) };
//        let psrc: *const N = unsafe { self.data.as_ptr() };
//
//        let mut inorder_offs = 0;
//        let mut prev_idx = 0;
//
//        Self::traverse_inorder_mut(self, 0, &mut inorder_offs, |_, inorder_offs, idx| {
//            unsafe {
//                let node = ptr::read(psrc.offset(idx as isize));
//                prev_idx = idx;
//                if &item.0 < node.key() {
//                    return true;
//                }
//                ptr::write(pdst.offset(*inorder_offs), node);
//            }
//            *inorder_offs += 1;
//            false
//        });
//
//        unsafe {
//            ptr::write(pdst.offset(inorder_offs), N::from_tuple(item));
//        }
//        inorder_offs += 1;
//
//        Self::traverse_inorder_from_mut(self, prev_idx, 0, &mut inorder_offs, |_, inorder_offs, idx| {
//            unsafe {
//                let node = ptr::read(psrc.offset(idx as isize));
//                ptr::write(pdst.offset(*inorder_offs), node);
//            }
//            *inorder_offs += 1;
//            false
//        });
//
//        let mut old_data = mem::replace(&mut self.data, data);
//        unsafe {
//            self.data.set_len(capacity);
//            // the items have been moved to the new storage, don't drop them
//            old_data.set_len(0);
//        }
//        self.mask = mask;
//        self.size += 1;
//
//        // build the tree
//        {
//            let src: &mut [N] = unsafe { mem::transmute(&mut self.data[base .. base+self.size]) };
//            let dst: &mut [N] = &mut self.data;
//            Self::build(src, 0, dst, &mut self.mask, &|n| n >> 1, &|node| node);
//        }
//    }
    pub fn layout_inorder_for_insert(&mut self, root: usize, dst: &mut Vec<N>, count: usize, item: (N::K, N::V)) {
        let mut inorder_offs: usize = 0;
        let mut prev_idx = 0;

        let psrc: *const N = self.data.as_ptr();
        let pdst: *mut N = unsafe { dst.as_mut_ptr().offset(dst.len() as isize) };

        Self::traverse_inorder_mut(self, root, &mut inorder_offs, |_, inorder_offs, idx| {
            unsafe {
                let node = ptr::read(psrc.offset(idx as isize));
                prev_idx = idx;
                if &item.0 < node.key() {
                    return true;
                }
                ptr::write(pdst.offset(*inorder_offs as isize), node);
            }
            *inorder_offs += 1;
            false
        });

        unsafe {
            ptr::write(pdst.offset(inorder_offs as isize), N::from_tuple(item));
        }
        inorder_offs += 1;

        if inorder_offs < count {
            Self::traverse_inorder_from_mut(self, prev_idx, root, &mut inorder_offs, |_, inorder_offs, idx| {
                unsafe {
                    let node = ptr::read(psrc.offset(idx as isize));
                    ptr::write(pdst.offset(*inorder_offs as isize), node);
                }
                *inorder_offs += 1;
                false
            });
        }

        unsafe {
            let len = dst.len();
            dst.set_len(len + inorder_offs);
        }
    }

    pub fn rebuild_subtree(&mut self, root: usize, insert_offs: usize, count: usize, item: (N::K, N::V)) {
        // copy the items from the tree to a sorted array
        let mut inorder_items = Vec::with_capacity(count);
        self.layout_inorder_for_insert(root, &mut inorder_items, count, item);

        // reset the mask, because build re-initializes it
        Self::traverse_inorder_mut(self, root, &mut (), |this, _, idx| {
            *this.mask_mut(idx) = false;
            false
        });

        Self::build_dispersed(&inorder_items, root, &mut self.data, &mut self.mask, &|node| node);

        self.size += 1;

        // drop the storage, but not the contents: the items have been moved into the tree
        unsafe { inorder_items.set_len(0); }
    }

    pub fn double_and_rebuild(&mut self, item: (N::K, N::V)) {
        let base = self.capacity();
        let capacity = base * 2 + 1;
        let mut data = Vec::with_capacity(capacity);
        let mask = vec![false; capacity];
        self.size += 1;

        // copy the nodes in-order into the leaves of the new tree
        let pdst: *mut N = unsafe { data.as_mut_ptr().offset(base as isize) };
        let mut inorder_items = unsafe { Vec::from_raw_parts(pdst, 0, self.size) };
        let size = self.size;
        self.layout_inorder_for_insert(0, &mut inorder_items, size, item);

        let mut old_data = mem::replace(&mut self.data, data);
        unsafe {
            self.data.set_len(capacity);
            // the items have been moved to the new storage, don't drop them
            old_data.set_len(0);
        }
        self.mask = mask;

        // build the tree
        Self::build_dispersed(&mut inorder_items, 0, &mut self.data, &mut self.mask, &|node| node);

        mem::forget(inorder_items);
    }

    /// Returns either the index of the first element equal to `query` if it is contained in the tree;
    /// or the index where it can be inserted if it is not.
    pub fn index_of<Q>(&self, query: &Q) -> usize
        where Q: PartialOrd<N::K>
    {
        if self.data.is_empty() || !self.mask(0) {
            return 0;
        }

        let mut idx = 0;
        loop {
            // TODO: this is faster for some benchmarks (10M items/1000 bulks), but slower for very
            // small ones. might want to introduce a heuristic based on n
//            idx = match query.partial_cmp(self.key(idx)).unwrap() {
//                Ordering::Equal   => return idx,
//                Ordering::Less    => lefti(idx),
//                Ordering::Greater => righti(idx),
//            };

            let k = self.key(idx);
            idx =
                if query == k { return idx; }
                else if query < k { lefti(idx) }
                else { righti(idx) };

            if self.is_nil(idx) {
                return idx;
            }
        }
    }

    #[inline]
    pub fn find_max(&self, mut idx: usize) -> usize {
        while self.has_right(idx) {
            idx = righti(idx);
        }
        idx
    }

    #[inline]
    pub fn find_min(&self, mut idx: usize) -> usize {
        while self.has_left(idx) {
            idx = lefti(idx);
        }
        idx
    }

    // The caller must make sure that `!self.is_nil(idx)`
    #[inline(always)]
    pub fn take(&mut self, idx: usize) -> N {
        debug_assert!(!self.is_nil(idx), "idx={}", idx);
        let node = unsafe {
            let p: &N = self.node_unsafe(idx);
            // We take care to set `mask[idx]` to `false`, so we must not drop the content of `p`.
            ptr::read(p)
        };
        *self.mask_mut(idx) = false;
        self.size -= 1;
        node
    }

    // The caller must make sure that `!self.is_nil(idx)`
    #[inline(always)]
    pub fn move_to<S>(&mut self, idx: usize, sink: &mut S)
        where S: Sink<(N::K, N::V)>
    {
        let node = self.take(idx);
        sink.consume(node.into_tuple());
    }

    /// The caller must ensure that:
    ///   a) both `src` and `dst` are valid indices into `data`
    ///   b) `!is_nil(src)`
    ///   c) `is_nil(dst)`
    #[inline(always)]
    pub unsafe fn move_from_to(&mut self, src: usize, dst: usize) {
        debug_assert!(!self.is_nil(src) && self.is_nil(dst), "is_nil(src)={}, is_nil(dst)={}", self.is_nil(src), self.is_nil(dst));
        let pdata = self.data.as_mut_ptr();
        let psrc: *mut N = pdata.offset(src as isize);
        let pdst: *mut N = pdata.offset(dst as isize);
        let x = ptr::read(psrc);
        *self.mask_mut(src) = false;
        *self.mask_mut(dst) = true;
        ptr::write(pdst, x);
    }

    // The caller must make sure that idx is inside bounds.
    #[inline(always)]
    pub fn place(&mut self, idx: usize, node: N) -> Option<N> {
        debug_assert!(idx < self.data.capacity());

        if self.mask(idx) {
            let p: &mut N = unsafe { self.data.get_unchecked_mut(idx) };
            let old = mem::replace(p, node);
            Some(old)
        } else {
            *self.mask_mut(idx) = true;
            self.size += 1;
            unsafe {
                let p = self.data.get_unchecked_mut(idx);
                // We must not drop the old content of `data[idx]`, as it was garbage.
                ptr::write(p, node);
            };

            None
        }
    }

    fn drop_items(&mut self) {
        let p = self.data.as_mut_ptr();
        if self.size*2 <= self.data.len() {
            Self::traverse_preorder_mut(self, 0, &mut 0, |this, _, idx| {
                unsafe {
                    // We know that `!is_nil(idx)`, therefore we must drop `*data[idx]` before dropping `data`.
                    *this.mask_mut(idx) = false;
                    ptr::drop_in_place(p.offset(idx as isize));
                }
            })
        } else {
            for i in 0..self.size {
                if self.mask(i) {
                    unsafe {
                        // We know that `!is_nil(i)`, therefore we must drop `*data[i]` before dropping `data`.
                        *self.mask_mut(i) = false;
                        ptr::drop_in_place(p.offset(i as isize));
                    }
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

    pub fn iter<'a>(&'a self) -> Iter<'a, N> {
        Iter::new(self)
    }

    pub fn count_nodes(&self, root: usize) -> usize {
        let mut count = 0;
        TreeRepr::traverse_inorder(self, root, &mut count, |_, count, _| {
            *count += 1;
            false
        });

        count
    }

    #[inline]
    pub fn complete_height(&self) -> usize {
        depth_of(self.capacity())
    }
}


impl<N: Node> IntoIterator for TreeRepr<N> {
    type Item = (N::K, N::V);
    type IntoIter = IntoIter<N>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self)
    }
}



macro_rules! traverse_preorder_block {
    ($this:expr, $root:expr, $a:expr, $on_next:expr) => (
        if $this.is_nil($root) {
            return;
        }

        let mut next = $root;

        loop {
            next = {
                $on_next($this, $a, next);

                if $this.has_left(next) {
                    lefti(next)
                } else if $this.has_right(next) {
                    righti(next)
                } else {
                    loop {
                        let l_enclosing = {
                            let z = next + 2;

                            if z == z & (!z+1) {
                                0
                            } else {
                                left_enclosing(next+1)-1
                            }
                        };

                        if l_enclosing <= $root {
                            // done
                            return;
                        }

                        next = l_enclosing + 1; // right sibling
                        if !$this.is_nil(next) {
                            break;
                        }
                    }
                    next
                }
            };
        }
    )
}

macro_rules! traverse_inorder_block {
    ($this:expr, $from:expr, $root:expr, $a:expr, $on_next:expr) => (
        if $this.is_nil($root) {
            return;
        }

        let mut next = $from;

        loop {
            next = {
                let stop = $on_next($this, $a, next);
                if stop {
                    break;
                }

                if $this.has_right(next) {
                    $this.find_min(righti(next))
                } else {
                    let l_enclosing = left_enclosing(next+1);

                    if l_enclosing <= $root+1 {
                        // done
                        break;
                    }

                    parenti(l_enclosing-1)
                }
            }
        }
    )
}

macro_rules! traverse_inorder_rev_block {
    ($this:expr, $root:expr, $a:expr, $on_next:expr) => (
        if $this.is_nil($root) {
            return;
        }

        let mut next = $this.find_max($root);

        loop {
            next = {
                $on_next($this, $a, next);

                if $this.has_left(next) {
                    $this.find_max(lefti(next))
                } else {
                    let r_enclosing = right_enclosing(next);

                    if r_enclosing <= $root {
                        // done
                        return;
                    }

                    parenti(r_enclosing)
                }
            }
        }
    )
}

pub trait Traverse<N: Node> {
    #[inline(always)]
    fn traverse_preorder<'a, A, F>(tree: &'a TreeRepr<N>, root: usize, a: &mut A, mut on_next: F)
        where F: FnMut(&'a TreeRepr<N>, &mut A, usize)
    {
        traverse_preorder_block!(tree, root, a, on_next);
    }

    #[inline(always)]
    fn traverse_inorder<'a, A, F>(tree: &'a TreeRepr<N>, root: usize, a: &mut A, on_next: F)
        where F: FnMut(&'a TreeRepr<N>, &mut A, usize) -> bool
    {
        TreeRepr::traverse_inorder_from(tree, tree.find_min(root), root, a, on_next)
    }

    fn traverse_inorder_from<'a, A, F>(tree: &'a TreeRepr<N>, from: usize, root: usize, a: &mut A, mut on_next: F)
        where F: FnMut(&'a TreeRepr<N>, &mut A, usize) -> bool
    {
        traverse_inorder_block!(tree, from, root, a, on_next);
    }

    fn traverse_inorder_rev<'a, A, F>(tree: &'a TreeRepr<N>, root: usize, a: &mut A, mut on_next: F)
        where F: FnMut(&'a TreeRepr<N>, &mut A, usize)
    {
        traverse_inorder_rev_block!(tree, root, a, on_next);
    }
}

pub trait TraverseMut<N: Node>: Traverse<N> {
    fn traverse_preorder_mut<'a, A, F>(tree: &'a mut TreeRepr<N>, root: usize, a: &mut A, mut on_next: F)
        where for<'b> F: FnMut(&'b mut TreeRepr<N>, &mut A, usize)
    {
        traverse_preorder_block!(tree, root, a, on_next);
    }

    #[inline(always)]
    fn traverse_inorder_mut<'a, A, F>(tree: &'a mut TreeRepr<N>, root: usize, a: &mut A, on_next: F)
        where for<'b> F: FnMut(&'b mut TreeRepr<N>, &mut A, usize) -> bool
    {
        let from = tree.find_min(root);
        Self::traverse_inorder_from_mut(tree, from, root, a, on_next)
    }

    fn traverse_inorder_from_mut<'a, A, F>(tree: &'a mut TreeRepr<N>, from: usize, root: usize, a: &mut A, mut on_next: F)
        where for<'b> F: FnMut(&'b mut TreeRepr<N>, &mut A, usize) -> bool
    {
        traverse_inorder_block!(tree, from, root, a, on_next);
    }


    fn traverse_inorder_rev_mut<'a, A, F>(tree: &'a mut TreeRepr<N>, root: usize, a: &mut A, mut on_next: F)
        where for<'b> F: FnMut(&'b mut TreeRepr<N>, &mut A, usize)
    {
        traverse_inorder_rev_block!(tree, root, a, on_next);
    }
}

impl<N: Node> Traverse<N> for TreeRepr<N> {}
impl<N: Node> TraverseMut<N> for TreeRepr<N> {}



impl<N: Node> Refill for TreeRepr<N> where N::K: Copy, N::V: Copy {
    fn refill(&mut self, master: &TreeRepr<N>) {
        let len = self.data.len();
        debug_assert!(len == master.data.len());
        unsafe {
            ptr::copy_nonoverlapping(master.data.as_ptr(), self.data.as_mut_ptr(), len);
            ptr::copy_nonoverlapping(master.mask.as_ptr(), self.mask.as_mut_ptr(), len);
        }
        self.size = master.size;
    }
}


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





pub struct Iter<'a, N: Node> where N: 'a, N::K: 'a, N::V: 'a {
    tree: &'a TreeRepr<N>,
    next_idx: usize,
    remaining: usize
}

impl <'a, N: Node> Iter<'a, N> where N::K: 'a, N::V: 'a {
    fn new(tree: &'a TreeRepr<N>) -> Iter<'a, N> {
        let next_idx = tree.find_min(0);
        Iter { tree:tree, next_idx:next_idx, remaining:tree.size }
    }
}

impl<'a, N: Node> Iterator for Iter<'a, N> where N: 'a, N::K: 'a, N::V: 'a {
    type Item = &'a Entry<N::K, N::V>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.next_idx;
        if self.tree.is_nil(curr) {
            None
        } else {
            self.next_idx =
                self.tree.successor(curr)
                    .map_or_else(|| self.tree.data.capacity(), |x| x);
            self.remaining -= 1;
            Some(self.tree.node(curr).deref())
        }
    }


    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, N: Node> ExactSizeIterator for Iter<'a, N> {}


pub struct IntoIter<N: Node> {
    tree: TreeRepr<N>,
    next_idx: usize
}

impl <N: Node> IntoIter<N> {
    pub fn new(tree: TreeRepr<N>) -> Self {
        let next_idx = tree.find_min(0);
        IntoIter { tree:tree, next_idx:next_idx }
    }
}

impl<N: Node> Iterator for IntoIter<N> {
    type Item = (N::K, N::V);

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.next_idx;
        let done = self.tree.data.capacity();
        if curr == done {
            None
        } else {
            debug_assert!(!self.tree.is_nil(curr));
            self.next_idx =
                self.tree.successor(self.next_idx)
                    .map_or_else(|| done, |x| x);

            Some(self.tree.take(curr).into_tuple())
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.tree.size, Some(self.tree.size))
    }
}

impl<N: Node> ExactSizeIterator for IntoIter<N> {}




impl<N: Node> Drop for TreeRepr<N> {
    fn drop(&mut self) {
        self.drop_items();
        unsafe {
            // the above call drops all contents of data, what remains is to drop the storage
            self.data.set_len(0)
        }
    }
}

impl<N: Node> Debug for TreeRepr<N> where N: Debug {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let mut nz: Vec<_> = self.mask.iter().enumerate()
            .rev()
            .skip_while(|&(_, flag)| !flag)
            .map(|(i, &flag)| match (self.node(i), flag) {
                (_, false) => String::from("X"),
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



#[inline]
pub fn inorder_to_idx_n(inorder: usize, complete_tree_n: usize) -> usize {
    debug_assert!(inorder < complete_tree_n, "inorder = {}, complete_tree_n = {}", inorder, complete_tree_n);

    let offs = complete_tree_n + 1;
    a025480(offs+inorder) - 1
}

// We use this sequence to implement inorder_to_idx: http://oeis.org/A025480
#[inline]
fn a025480(k: usize) -> usize {
    let shift = (!k).trailing_zeros();
    k >> (shift+1)
}



#[cfg(test)]
mod tests {
    use applied::plain_tree::PlTree;

    #[test]
    fn test_a025480() {
        let a = &super::a025480;
        assert_eq!(a(0), 0);
        assert_eq!(a(1), 0);
        assert_eq!(a(2), 1);
        assert_eq!(a(3), 0);
        assert_eq!(a(4), 2);
        assert_eq!(a(5), 1);
        assert_eq!(a(6), 3);
        assert_eq!(a(7), 0);
        assert_eq!(a(8), 4);
        assert_eq!(a(9), 2);
        assert_eq!(a(10), 5);
    }

    #[test]
    fn test_inorder_to_idx_exhaustive() {
        for h in 1..18 {
            let n = (1 << h) - 1;
            let items = (0..n).map(|x| (x, ())).collect();
            let tree = PlTree::new(items);

            for i in 0..n {
                let idx = super::inorder_to_idx_n(i, n);
                assert_eq!(tree.key(idx), &i);
            }
        }
    }
}



#[cfg(all(feature = "unstable", test))]
mod bench {
    extern crate test;

    use applied::plain_tree::{PlTree, PlNode};
    use self::test::Bencher;

    use base::{TreeRepr};
    use base::node::Node;

    type Nd = PlNode<usize, ()>;
    type Tree = TreeRepr<Nd>;

    fn bench_build(n: usize, bencher: &mut Bencher) {
        let mut items = (0..n)
            .map(|x| (x,()))
            .collect::<Vec<_>>();

        let mut data = Vec::with_capacity(n);
        let mut mask = Vec::with_capacity(n);

        bencher.iter(|| {
            let height = TreeRepr::<Nd>::build_complete(&mut items, 0, &mut data, &mut mask, &|item| Nd::from_tuple(item));
            test::black_box(height);
        });
    }

    #[bench]
    fn bench_build_00010000(bencher: &mut Bencher) {
        bench_build(10_000, bencher);
    }

    #[bench]
    fn bench_build_00020000(bencher: &mut Bencher) {
        bench_build(20_000, bencher);
    }

    #[bench]
    fn bench_build_00030000(bencher: &mut Bencher) {
        bench_build(30_000, bencher);
    }

    #[bench]
    fn bench_build_00040000(bencher: &mut Bencher) {
        bench_build(40_000, bencher);
    }

    #[bench]
    fn bench_build_00050000(bencher: &mut Bencher) {
        bench_build(50_000, bencher);
    }


    #[bench]
    fn bench_build_01000000(bencher: &mut Bencher) {
        bench_build(1_000_000, bencher);
    }

    #[bench]
    fn bench_build_02000000(bencher: &mut Bencher) {
        bench_build(2_000_000, bencher);
    }

    #[bench]
    fn bench_build_03000000(bencher: &mut Bencher) {
        bench_build(3000000, bencher);
    }

    #[bench]
    fn bench_build_04000000(bencher: &mut Bencher) {
        bench_build(4000000, bencher);
    }

    #[bench]
    fn bench_build_05000000(bencher: &mut Bencher) {
        bench_build(5000000, bencher);
    }

    #[bench]
    fn bench_build_06000000(bencher: &mut Bencher) {
        bench_build(6000000, bencher);
    }

    #[bench]
    fn bench_build_07000000(bencher: &mut Bencher) {
        bench_build(7000000, bencher);
    }

    #[bench]
    fn bench_build_08000000(bencher: &mut Bencher) {
        bench_build(8000000, bencher);
    }

    #[bench]
    fn bench_build_09000000(bencher: &mut Bencher) {
        bench_build(9000000, bencher);
    }
}
