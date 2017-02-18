//#![feature(specialization)]
//#![feature(unique)]
#![cfg_attr(feature = "unstable", feature(test))]

//#![cfg_attr(test, feature(plugin))]
//#![cfg_attr(test, plugin(quickcheck_macros))]
#[cfg(test)] #[macro_use] extern crate quickcheck;


extern crate rand;
#[macro_use] extern crate derive_new;

mod base;
mod applied;
mod external_api;

mod rust_bench;

pub use self::external_api::{IntervalTeardownTreeMap, IntervalTeardownTreeSet, Interval, KeyInterval,
                             TeardownTreeMap, TeardownTreeSet, TeardownTreeRefill,
                             iter};
pub use self::base::{ItemFilter, NoopFilter, Sink};
pub use self::base::sink;
pub use self::base::util;



#[cfg(test)]
mod test_delete_plain {
    use base::sink::UncheckedVecRefSink;
    use base::{ItemFilter, NoopFilter};
    use base::util::make_teardown_seq;
    use base::validation::{check_bst_del_range, check_integrity_del_range};
    use applied::plain_tree::{PlTree, PlNode};
    use external_api::{TeardownTreeSet, TreeWrapperAccess};
    use super::common::{conv_from_tuple_vec, check_tree, test_exhaustive_items, exhaustive_range_check, mk_prebuilt};

    use rand::{Rng, XorShiftRng, SeedableRng};
    use std::fmt::Debug;
    use std::ops::Range;

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
        let tree = PlTree::new((1..n+1)
            .zip( vec![(); n].into_iter() )
            .collect::<Vec<_>>());
        delete_range_exhaustive_with_tree(tree);
    }


    fn test_prebuilt(items: &[usize], range: Range<usize>) {
        let nodes: Vec<Option<Nd>> = mk_prebuilt(items);
        let tree = PlTree::with_nodes(nodes);
        let mut output = Vec::with_capacity(tree.size());
        delete_and_check(&mut TeardownTreeSet::from_internal(tree), range, &mut NoopFilter, &mut output);
    }

    pub fn test_exhaustive_n<F>(n: usize, check: &F)
        where F: Fn(Tree) -> ()
    {
        let elems: Vec<_> = (1..n + 1).collect();
        println!("exhaustive n={}: elems={:?} ------------------------", n, &elems);

        let mut items: Vec<_> = elems.into_iter().map(|x| Some((x, ()))).collect();
        test_exhaustive_items::<_, Tree, _>(&mut items, check);
    }

    fn delete_and_check<Flt>(orig: &mut TeardownTreeSet<usize>, search: Range<usize>, filter: &mut Flt, output: &mut Vec<usize>) -> TeardownTreeSet<usize>
        where Flt: ItemFilter<usize>+Clone+Debug
    {
        let mut tree = orig.clone();
        tree.filter_range(search.clone(), filter.clone(), UncheckedVecRefSink::new(output));
        check_tree(orig.internal(), tree.internal_mut(), &search, filter, output);
        tree
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
            exhaustive_check(n, i..i+1, &mut output, tree_mod, &tree, &NoopFilter);
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
                tree_mod.delete_range(i..j, UncheckedVecRefSink::new(&mut output));
                let output = conv_from_tuple_vec(&mut output);
                exhaustive_check(n, i..j, output, tree_mod, &tree, &NoopFilter);
            }
        }
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
        delete_and_check(&mut tree, rm, &mut NoopFilter, output);
        true
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
            orig = delete_and_check(&mut orig, range, &mut SetRefFilter::new(&flt_tree), &mut output);
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

    fn exhaustive_check<Flt>(n: usize, range: Range<usize>, output: &mut Vec<usize>, tree_mod: Tree, tree_orig: &Tree, filter: &Flt)
        where Flt: Debug
    {
        exhaustive_range_check(n, &range, output, tree_orig);
        check_bst_del_range(&range, &tree_mod, &output, &tree_orig, filter);
        check_integrity_del_range(&range, &tree_mod, output, &tree_orig, filter);
        assert!(tree_mod.size() + output.len() == n, "filter={:?}, tree'={:?}, tree={}, tree_mod={}, sz={}, output={:?}, n={}", filter, **tree_orig, **tree_orig, *tree_mod, tree_mod.size(), output, n);
    }
}



