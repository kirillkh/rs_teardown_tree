//#![feature(specialization)]
//#![feature(unique)]
#![cfg_attr(feature = "unstable", feature(test))]

//#![cfg_attr(test, feature(plugin))]
//#![cfg_attr(test, plugin(quickcheck_macros))]
#[cfg(test)] #[macro_use] extern crate quickcheck;


extern crate rand;

mod base;
mod applied;
mod external_api;

mod rust_bench;

pub use self::external_api::{IntervalTeardownTreeMap, IntervalTeardownTreeSet, Interval, KeyInterval, TeardownTreeMap, TeardownTreeSet, TeardownTreeRefill};
pub use self::base::{Traverse, ItemFilter, NoopFilter};
pub use self::base::util;



#[cfg(test)]
mod test_plain {
    use base::{Node, ItemFilter, NoopFilter, Traverse, lefti, righti};
    use base::util::make_teardown_seq;
    use base::validation::{check_bst_del_range, check_integrity_del_range};
    use applied::plain_tree::{PlTree, PlNode};
    use applied::interval::{Interval, KeyInterval};
    use external_api::{TeardownTreeSet, TreeWrapperAccess};
    use super::check_output_sorted;

    use rand::{Rng, XorShiftRng, SeedableRng};
    use std::cmp;
    use std::fmt::Debug;

    type Nd = PlNode<usize, ()>;
    type Tree = PlTree<usize, ()>;


