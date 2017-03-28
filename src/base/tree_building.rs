use base::{Node, TreeRepr, parenti, lefti, righti, depth_of, TraverseMut};

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
    type Item = usize;

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
                    return Some(dst_idx)
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



struct RebuildIter {
    end: usize,
    start: usize,
    height: usize,
    complete_count: usize,
}

impl RebuildIter {
    pub fn new(count: usize) -> Self {
        let height =
            if count == 0 {
                0
            } else {
                depth_of(count - 1) + 1
            };
        let complete_count = (1 << height) - 1;

        RebuildIter { start: 0, end: count, height: height, complete_count: complete_count, }
    }

    pub fn relative_to(self, root: usize) -> GlobalBuildIter<Self> {
        GlobalBuildIter { root:root, local: self }
    }
}


impl Iterator for RebuildIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            let dst_idx = a025480(2*self.complete_count + 1 - self.end) - 1;
            self.end -= 1;
//            println!("complete_count={}, returning {} -> {}", self.complete_count, self.src_offs-1, dst_idx);
            Some(dst_idx)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.end - self.start;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for RebuildIter {}

impl DoubleEndedIterator for RebuildIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            self.start += 1;
            let dst_idx = a025480(2*self.complete_count + 1 - self.start) - 1;
//            println!("complete_count={}, returning {} -> {}", self.complete_count, self.src_offs-1, dst_idx);
            Some(dst_idx)
        }
    }
}



struct GlobalBuildIter<I: Iterator<Item=usize>> {
    root: usize,
    local: I
}

impl<I: Iterator<Item=usize>> GlobalBuildIter<I> {
    #[inline] fn to_global(&self, local_idx: Option<usize>) -> Option<usize> {
        local_idx.map(|local_idx| {
            let local_depth = depth_of(local_idx);
            local_idx + (self.root << local_depth)
        })
    }
}

impl<I: Iterator<Item=usize>> Iterator for GlobalBuildIter<I> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.local.next();
        self.to_global(next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.local.size_hint() }
}

impl<I: Iterator<Item=usize>> ExactSizeIterator for GlobalBuildIter<I> {}

impl<I: Iterator<Item=usize>+DoubleEndedIterator> DoubleEndedIterator for GlobalBuildIter<I> {
    fn next_back(&mut self) -> Option <Self::Item> {
        let next = self.local.next_back();
        self.to_global(next)
    }
}



struct GapIter {
    gap: usize,
    full_gap: usize,
    internal_nodes: usize,
    remainder: usize,
    iter: BuildIter,
}

impl GapIter {
    pub fn new(iter: BuildIter, count: usize) -> Self {
        let complete_count = iter.complete_count;
        let complete_leaves = (complete_count+1) >> 1;
        let internal_nodes = complete_count - complete_leaves;
        let leaves = count - internal_nodes;
        let nils = complete_leaves - leaves;

        let full_gap = nils / (leaves+1);
        let remainder = nils % (leaves+1);

        GapIter { gap:full_gap, full_gap:full_gap, internal_nodes:internal_nodes, remainder:remainder, iter:iter }
    }

    pub fn relative_to(self, root: usize) -> GlobalBuildIter<Self> {
        GlobalBuildIter { root:root, local: self }
    }
}


impl Iterator for GapIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().and_then(|next| {
            let skip =
                if next >= self.internal_nodes {
                    if self.gap == 0 {
                        self.gap = self.full_gap +
                            if self.remainder == 0 { 0 }
                            else { self.remainder -= 1; 1 };
                        false
                    } else {
                        self.gap -= 1;
                        true
                    }
                } else {
                    false
                };

            if skip {
                self.iter.next()
            } else {
                Some(next)
            }
        })
    }
}





// building and rebuilding
impl<N: Node> TreeRepr<N> {
    #[inline(never)]
    pub fn rebuild_subtree2(&mut self, root: usize, insert_offs: usize, count: usize, item: (N::K, N::V))
    {
        let capacity = self.capacity();

        // 1. compact the subtree at the bottom right corner of the complete subtree's storage
        let dst_iter = RebuildIter::new(count);

        let complete_subtree_height = dst_iter.height;
        let complete_tree_height = self.complete_height();
        let root_depth = depth_of(root);

        // dst_root is the root of the tree that is going to be the result of step 1
        let mut dst_root = root;
        for _ in 0 .. (complete_tree_height - complete_subtree_height) - root_depth {
            dst_root = righti(dst_root);
        }

//        println!("count={}, root={}, dst_root={}, subtree_height={}, sotrage_h={}, root_d={}", count, root, dst_root, subtree_height, storage_height, root_depth);

        let mut dst_iter = dst_iter.relative_to(dst_root).rev();

        let mut item = N::from_tuple(item);
        TreeRepr::traverse_inorder_rev_mut(self, root, &mut (), (), |this, _, src_idx| {
            let dst_idx = dst_iter.next().unwrap();
            unsafe {
                this.move_from_to(src_idx, dst_idx);
            }
            None
        });

        let src_root = dst_root;
        let mut src_iter = RebuildIter::new(count).relative_to(src_root);
        src_iter.next(); // TODO the first element is dummy

        let mut b_iter = BuildIter::new(count);
        b_iter.count = b_iter.complete_count;
        let mut dst_iter = GapIter::new(b_iter, count).relative_to(root);

        for i in 0..insert_offs {
            src_iter.next().and_then(|src_idx| {
                dst_iter.next().map(|dst_idx| {
                    unsafe {
                        self.move_from_to(src_idx, dst_idx);
                    }
                })
            });
        }

        dst_iter.next().map(|dst_idx| {
            debug_assert!(self.is_nil(dst_idx));
            self.place(dst_idx, item);
        });


        for i in insert_offs..count {
            src_iter.next().and_then(|src_idx| {
                dst_iter.next().map(|dst_idx| {
                    unsafe {
                        self.move_from_to(src_idx, dst_idx);
                    }
                })
            });
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


            let skip =
                if local_idx >= internal_nodes {
                    if gap == 0 {
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
                } else {
                    false
                };

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
    use super::RebuildIter;

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
        let mut iter = RebuildIter::new(1);
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(2);
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(3);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(4);
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(5);
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(6);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(7);
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(8);
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
        let mut iter = RebuildIter::new(1).rev();
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(2).rev();
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(3).rev();
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(4).rev();
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(5).rev();
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(6).rev();
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(7).rev();
        assert_eq!(iter.next(), Some(6));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);

        let mut iter = RebuildIter::new(8).rev();
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