#[cfg(test)]
mod test_query_plain {
    use std::ops::Range;

    use applied::interval::{KeyInterval, Interval};
    use applied::plain_tree::{PlTree, PlNode};
    use external_api::{TeardownTreeSet, TeardownTreeMap, TreeWrapperAccess};
    use base::{TreeRepr, Traverse};
    use base::sink::{RefCopyingSink, UncheckedVecRefSink};
    use super::test_delete_plain::test_exhaustive_n;
    use super::common::{exhaustive_range_check, mk_prebuilt, check_output_sorted, test_exhaustive_items};

    type Nd = PlNode<usize, ()>;
    type Tree = PlTree<usize, ()>;


    //---- exhaustive query_range ------------------------------------------------------------------
    #[test]
    fn query_range_exhaustive() {
        for i in 1..8 {
            query_range_exhaustive_n(i);
        }
    }

    fn query_range_exhaustive_n(n: usize) {
        test_exhaustive_n(n, &|tree| query_range_exhaustive_with_tree(tree));
    }

    fn query_range_exhaustive_with_tree(tree: Tree) {
        let tree = TeardownTreeSet::from_internal(tree);
        let n = tree.size();
        let mut output = Vec::with_capacity(n);
        for i in 0..n+2 {
            for j in i..n+2 {
                output.truncate(0);
                {
                    let sink = RefCopyingSink::new(UncheckedVecRefSink::new(&mut output));
                    tree.query_range(i..j, sink);
                }
                exhaustive_range_check(n, &(i..j), &mut output, tree.internal());
            }
        }
    }


    //---- exhaustive iter -------------------------------------------------------------------------
    #[test]
    fn iter_exhaustive() {
        for i in 1..10 {
            iter_exhaustive_n(i);
        }
    }

    fn iter_exhaustive_n(n: usize) {
        test_exhaustive_n(n, &|tree| iter_exhaustive_with_tree(tree));
    }

    fn iter_exhaustive_with_tree(tree: Tree) {
        let tree = TeardownTreeSet::from_internal(tree);
        let mut n = tree.size();
        for (i, &x) in tree.iter().enumerate() {
            assert!(i+1 == x, "i={}, x={}, tree={}", i, x, &tree);
            n -= 1;
        }
        assert!(n == 0);
    }


    //---- exhaustive into_iter --------------------------------------------------------------------
    #[test]
    fn into_iter_exhaustive() {
        for i in 1..10 {
            into_iter_exhaustive_n(i);
        }
    }

    fn into_iter_exhaustive_n(n: usize) {
        test_exhaustive_n(n, &|tree| into_iter_exhaustive_with_tree(tree));
    }

    fn into_iter_exhaustive_with_tree(tree: Tree) {
        let tree = TeardownTreeSet::from_internal(tree);
        let mut n = tree.size();
        for (i, x) in tree.clone().into_iter().enumerate() {
            assert!(i+1 == x, "i={}, x={}, tree={}", i, x, &tree);
            n -= 1;
        }
        assert!(n == 0);
    }


    //---- exhaustive find -------------------------------------------------------------------------
    #[test]
    fn find_exhaustive() {
        for i in 1..10 {
            find_exhaustive_n(i);
        }
    }

    fn find_exhaustive_n(n: usize) {
        let elems: Vec<_> = (1..n + 1).collect();
        println!("exhaustive n={}: elems={:?} ------------------------", n, &elems);

        let mut items: Vec<_> = elems.into_iter().map(|x| Some((x, x))).collect();
        test_exhaustive_items::<_, PlTree<usize, usize>, _>(&mut items, &|tree| find_exhaustive_with_tree(tree));

    }