    #[test]
    fn build() {
        TeardownTreeSet::new(vec![1]);
        TeardownTreeSet::new(vec![1, 2]);
        TeardownTreeSet::new(vec![1, 2, 3]);
        TeardownTreeSet::new(vec![1, 2, 3, 4]);
        TeardownTreeSet::new(vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn delete_range1() {
        delete_range_n(1);
    }

    #[test]
    fn delete_range2() {
        delete_range_n(1);
    }

    #[test]
    fn delete_range3() {
        delete_range_n(1);
    }

    #[test]
    fn delete_range4() {
        delete_range_n(4);
    }


    fn delete_range_n(n: usize) {
        let tree = Tree::new((1..n+1)
            .zip( vec![(); n].into_iter() )
            .collect::<Vec<_>>());
        delete_range_exhaustive_with_tree(tree);
    }


    fn test_prebuilt(items: &[usize], range: Range<usize>) {
        let nodes: Vec<Option<Nd>> = mk_prebuilt(items);
        let tree = Tree::with_nodes(nodes);
        let mut output = Vec::with_capacity(tree.size());
        check_tree(&mut TeardownTreeSet::from_internal(tree), range, NoopFilter, &mut output);
    }

    #[test]
    fn delete_range_prebuilt() {
        test_prebuilt(&[1], 1..2);

        test_prebuilt(&[1], 1..1);

        test_prebuilt(&[1, 0, 2], 1..1);

        test_prebuilt(&[1, 0, 2], 2..2);

        test_prebuilt(&[3, 2, 0, 1], 1..3);

        test_prebuilt(&[3, 2, 0, 1], 2..4);

        test_prebuilt(&[3, 2, 4, 1], 1..3);

        test_prebuilt(&[3, 1, 4, 0, 2], 2..4);

        test_prebuilt(&[4, 2, 0, 1, 3], 3..4);


        test_prebuilt(&[2, 2, 2, 1], 2..3);


        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], 1..1);

        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], 2..2);

        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], 3..3);

        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], 4..4);

        test_prebuilt(&[1, 0, 3, 0, 0, 2, 4], 1..2);


        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], 1..1);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], 2..2);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], 3..3);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], 4..4);

        test_prebuilt(&[1, 0, 4, 0, 0, 2, 0, 0, 0, 0, 0, 0, 3], 1..4);

        test_prebuilt(&[6, 4, 0, 1, 5, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3], 4..6);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 3], 1..1);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4], 1..2);
    }


    fn mk_prebuilt(items: &[usize]) -> Vec<Option<Nd>> {
        let nodes: Vec<_> = items.iter().map(|&x| if x==0 {
            None
        } else {
            Some(Node::new(x, ()))
        }).collect();

        nodes
    }




    use std::ops::Range;

    #[derive(Debug)]
    struct TreeRangeInfo {
        range: Range<usize>,
        root_idx: usize
    }


    #[test]
    fn delete_range_exhaustive() {
        for i in 1..8 {
            delete_range_exhaustive_n(i);
        }
    }

    #[test]
    fn delete_single_exhaustive() {
        for i in 1..8 {
            delete_single_exhaustive_n(i);
        }
    }

    fn delete_single_exhaustive_n(n: usize) {
        test_exhaustive_n(n, &|tree| delete_single_exhaustive_with_tree(tree));
    }

    fn delete_range_exhaustive_n(n: usize) {
        test_exhaustive_n(n, &|tree| delete_range_exhaustive_with_tree(tree));
    }

    fn test_exhaustive_n<F>(n: usize, check: &F)
                        where F: Fn(Tree) -> () {
        let elems: Vec<_> = (1..n+1).collect();
        println!("exhaustive {}: elems={:?} ------------------------", n, &elems);

        let mut stack = vec![TreeRangeInfo { range: (1..n+1), root_idx: 0 }];
        let mut items: Vec<usize> = vec![0; 1 << n];
        test_exhaustive_rec(&mut stack, &mut items, check);
    }

    fn test_exhaustive_rec<F>(stack: &mut Vec<TreeRangeInfo>, items: &mut Vec<usize>, check: &F)
                                                            where F: Fn(Tree) -> () {
        if stack.is_empty() {
            let nodes: Vec<Option<Nd>> = mk_prebuilt(items);
            let tree = Tree::with_nodes(nodes);
            check(tree);
        } else {
            let info = stack.pop().unwrap();
            let (lefti, righti) = (lefti(info.root_idx), righti(info.root_idx));
            for i in info.range.clone() {
                items[info.root_idx] = i;

                let mut pushed = 0;
                if info.range.start < i {
                    let range1 = info.range.start .. i;
                    stack.push(TreeRangeInfo { range: range1, root_idx: lefti });
                    pushed += 1;
                }

                if i+1 < info.range.end {
                    let range2 = i+1 .. info.range.end;
                    stack.push(TreeRangeInfo { range: range2, root_idx: righti });
                    pushed += 1;
                }

                test_exhaustive_rec(stack, items, check);

                for _ in 0..pushed {
                    stack.pop();
                }
            }

            items[info.root_idx] = 0;
            stack.push(info);
        }
    }


    fn delete_single_exhaustive_with_tree(tree: Tree) {
        let n = tree.size();
        let mut output = Vec::with_capacity(n);
        for i in 1..n+1 {
            output.truncate(0);
            let mut tree_mod = tree.clone();
//                println!("tree={:?}, from={}, to={}", &tree, i, j);
            let deleted = tree_mod.delete(&i);
            assert!(deleted.is_some());
            output.push(i);
            delete_range_check(n, i..i+1, &mut output, tree_mod, &tree, &NoopFilter);
        }
    }

    fn delete_range_exhaustive_with_tree(tree: Tree) {
        let n = tree.size();
        let mut output = Vec::with_capacity(n);
        for i in 0..n+2 {
            for j in i..n+2 {
                let mut tree_mod = tree.clone();
//                println!("tree={:?}, from={}, to={}, {}", &tree, i, j, &tree);
                output.truncate(0);
                tree_mod.delete_range(i..j, &mut output);
                let output = super::conv_from_tuple_vec(&mut output);
                delete_range_check(n, i..j, output, tree_mod, &tree, &NoopFilter);
            }
        }
    }

    fn delete_range_check<Flt>(n: usize, range: Range<usize>, output: &mut Vec<usize>, tree_mod: Tree, tree_orig: &Tree, filter: &Flt)
        where Flt: ItemFilter<usize>+Debug
    {
        let expected_range = cmp::max(1, range.start) .. cmp::min(n+1, range.end);

        assert_eq!(output, &expected_range.collect::<Vec<_>>(), "tree_orig={}", tree_orig);
        assert!(tree_mod.size() + output.len() == n, "tree'={:?}, tree={}, tree_mod={}, sz={}, output={:?}, n={}", tree_orig, tree_orig, tree_mod, tree_mod.size(), output, n);

        check_bst_del_range(&range, &tree_mod, &output, &tree_orig, filter);
        check_integrity_del_range(&range, &tree_mod, output, &tree_orig, filter);
    }




    quickcheck! {
        fn quickcheck_plain_(xs: Vec<usize>, rm: Range<usize>) -> bool {
            let mut output = Vec::with_capacity(xs.len());
            check_plain_tree(xs, rm, &mut output)
        }
    }

    fn check_plain_tree(xs: Vec<usize>, rm: Range<usize>, output: &mut Vec<usize>) -> bool {
        let rm = if rm.start <= rm.end { rm } else {rm.end .. rm.start};

        let mut tree = TeardownTreeSet::new(xs);
        check_tree(&mut tree, rm, NoopFilter, output);
        true
    }

    fn check_tree<Flt>(orig: &mut TeardownTreeSet<usize>, rm: Range<usize>, mut filter: Flt,
                       output: &mut Vec<usize>) -> TeardownTreeSet<usize>
        where Flt: ItemFilter<usize>+Clone+Debug
    {
        let mut tree = orig.clone();
        tree.filter_range(rm.clone(), filter.clone(), output);

        {
            let (tree, orig): (&mut Tree, &mut Tree) = (tree.internal(), orig.internal());
            check_bst_del_range(&rm, tree, output, orig, &filter);
            check_integrity_del_range(&rm, tree, output, orig, &filter);
            check_output_overlaps(&rm, tree, output, orig, &filter);
            check_tree_doesnt_overlap(&rm, tree, &mut filter);

            assert!(output.len() + tree.size() == orig.size());

            check_output_sorted(output, orig, &rm);
        }

        tree
    }


    #[derive(Clone, Debug)]
    struct SetRefFilter<'a> {
        set: &'a TeardownTreeSet<usize>
    }

    impl<'a> SetRefFilter<'a> {
        pub fn new(set: &'a TeardownTreeSet<usize>) -> Self {
            SetRefFilter { set: set }
        }
    }

    impl<'a> ItemFilter<usize> for SetRefFilter<'a> {
        fn accept(&mut self, key: &usize) -> bool { self.set.contains(key) }
        fn is_noop() -> bool { false }
    }

    fn full_teardown_filter_n(n: usize, rm_items: usize, flt_items: usize) {
        assert!(flt_items <= n);
        let mut rng = XorShiftRng::from_seed([96511, 42, 1423, 51984]);
        let elems: Vec<_> = (0..n).collect();
        let ranges: Vec<Range<usize>> = make_teardown_seq(n, rm_items, &mut rng);
        let mut flt_elems: Vec<_> = elems.clone();

        for i in 0..(n-flt_items) {
            let pos = rng.gen_range(0, n-flt_items-i);
            flt_elems.swap_remove(pos);
        }

        let mut orig = TeardownTreeSet::new(elems);
        let mut output = Vec::with_capacity(orig.size());
        let flt_tree = TeardownTreeSet::new(flt_elems);

        for range in ranges.into_iter() {
            output.truncate(0);
            orig = check_tree(&mut orig, range, SetRefFilter::new(&flt_tree), &mut output);
        }
    }

    #[test]
    fn test_full_teardown_filter() {
//        for i in 1..260 {
//            for j in 1..i {
//                println!("ij = {} {}", i, j);
//                for k in 0..i {
////                    println!("ijk = {} {} {}", i, j, k);
//                    full_teardown_filter_n(i, j, k);
//                }
//            }
//        }


        full_teardown_filter_n(3, 2, 2);

        full_teardown_filter_n(3, 2, 3);

        full_teardown_filter_n(3, 2, 2);

        full_teardown_filter_n(4, 2, 3);

        full_teardown_filter_n(4, 3, 2);

        full_teardown_filter_n(4, 3, 3);

        full_teardown_filter_n(5, 2, 0);
        full_teardown_filter_n(5, 2, 2);
        full_teardown_filter_n(5, 2, 5);

        full_teardown_filter_n(6, 3, 2);

        full_teardown_filter_n(8, 3, 5);

        full_teardown_filter_n(10, 3, 0);
        full_teardown_filter_n(10, 3, 5);
        full_teardown_filter_n(10, 3, 10);

        full_teardown_filter_n(15, 3, 12);

        full_teardown_filter_n(259, 3, 0);
        full_teardown_filter_n(259, 3, 123);
        full_teardown_filter_n(259, 3, 259);

        full_teardown_filter_n(1598, 21, 0);
        full_teardown_filter_n(1598, 21, 711);
        full_teardown_filter_n(1598, 21, 1598);

        full_teardown_filter_n(65918, 7347, 1965);
        full_teardown_filter_n(88165, 9664, 1);
        full_teardown_filter_n(196561, 81669, 97689);
        full_teardown_filter_n(756198, 247787, 17);
    }


    fn check_output_overlaps<Flt>(search: &Range<usize>, tree: &Tree, output: &Vec<usize>, tree_orig: &Tree, filter: &Flt)
        where Flt: ItemFilter<usize>+Debug
    {
        let search = KeyInterval::from_range(search);
        for (_, &x) in output.iter().enumerate() {
            let iv = KeyInterval::new(x,x);
            assert!(search.overlaps(&iv), "search={:?}, output={:?}, tree={:?}, flt={:?}, orig={:?}, {}", search, output, tree, filter, tree_orig, tree_orig);
        }
    }

    fn check_tree_doesnt_overlap<Flt>(search: &Range<usize>, tree: &mut Tree, flt: &mut Flt)
        where Flt: ItemFilter<usize>
    {
        tree.traverse_inorder(0, &mut (), |this, _, idx| {
            let &x = this.key(idx);
            assert!((x<search.start || search.end<=x) || !flt.accept(&x), "idx={}, key(idx)={:?}, search={:?}, tree={:?}, {}", idx, x, search, this, this);
            false
        });
    }
}





