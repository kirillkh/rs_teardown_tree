use base::{Node, TreeRepr, parenti, wparenti, lefti, righti, depth_of, TraverseMut, right_enclosing, left_enclosing};

use std::mem;
use std::cmp::{max};
use std::ptr;
use std::iter::Map;


struct BuildIter {
    count: usize,
    complete_count: usize,

    skipped: usize,
    src_offs: usize,

}

impl BuildIter {
    pub fn new(count: usize) -> Self {
        let complete_count =
            if count == 0 {
                0
            } else {
                let height = depth_of(count - 1) + 1;
                (1 << height) - 1
            };

        BuildIter { count: count, complete_count: complete_count, skipped: 0, src_offs: 0 }
    }

    pub fn relative_to(self, root: usize) -> GlobalBuildIter<Self> {
        GlobalBuildIter { root:root, local: self }
    }
}


impl Iterator for BuildIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let (src_offs, count) = (self.src_offs, self.count);

        if src_offs == count {
            None
        } else {
            debug_assert!(src_offs < count);

            loop {
                let inorder = src_offs + self.skipped;
                debug_assert!(inorder < self.complete_count);
                let dst_idx = a025480(self.complete_count + 1 + inorder) - 1;


                if dst_idx < count {
                    self.src_offs += 1;
//                    println!("count={}, returning {} -> {}", count, self.src_offs-1, dst_idx);
                    return Some((dst_idx, depth_of(dst_idx)));
                } else {
//                    println!("count={}, skipping {} -> {}", count, self.src_offs-1, dst_idx);
                    self.skipped += 1;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count - self.src_offs;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for BuildIter {}


struct FwdRebuildIter {
    remaining: usize,
    complete_height: usize,
    complete_count: usize,

    next: usize,
    depth: usize,
}

impl FwdRebuildIter {
    pub fn new(count: usize) -> Self {
        let height =
            if count == 0 {
                0
            } else {
                depth_of(count - 1) + 1
            };
        let complete_count = (1 << height) - 1;
        let fst = a025480(2*complete_count + 1 - count) - 1;
        let depth = depth_of(fst);
        FwdRebuildIter { remaining: count, complete_height: height, complete_count: complete_count, next: fst, depth: depth }
    }

    #[inline] fn complete_subtree_min(root: usize, complete_subtree_height: usize) -> usize {
        ((root+1) << (complete_subtree_height-1)) - 1
    }

    pub fn relative_to(self, root: usize) -> GlobalBuildIter<Self> {
        GlobalBuildIter { root:root, local: self }
    }
}


impl Iterator for FwdRebuildIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            let ret = (self.next, self.depth);
            if self.depth + 1 == self.complete_height {
                let l_enclosing = left_enclosing(self.next+1);
                self.next = parenti(l_enclosing.wrapping_sub(1));
                self.depth = depth_of(self.next);
            } else {
                let right = righti(self.next);
                self.next = Self::complete_subtree_min(right, self.complete_height - 1 - self.depth);
                self.depth = self.complete_height - 1;
            }

            self.remaining -= 1;
            Some(ret)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for FwdRebuildIter {}





struct BckRebuildIter {
    remaining: usize,
    complete_height: usize,
    complete_count: usize,

    next: usize,
    depth: usize,
}

impl BckRebuildIter {
    pub fn new(count: usize) -> Self {
        let height =
            if count == 0 {
                0
            } else {
                depth_of(count - 1) + 1
            };
        let complete_count = (1 << height) - 1;

        let fst = Self::complete_subtree_max(0, height);
        Self { remaining: count, complete_height: height, complete_count: complete_count, next: fst, depth: height-1 }
    }

    #[inline] fn complete_subtree_max(root: usize, complete_subtree_height: usize) -> usize {
        let h = complete_subtree_height;
        (root << (h-1)) +
        ((1 << h) - 2)
    }

    pub fn relative_to(self, root: usize) -> GlobalBuildIter<Self> {
        GlobalBuildIter { root:root, local: self }
    }
}


impl Iterator for BckRebuildIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            let ret = (self.next, self.depth);
            if self.depth + 1 == self.complete_height {
                let r_enclosing = right_enclosing(self.next+1);
                self.next = wparenti(r_enclosing.wrapping_sub(1));
                self.depth = depth_of(self.next);
            } else {
                let left = lefti(self.next);
                self.next = Self::complete_subtree_max(left, self.complete_height - 1 - self.depth);
                self.depth = self.complete_height - 1;
            }

            self.remaining -= 1;
            Some(ret)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for BckRebuildIter {}






struct GlobalBuildIter<I: Iterator<Item=(usize, usize)>> {
    root: usize,
    local: I,
}

impl<I: Iterator<Item=(usize, usize)>> GlobalBuildIter<I> {
    #[inline] fn to_global(&self, local_idx: Option<(usize, usize)>) -> Option<usize> {
        local_idx.map(|(local_idx, depth)| {
            local_idx + (self.root << depth)
        })
    }
}