    fn find_exhaustive_with_tree(tree: PlTree<usize, usize>) {
        let tree = TeardownTreeMap::from_internal(tree);
        let n = tree.size();

        assert_eq!(tree.find(&0), None);
        for i in 1..n+1 {
            assert_eq!(tree.find(&i), Some(&i));
        }
        for i in n+1..2*n+2 {
            assert_eq!(tree.find(&i), None);
        }
    }


    //---- prebuilt --------------------------------------------------------------------------------
    fn test_prebuilt(items: &[usize], range: Range<usize>) {
        let nodes: Vec<Option<Nd>> = mk_prebuilt(items);
        let tree = PlTree::with_nodes(nodes);
        let tree = TeardownTreeSet::from_internal(tree);
        let mut output = Vec::with_capacity(tree.size());

        {
            let sink = RefCopyingSink::new(UncheckedVecRefSink::new(&mut output));
            tree.query_range(range.clone(), sink);
        }

        let search = KeyInterval::from_range(&range);
        check_output_sorted(&output, &*tree.internal(), &search);

        let mut expected = vec![];
        TreeRepr::traverse_inorder(&*tree.internal(), 0, &mut (), |this, _, idx| {
            if this.key(idx).overlaps(&search) {
                expected.push(this.key(idx).clone());
            }
            false
        });

        assert_eq!(output, expected, "range={:?}, tree={}", &range, &tree);
    }

    #[test]
    fn query_range_prebuilt() {
        test_prebuilt(&[1], 1..1);
        test_prebuilt(&[1], 0..2);
    }
}



#[cfg(test)]
mod test_delete_interval {
    use std::ops::{Range};
    use rand::{Rng, XorShiftRng, SeedableRng};
    use std::cmp;
    use std::fmt::Debug;

    use base::sink::UncheckedVecRefSink;
    use base::{Node, ItemFilter, NoopFilter, lefti, righti};
    use base::validation::{check_bst_del_range, check_integrity_del_range, gen_tree_keys};
    use base::util::make_teardown_seq;
    use applied::AppliedTree;
    use applied::interval::{Interval, IvNode, KeyInterval};
    use applied::interval_tree::{IvTree};
    use external_api::{IntervalTeardownTreeSet, TreeWrapperAccess};
    use super::common::{check_tree};

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
            delete_and_check(tree, rm.clone(), filter, output);
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


    fn delete_and_check<Flt>(orig: &mut IntervalTeardownTreeSet<KeyInterval<usize>>, rm: Range<usize>, mut filter: Flt,
                             output: &mut Vec<KeyInterval<usize>>) -> IntervalTeardownTreeSet<KeyInterval<usize>>
        where Flt: ItemFilter<KeyInterval<usize>>+Clone+Debug
    {
        let mut tree = orig.clone();
        {
            let query = KeyInterval::from_range(&rm);
            tree.filter_overlap(&query, filter.clone(), UncheckedVecRefSink::new(output));
            let (mut orig, mut tree) = (orig.internal_mut(), tree.internal_mut());

            check_tree(&mut *orig, &mut tree, &rm, &mut filter, output);
            if tree.size() > 0 {
                check_maxb(&orig, &tree, 0);
            }
        }

        tree
    }


    fn test_random_shape_overlap(xs: Vec<Range<usize>>, rm: Range<usize>, rng: &mut XorShiftRng) -> bool {
        let rm = normalize_range(rm);
        let mut output = Vec::with_capacity(xs.len());
        test_random_shape(xs, rng, |tree| { delete_and_check(tree, rm.clone().into(), NoopFilter, &mut output); } )
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
            let (tree, orig): (&Tree, &Tree) = (tree.internal(), orig.internal());
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

        let mut maxb = *tree.node(idx).key().b();
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
        let items: Vec<Option<(Iv, ())>> = items.into_iter().map(|opt| opt.map(|iv| (iv, ()))).collect();
        unsafe { Tree::with_shape(items) }
    }