#[cfg(test)]
mod test_interval {
    use std::ops::{Range};
    use rand::{Rng, XorShiftRng, SeedableRng};
    use std::cmp;
    use std::fmt::Debug;

    use base::{Traverse, Node, ItemFilter, NoopFilter, parenti, lefti, righti};
    use base::validation::{check_bst_del_range, check_integrity_del_range, gen_tree_keys};
    use base::util::make_teardown_seq;
    use applied::interval::{Interval, IvNode, KeyInterval};
    use applied::interval_tree::{IvTree};
    use external_api::{IntervalTeardownTreeSet, TreeWrapperAccess};
    use super::check_output_sorted;

    type Iv = KeyInterval<usize>;
    type Tree = IvTree<Iv, ()>;


    //---- quickcheck overlap ----------------------------------------------------------------------
    quickcheck! {
        fn quickcheck_interval_delete_overlap(xs: Vec<Range<usize>>, rm: Range<usize>) -> bool {
            let mut rng = XorShiftRng::from_seed([3, 1, 4, 15]);
            test_random_shape_overlap(xs, rm, &mut rng)
        }
    }

    fn test_shape_delete_overlap<Flt>(xs: Vec<Range<usize>>, filter: Flt, rm: Range<usize>)
        where Flt: ItemFilter<KeyInterval<usize>>+Clone+Debug
    {
        test_shape(xs, filter, |tree: &mut IntervalTeardownTreeSet<Iv>, filter, output| {
            check_delete_overlap(tree, KeyInterval::from_range(&rm), filter, output);
        });
    }