impl<I: Iterator<Item=(usize, usize)>> Iterator for GlobalBuildIter<I> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.local.next();
        self.to_global(next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.local.size_hint() }
}

impl<I: Iterator<Item=(usize, usize)>> ExactSizeIterator for GlobalBuildIter<I> {}




    // building and rebuilding
impl<N: Node> TreeRepr<N> {
    #[inline(never)]
    pub fn rebuild_subtree2(&mut self, root: usize, insert_offs: usize, count: usize, item: (N::K, N::V))
    {
        let capacity = self.capacity();

        // 1. rebuild the subtree at the bottom right corner of the complete subtree's storage
        let dst_iter = BckRebuildIter::new(count);

        let complete_subtree_height = dst_iter.complete_height;
        let complete_tree_height = self.complete_height();
        let root_depth = depth_of(root);

        // dst_root is the root of the tree that is going to be the result of step 1
        let mut dst_root = root;
        for _ in 0 .. (complete_tree_height - complete_subtree_height) - root_depth {
            dst_root = righti(dst_root);
        }

//        println!("count={}, root={}, dst_root={}, subtree_height={}, sotrage_h={}, root_d={}", count, root, dst_root, subtree_height, storage_height, root_depth);

        let mut dst_iter = dst_iter.relative_to(dst_root).enumerate();
        let rev_insert_offs = count - insert_offs - 1;

        let mut item = N::from_tuple(item);
        let mut from = TreeRepr::traverse_inorder_rev_mut(self, root, &mut item, capacity, |this, item, src_idx| {
            let (i, dst_idx) = dst_iter.next().unwrap();
//            println!("a src_idx={} dst_idx={}", src_idx, dst_idx);
//            println!("i={}, insert_offs={}, src_idx={}, dst_idx={}", i, insert_offs, src_idx, dst_idx);
            if i == rev_insert_offs {
                let next = this.take(src_idx);
                let prev = mem::replace(item, next);
                debug_assert!(this.is_nil(dst_idx));
                this.place(dst_idx, prev); // TODO we can take a shortcut here by not modifying the mask
                let pred = this.predecessor(src_idx);
//                println!("a1: {}", src_idx);
                pred
            } else {
                unsafe {
                    this.move_from_to(src_idx, dst_idx);
                }
                None
            }
        });

        if from != capacity && /* corner case above */ root <= from  {
//            println!("root={}, from={}", root, from);
            TreeRepr::traverse_inorder_rev_from_mut(self, from, root, &mut item, (), |this, item, src_idx| {
                let (_, dst_idx) = dst_iter.next().unwrap();
//                println!("b src_idx={} dst_idx={}", src_idx, dst_idx);
                let prev_item = mem::replace(item, this.take(src_idx));
                this.place(dst_idx, prev_item); // TODO we can cut the corner here by not modifying the mask
                None // TODO this can be made more efficient by breaking when dst_idx != src_idx
            });
        }


        // 2. move the subtree to the root
//        let mut src_level_start = dst_root;
//        let mut dst_level_start = root;
//        let mut level_n = 1;
//        for _ in 1..subtree_height {
//            for i in 0..level_n {
//                debug_assert!(!self.is_nil(src_level_start + i));
//                debug_assert!(self.is_nil(dst_level_start + i));
//                unsafe {
//                    self.move_from_to(src_level_start + i, dst_level_start + i);
//                }
//            }
//            src_level_start = lefti(src_level_start);
//            dst_level_start = lefti(dst_level_start);
//            level_n <<= 1;
//        }
//
//        level_n = count + 1 - level_n;
//        for i in 0..level_n {
//            unsafe {
//                self.move_from_to(src_level_start + i, dst_level_start + i);
//            }
//        }



//        let src_root = dst_root;
//        let mut dst_iter = BuildIter::new(count).relative_to(root);
//        TreeRepr::traverse_inorder_mut(self, src_root, &mut (), (), |this, _, src_idx| {
//            println!("stage 2 src_idx={}", src_idx);
//            let dst_idx = dst_iter.next().unwrap();
//            let item = this.take(src_idx);
//            debug_assert!(this.is_nil(dst_idx));
//            this.place(dst_idx, item);
//            None
//        });

        let src_root = dst_root;
        let mut src_iter = FwdRebuildIter::new(count).relative_to(src_root);
        let mut dst_iter = BuildIter::new(count).relative_to(root);
        let mut iter = src_iter.zip(dst_iter);

        let (_, fst) = iter.next().unwrap();
        debug_assert!(self.is_nil(fst));
        self.place(fst, item);

        for (src_idx, dst_idx) in iter {
//            println!("c src_idx={} dst_idx={}", src_idx, dst_idx);
            debug_assert!(!self.is_nil(src_idx), "src_idx={}, dst_idx={}", src_idx, dst_idx);
            unsafe {
                self.move_from_to(src_idx, dst_idx);
            }
        }
    }