    fn check_maxb(orig: &Tree, tree: &Tree, idx: usize) -> usize {
        assert!(!tree.is_nil(idx));

        let mut expected_maxb = *tree.node(idx).key().b();
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
            orig = delete_and_check(&mut orig, range, NoopFilter, &mut output);
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
            orig = delete_and_check(&mut orig, range, SetRefFilter::new(&flt_tree), &mut output);
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



#[cfg(test)]
mod test_query_interval {
    use applied::interval::{KeyInterval};
    use applied::interval_tree::{IvTree};
    use external_api::{IntervalTeardownTreeSet, TreeWrapperAccess};
    use base::sink::{RefCopyingSink, UncheckedVecRefSink};
    use super::common::{exhaustive_range_check, test_exhaustive_items};

    type Tree = IvTree<usize, ()>;


    //---- exhaustive ------------------------------------------------------------------------------
    #[test]
    fn query_overlap_exhaustive() {
        for i in 1..8 {
            query_overlap_exhaustive_n(i);
        }
    }

    fn query_overlap_exhaustive_n(n: usize) {
        test_exhaustive_n(n, &|tree| query_overlap_exhaustive_with_tree(tree));
    }

    fn query_overlap_exhaustive_with_tree(tree: Tree) {
        let tree = IntervalTeardownTreeSet::from_internal(tree);
        let n = tree.size();
        let mut output = Vec::with_capacity(n);
        for i in 0..n+2 {
            for j in i..n+2 {
                let tree_mod: IntervalTeardownTreeSet<_> = tree.clone();
                output.truncate(0);
                {
                    let sink = RefCopyingSink::new(UncheckedVecRefSink::new(&mut output));
                    tree_mod.query_overlap(&KeyInterval::new(i, j), sink);
                }
                exhaustive_range_check(n, &(i..j), &mut output, tree.internal());
            }
        }
    }

    pub fn test_exhaustive_n<F>(n: usize, check: &F)
        where F: Fn(Tree) -> ()
    {
        let elems: Vec<_> = (1..n + 1).collect();
        println!("exhaustive n={}: elems={:?} ------------------------", n, &elems);

        let mut items: Vec<_> = elems.into_iter().map(|x| Some((x, ()))).collect();
        test_exhaustive_items::<_, Tree, _>(&mut items, check);
    }
}



#[cfg(test)]
mod common {
    use base::validation::{check_bst_del_range, check_integrity_del_range};
    use base::{Node, TreeRepr, TreeDeref, Traverse, ItemFilter, lefti, righti};
    use applied::AppliedTree;
    use applied::interval::{Interval, KeyInterval};
    use applied::plain_tree::{PlNode};

    use std::fmt::{Debug, Display};
    use std::ops::{Range};
    use std::cmp;

    //---- exhaustive testing ----------------------------------------------------------------------
    #[derive(Debug)]
    struct TreeRangeInfo {
        range: Range<usize>,
        root_idx: usize
    }

    pub fn test_exhaustive_items<N: Node, Tree: AppliedTree<N>, F>(items: &mut Vec<Option<(N::K, N::V)>>, check: &F)
        where N::K: Clone, N::V: Clone,
              F: Fn(Tree) -> ()
    {
        let n = items.len();
        let mut stack = vec![TreeRangeInfo { range: (0..n), root_idx: 0 }];
        let mut shape = vec![None; 1 << n];
        test_exhaustive_rec::<_, Tree, _>(items, &mut shape, &mut stack, check)
    }

    fn test_exhaustive_rec<N: Node, Tree: AppliedTree<N>, F>(items: &mut Vec<Option<(N::K, N::V)>>, shape: &mut Vec<Option<(N::K, N::V)>>,
                                                             stack: &mut Vec<TreeRangeInfo>, check: &F)
        where N::K: Clone, N::V: Clone,
              F: Fn(Tree) -> ()
    {
        if stack.is_empty() {
            let tree = unsafe { Tree::with_shape(shape.clone()) };
            check(tree);
        } else {
            let info = stack.pop().unwrap();
            let (lefti, righti) = (lefti(info.root_idx), righti(info.root_idx));
            for i in info.range.clone() {
                assert!(shape[info.root_idx].is_none() && items[i].is_some());
                shape[info.root_idx] = items[i].take();

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

                test_exhaustive_rec(items, shape, stack, check);

                items[i] = shape[info.root_idx].take();
                for _ in 0..pushed {
                    stack.pop();
                }
            }

            stack.push(info);
        }
    }


    //---- misc ------------------------------------------------------------------------------------
    pub fn mk_prebuilt(items: &[usize]) -> Vec<Option<PlNode<usize, ()>>> {
        let nodes: Vec<_> = items.iter().map(|&x| if x==0 {
            None
        } else {
            Some(Node::new(x, ()))
        }).collect();

        nodes
    }

    pub fn conv_from_tuple_vec<K>(items: &mut Vec<(K, ())>) -> &mut Vec<K> {
        use std::mem;
        unsafe { mem::transmute(items) }
    }

    pub fn exhaustive_range_check<N: Node, Tree: TreeDeref<N>>(n: usize, range: &Range<usize>, output: &mut Vec<usize>, tree_orig: &Tree)
        where N: Debug
    {
        let expected: Vec<_> =
            if range.start == range.end && range.start<=n && 1<=range.start {
                vec![range.start]
            } else {
                let expected_range = cmp::max(1, range.start) .. cmp::min(n+1, range.end);
                expected_range.collect()
            };

        assert_eq!(output, &expected, "range={:?}, tree_orig={}", range, **tree_orig);
    }

    pub fn check_output_sorted<N: Node, Item, Rm>(output: &Vec<Item>, orig: &TreeRepr<N>, rm: &Rm)
        where N::K: Debug, N: Debug, Rm: Debug, Item: Ord+Debug
    {
        for i in 1..output.len() {
            assert!(output[i - 1] <= output[i], "output={:?}, rm={:?}, orig={:?}, {}", output, rm, orig, orig);
        }
    }



    pub fn check_tree<K: Interval<K=usize>, N: Node<K=K>, Item, Flt>(orig: &TreeRepr<N>, tree: &mut TreeRepr<N>,
                                                                     search: &Range<usize>, filter: &mut Flt, output: &Vec<Item>)
          where Item: Interval<K=usize>+Debug,
                Flt: ItemFilter<K>+Debug,
                N: Debug, K: Debug
    {
        use applied::interval::KeyInterval;
        let search = KeyInterval::from(search.clone());
        check_bst_del_range(&search, tree, output, orig, filter);
        check_integrity_del_range(&search, tree, output, orig, filter);
        check_output_overlaps(&search, tree, output, orig, filter);
        check_tree_doesnt_overlap(&search, tree, filter);

        assert!(output.len() + tree.size() == orig.size());

        check_output_sorted(output, orig, &search);
    }


    fn check_tree_doesnt_overlap<K, N, Search, Flt>(search: &Search, tree: &mut TreeRepr<N>, flt: &mut Flt)
        where K: Interval<K=usize>+Debug,
              N: Node<K=K>+Debug,
              Search: Interval<K=usize>+Debug,
              Flt: ItemFilter<K>
    {
        TreeRepr::traverse_inorder(tree, 0, &mut (), |this, _, idx| {
            assert!(!this.key(idx).overlaps(search) || !flt.accept(this.key(idx)), "idx={}, key(idx)={:?}, search={:?}, tree={:?}, {}", idx, this.key(idx), search, this, this);
            false
        });
    }


    fn check_output_overlaps<Tree, Item, Flt>(search: &KeyInterval<usize>, tree: &Tree, output: &Vec<Item>, tree_orig: &Tree, filter: &Flt)
          where Item: Interval<K=usize>+Debug,
                Flt: Debug,
                Tree: Debug+Display
    {
        for (_, x) in output.iter().enumerate() {
            assert!(search.overlaps(x), "search={:?}, output={:?}, tree={:?}, flt={:?}, orig={:?}, {}", search, output, tree, filter, tree_orig, tree_orig);
        }
    }
}