    #[test]
    fn prebuilt_shape_overlap() {
        test_shape_delete_overlap(vec![1..1, 0..2], NoopFilter, 0..0);
        test_shape_delete_overlap(vec![1..1, 0..0, 2..2], SetFilter::new(vec![2..2, 1..1]), 0..2);
    }

    #[test]
    fn prebuilt_random_shape_overlap() {
        let rng = &mut XorShiftRng::from_seed([3, 1, 4, 15]);

        test_random_shape_overlap(vec![0..0], 0..0, rng);
        test_random_shape_overlap(vec![0..2, 1..1], 0..0, rng);
        test_random_shape_overlap(vec![0..0, 0..0, 0..1], 0..1, rng);
        test_random_shape_overlap(vec![0..0, 1..1, 2..2], 0..1, rng);

        test_random_shape_overlap(vec![1..1, 0..0, 0..0, 0..0], 0..1, rng);
        test_random_shape_overlap(vec![0..0, 1..1, 0..0, 0..0], 0..1, rng);
        test_random_shape_overlap(vec![0..0, 0..0, 1..1, 0..0], 0..1, rng);
        test_random_shape_overlap(vec![0..0, 0..0, 0..0, 1..1], 0..1, rng);
        test_random_shape_overlap(vec![1..1, 1..1, 1..1, 1..1], 0..1, rng);

        test_random_shape_overlap(vec![0..2, 1..2, 1..1, 1..2], 1..2, rng);
        test_random_shape_overlap(vec![0..2, 0..2, 2..0, 1..2, 0..2, 1..2, 0..2, 0..2, 1..0, 1..2], 1..2, rng);
        test_random_shape_overlap(vec![0..2, 1..1, 0..2, 0..2, 1..2, 1..2, 1..2, 0..2, 1..2, 0..2], 1..2, rng);
    }