    /// Returns the height of the tree.
    pub fn build_nearly_complete<T, I>(sorted: I, count: usize, root: usize, data: &mut [T], mask: &mut [bool]) -> usize
        where I: Iterator<Item=T>
    {
        let pdst = data.as_mut_ptr();
        let dst_iter = BuildIter::new(count);
        let height = (dst_iter.complete_count+1).trailing_zeros();
        let mut iter = dst_iter
            .relative_to(root)
            .zip(sorted);
        while let Some((global_idx, item)) = iter.next() {
            unsafe {
                ptr::write(pdst.offset(global_idx as isize), item);
                *mask.get_unchecked_mut(global_idx) = true;
            }
        }

        height as usize
    }

    /// Returns the height of the tree. This consumes the contents of `data`, so the caller must
    /// make sure the contents are never reused or dropped after this call returns.
    pub fn build_with_dispersed_gaps<T, U, G>(sorted: &[T], root: usize, data: &mut [U], mask: &mut [bool], convert: G) -> usize
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
//                    println!("setting mask[{}] = true, src_offs={}", global_idx, src_offs);
                    *mask.get_unchecked_mut(global_idx) = true;
                }
                src_offs += 1;
            } else {
                skipped += 1;
            }
        }

        height
    }


    pub fn layout_inorder_for_insert(&mut self, root: usize, dst: &mut Vec<N>, count: usize, item: (N::K, N::V)) {
        let mut inorder_offs: usize = 0;
        let mut prev_idx = 0;

        let psrc: *const N = self.data.as_ptr();
        let pdst: *mut N = unsafe { dst.as_mut_ptr().offset(dst.len() as isize) };

        Self::traverse_inorder_mut(self, root, &mut inorder_offs, (), |_, inorder_offs, idx| {
            unsafe {
                let node = ptr::read(psrc.offset(idx as isize));
                prev_idx = idx;
                if &item.0 < node.key() {
                    return Some(());
                }
                ptr::write(pdst.offset(*inorder_offs as isize), node);
            }
            *inorder_offs += 1;
            None
        });

        unsafe {
            ptr::write(pdst.offset(inorder_offs as isize), N::from_tuple(item));
        }
        inorder_offs += 1;

        if inorder_offs < count {
            Self::traverse_inorder_from_mut(self, prev_idx, root, &mut inorder_offs, (), |_, inorder_offs, idx| {
                unsafe {
                    let node = ptr::read(psrc.offset(idx as isize));
                    ptr::write(pdst.offset(*inorder_offs as isize), node);
                }
                *inorder_offs += 1;
                None
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
        Self::traverse_inorder_mut(self, root, &mut (), (), |this, _, idx| {
            *this.mask_mut(idx) = false;
            None
        });

        Self::build_with_dispersed_gaps(&inorder_items, root, &mut self.data, &mut self.mask, &|node| node);

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
        Self::build_with_dispersed_gaps(&mut inorder_items, 0, &mut self.data, &mut self.mask, &|node| node);

        mem::forget(inorder_items);
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
    use super::{FwdRebuildIter, BckRebuildIter};

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

    #[test]
    fn fwd_rebuild_iter() {
        let mut iter = FwdRebuildIter::new(1).relative_to(0);
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), None);

        let mut iter = FwdRebuildIter::new(2).relative_to(0);
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);

        let mut iter = FwdRebuildIter::new(3).relative_to(0);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);

        let mut iter = FwdRebuildIter::new(4).relative_to(0);
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), None);

        let mut iter = FwdRebuildIter::new(5).relative_to(0);
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), None);

        let mut iter = FwdRebuildIter::new(6).relative_to(0);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), None);

        let mut iter = FwdRebuildIter::new(7).relative_to(0);
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), None);

        let mut iter = FwdRebuildIter::new(8).relative_to(0);
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(11));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(12));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(13));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(14));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn bck_rebuild_iter() {
        let mut iter = BckRebuildIter::new(1).relative_to(0);
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), None);

        let mut iter = BckRebuildIter::new(2).relative_to(0);
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), None);

        let mut iter = BckRebuildIter::new(3).relative_to(0);
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), None);

        let mut iter = BckRebuildIter::new(4).relative_to(0);
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), None);

        let mut iter = BckRebuildIter::new(5).relative_to(0);
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), None);

        let mut iter = BckRebuildIter::new(6).relative_to(0);
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), None);

        let mut iter = BckRebuildIter::new(7).relative_to(0);
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);

        let mut iter = BckRebuildIter::new(8).relative_to(0);
        assert_eq!(iter.next(), Some(14));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(13));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(12));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(11));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), None);
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

        bencher.iter(move || {
            let height = TreeRepr::<Nd>::build_nearly_complete(items.iter().cloned(), n, 0, &mut data, &mut mask);
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