    fn check_delete_overlap<Flt>(orig: &mut IntervalTeardownTreeSet<KeyInterval<usize>>, rm: KeyInterval<usize>, mut filter: Flt,
                                 output: &mut Vec<KeyInterval<usize>>) -> IntervalTeardownTreeSet<KeyInterval<usize>>
        where Flt: ItemFilter<KeyInterval<usize>>+Clone+Debug
    {
        let mut tree = orig.clone();
        tree.filter_overlap(&rm, filter.clone(), output);

        {
            let (tree, orig): (&mut Tree, &mut Tree) = (tree.internal(), orig.internal());
            check_bst_del_range(&rm, tree, &output, orig, &filter);
            check_integrity_del_range(&rm.to_range(), tree, output, orig, &filter);
            check_output_overlaps(&rm, tree, &output, orig, &filter);
            check_tree_doesnt_overlap(&rm, tree, &mut filter);

            assert!(output.len() + tree.size() == orig.size());

            if tree.size() > 0 {
                check_maxb(orig, tree, 0);
            }

            check_output_sorted(&output, orig, &rm.to_range());
        }

        tree
    }

    fn check_output_overlaps<Flt>(search: &Iv, tree: &Tree, output: &Vec<Iv>, tree_orig: &Tree, filter: &Flt)
        where Flt: ItemFilter<KeyInterval<usize>>+Debug
    {
        for (_, iv) in output.iter().enumerate() {
            assert!(search.overlaps(iv), "search={:?}, output={:?}, tree={:?}, flt={:?}, orig={:?}, {}", search, output, tree, filter, tree_orig, tree_orig);
        }
    }

    fn check_tree_doesnt_overlap<Flt>(search: &Iv, tree: &mut Tree, flt: &mut Flt)
        where Flt: ItemFilter<KeyInterval<usize>>
    {
        tree.traverse_inorder(0, &mut (), |this, _, idx| {
            assert!(!this.key(idx).overlaps(search) || !flt.accept(this.key(idx)), "idx={}, key(idx)={:?}, search={:?}, tree={:?}, {}", idx, this.key(idx), search, this, this);
            false
        });
    }

    fn test_random_shape_overlap(xs: Vec<Range<usize>>, rm: Range<usize>, rng: &mut XorShiftRng) -> bool {
        let rm = normalize_range(rm);
        let mut output = Vec::with_capacity(xs.len());
        test_random_shape(xs, rng, |tree| { check_delete_overlap(tree, rm.clone().into(), NoopFilter, &mut output); } )
    }

    //---- quickcheck single -----------------------------------------------------------------------
    quickcheck! {
        fn quickcheck_interval_delete_single(xs: Vec<Range<usize>>, rm: usize) -> bool {
            let mut rng = XorShiftRng::from_seed([561, 92881, 12, 562453]);
            test_random_shape_single(xs, rm, &mut rng)
        }
    }

    #[test]
    fn prebuilt_random_shape_single() {
        let rng = &mut XorShiftRng::from_seed([3, 1, 4, 15]);

        test_random_shape_single(vec![0..0], 0, rng);
        test_random_shape_single(vec![0..2, 1..1], 0, rng);
        test_random_shape_single(vec![0..0, 0..0, 0..1], 2, rng);
        test_random_shape_single(vec![0..0, 1..1, 2..2], 1, rng);

        test_random_shape_single(vec![1..1, 0..0, 0..0, 0..0], 0, rng);
        test_random_shape_single(vec![1..1, 0..0, 0..0, 0..0], 1, rng);
        test_random_shape_single(vec![1..1, 0..0, 0..0, 0..0], 2, rng);
        test_random_shape_single(vec![1..1, 0..0, 0..0, 0..0], 3, rng);
        test_random_shape_single(vec![1..1, 1..1, 1..1, 1..1], 3, rng);

        test_random_shape_single(vec![0..2, 1..2, 1..1, 1..2], 1, rng);
        test_random_shape_single(vec![0..2, 0..2, 2..0, 1..2, 0..2, 1..2, 0..2, 0..2, 1..0, 1..2], 3, rng);
        test_random_shape_single(vec![0..2, 1..1, 0..2, 0..2, 1..2, 1..2, 1..2, 0..2, 1..2, 0..2], 4, rng);
    }


    fn test_random_shape_single(xs: Vec<Range<usize>>, rm: usize, rng: &mut XorShiftRng) -> bool {
        let iv = if !xs.is_empty() {
            xs[rm % xs.len()].clone().into()
        } else {
            KeyInterval::new(0, 1)
        };

        test_random_shape(xs, rng, |tree| {
            check_delete_single(tree, iv);
        });

        true
    }


    fn check_delete_single(orig: &mut IntervalTeardownTreeSet<KeyInterval<usize>>, rm: KeyInterval<usize>) -> IntervalTeardownTreeSet<KeyInterval<usize>>
    {
        let mut tree = orig.clone();

        let deleted = tree.delete(&rm);

        {
            let (tree, orig): (&mut Tree, &mut Tree) = (tree.internal(), orig.internal());
            check_bst_del_range(&rm, tree, &(), orig, &());
            check_integrity_del_range(&rm.to_range(), tree, &(), orig, &());
            assert!(deleted == orig.contains(&rm));
            assert!(deleted && tree.size()+1 == orig.size() || !deleted && tree.size() == orig.size());

            if tree.size() > 0 {
                check_maxb(orig, tree, 0);
            }
        }

        tree
    }



    //---- quickcheck helpers ----------------------------------------------------------------------
    use std::borrow::Borrow;
    fn normalize_range<R: Borrow<Range<usize>>>(r: R) -> Range<usize> {
        let r: &Range<usize> = r.borrow();
        cmp::min(r.start, r.end) .. cmp::max(r.start, r.end)
    }

    fn init_maxb(tree: &mut Tree, idx: usize) -> usize {
        assert!(!tree.is_nil(idx));

        let mut maxb = *tree.node(idx).key.b();
        if tree.has_left(idx) {
            maxb = cmp::max(maxb, init_maxb(tree, lefti(idx)));
        }
        if tree.has_right(idx) {
            maxb = cmp::max(maxb, init_maxb(tree, righti(idx)));
        }

        tree.node_mut(idx).maxb = maxb;
        maxb
    }


    fn test_random_shape<C>(xs: Vec<Range<usize>>, rng: &mut XorShiftRng, mut check: C) -> bool
        where C: FnMut(&mut IntervalTeardownTreeSet<Iv>)
    {
        let mut intervals = xs.into_iter()
            .map(|r| normalize_range(r).into())
            .collect::<Vec<_>>();
        intervals.sort();

        let tree = gen_tree(intervals, rng);

        check(&mut IntervalTeardownTreeSet::from_internal(tree));
        true
    }

    fn test_shape<Flt, C>(xs: Vec<Range<usize>>, filter: Flt, mut check: C)
        where Flt: ItemFilter<KeyInterval<usize>>+Clone+Debug,
              C: FnMut(&mut IntervalTeardownTreeSet<Iv>, Flt, &mut Vec<KeyInterval<usize>>)
    {
        let nodes = xs.into_iter()
                .map(|r| if r.start<=r.end {
                    Some(IvNode::new(Iv::new(r.start, r.end), ()))
                } else {
                    None
                }
            )
            .collect::<Vec<_>>();

        let mut internal = Tree::with_nodes(nodes);
        if internal.size() > 0 {
            init_maxb(&mut internal, 0);
        }

        let mut tree = IntervalTeardownTreeSet::from_internal(internal);
        let mut output = Vec::with_capacity(tree.size());
        check(&mut tree, filter, &mut output);
    }


    fn gen_tree(items: Vec<Iv>, rng: &mut XorShiftRng) -> Tree {
        let items: Vec<Option<Iv>> = gen_tree_keys(items, rng);
        let mut nodes = items.into_iter()
            .map(|opt| opt.map(|k| IvNode::new(k.clone(), ())))
            .collect::<Vec<_>>();
        for i in (1..nodes.len()).rev() {
            let maxb = if let Some(ref node) = nodes[i] {
                node.maxb as usize
            } else {
                continue
            };

            let parent = nodes[parenti(i)].as_mut().unwrap();
            parent.maxb = cmp::max(&parent.maxb, &maxb).clone();
        }
        Tree::with_nodes(nodes)
    }


    fn check_maxb(orig: &Tree, tree: &Tree, idx: usize) -> usize {
        assert!(!tree.is_nil(idx));

        let mut expected_maxb = *tree.node(idx).key.b();
        if tree.has_left(idx) {
            expected_maxb = cmp::max(expected_maxb, check_maxb(orig, tree, lefti(idx)));
        }
        if tree.has_right(idx) {
            expected_maxb = cmp::max(expected_maxb, check_maxb(orig, tree, righti(idx)));
        }

        assert!(expected_maxb==tree.node(idx).maxb, "expected maxb={}, actual maxb={}, idx={}, tree={:?}, orig={:?}, {}", expected_maxb, tree.node(idx).maxb, idx, tree, orig, orig);
        expected_maxb
    }



    //---- non-quickcheck --------------------------------------------------------------------------
    fn full_teardown_n(n: usize, rm_items: usize) {
        let mut rng = XorShiftRng::from_seed([96511, 42, 1423, 51984]);
        let elems: Vec<_> = (0..n).map(|x| (KeyInterval::new(x,x))).collect();
        let ranges: Vec<Range<usize>> = make_teardown_seq(n, rm_items, &mut rng);

        let mut orig = IntervalTeardownTreeSet::new(elems);
        let mut output = Vec::with_capacity(orig.size());

        for range in ranges.into_iter() {
            output.truncate(0);
            let rm = KeyInterval::from_range(&range);
            orig = check_delete_overlap(&mut orig, rm, NoopFilter, &mut output);
        }
        assert!(orig.size() == 0);
    }


    #[test]
    fn test_full_teardown() {
        full_teardown_n(5, 2);
        full_teardown_n(10, 3);
        full_teardown_n(259, 3);
        full_teardown_n(1598, 21);
        full_teardown_n(65918, 7347);
        full_teardown_n(88165, 9664);
        full_teardown_n(196561, 81669);
        full_teardown_n(756198, 247787);
    }


    #[derive(Clone, Debug)]
    struct SetRefFilter<'a> {
        set: &'a IntervalTeardownTreeSet<KeyInterval<usize>>
    }

    impl<'a> SetRefFilter<'a> {
        pub fn new(set: &'a IntervalTeardownTreeSet<KeyInterval<usize>>) -> Self {
            SetRefFilter { set: set }
        }
    }

    impl<'a> ItemFilter<KeyInterval<usize>> for SetRefFilter<'a> {
        fn accept(&mut self, key: &KeyInterval<usize>) -> bool { self.set.contains(key) }
        fn is_noop() -> bool { false }
    }


    #[derive(Clone, Debug)]
    struct SetFilter {
        set: IntervalTeardownTreeSet<KeyInterval<usize>>
    }

    impl SetFilter {
        pub fn new(xs: Vec<Range<usize>>) -> Self {
            let items = xs.into_iter()
                .map(|r| Iv::new(r.start, r.end))
                .collect::<Vec<_>>();

            let filter_set = IntervalTeardownTreeSet::new(items);
            SetFilter { set: filter_set }
        }
    }

    impl ItemFilter<KeyInterval<usize>> for SetFilter {
        fn accept(&mut self, key: &KeyInterval<usize>) -> bool { self.set.contains(key) }
        fn is_noop() -> bool { false }
    }



    fn full_teardown_filter_n(n: usize, rm_items: usize, flt_items: usize) {
        assert!(flt_items <= n);
        let mut rng = XorShiftRng::from_seed([96511, 42, 1423, 51984]);
        let elems: Vec<_> = (0..n).map(|x| (KeyInterval::new(x,x))).collect();
        let ranges: Vec<Range<usize>> = make_teardown_seq(n, rm_items, &mut rng);
        let mut flt_elems: Vec<_> = elems.clone();

        for i in 0..(n-flt_items) {
            let pos = rng.gen_range(0, n-flt_items-i);
            flt_elems.swap_remove(pos);
        }

        let mut orig = IntervalTeardownTreeSet::new(elems);
        let mut output = Vec::with_capacity(orig.size());
        let flt_tree = IntervalTeardownTreeSet::new(flt_elems);

        for range in ranges.into_iter() {
            output.truncate(0);
            let rm = KeyInterval::from_range(&range);
            orig = check_delete_overlap(&mut orig, rm, SetRefFilter::new(&flt_tree), &mut output);
        }
    }


    #[test]
    fn test_full_teardown_filter() {
//        for i in 1..260 {
//            for j in 1..i {
//                println!("ij = {} {}", i, j);
//                for k in 0..i {
//                    full_teardown_filter_n(i, j, k);
//                }
//            }
//        }


        full_teardown_filter_n(3, 2, 2);

        full_teardown_n(3, 2);
        full_teardown_filter_n(3, 2, 3);

        full_teardown_filter_n(3, 2, 2);

        full_teardown_filter_n(5, 2, 0);
        full_teardown_filter_n(5, 2, 2);
        full_teardown_filter_n(5, 2, 5);

        full_teardown_filter_n(10, 3, 0);
        full_teardown_filter_n(10, 3, 5);
        full_teardown_filter_n(10, 3, 10);

        full_teardown_filter_n(259, 3, 0);
        full_teardown_filter_n(259, 3, 123);
        full_teardown_filter_n(259, 3, 259);

        full_teardown_filter_n(1598, 21, 0);
        full_teardown_filter_n(1598, 21, 711);
        full_teardown_filter_n(1598, 21, 1598);

        full_teardown_filter_n(65918, 7347, 1965);
        full_teardown_filter_n(88165, 9664, 1);
        full_teardown_filter_n(196561, 81669, 97689);
        full_teardown_filter_n(756198, 247787, 17);
    }
}



use base::{Node, TreeDeref, TreeRepr};
use std::fmt::Debug;

#[cfg(test)]
fn conv_from_tuple_vec<K>(items: &mut Vec<(K, ())>) -> &mut Vec<K> {
    use std::mem;
    unsafe { mem::transmute(items) }
}

fn check_output_sorted<N: Node, Rm>(output: &Vec<N::K>, orig: &mut TreeDeref<N, Target=TreeRepr<N>>, rm: &Rm)
    where N::K: Debug, N: Debug, Rm: Debug
{
    for i in 1..output.len() {
        assert!(output[i-1] <= output[i], "output={:?}, rm={:?}, orig={:?}, {}", output, rm, orig.deref(), orig.deref());
    }
}
