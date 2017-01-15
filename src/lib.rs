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
pub use self::base::util;



#[cfg(test)]
mod test_plain {
    use base::{TreeBase, TreeWrapper, Node, lefti, righti};
    use base::validation::{check_bst, check_integrity};
    use applied::plain_tree::{PlainDeleteInternal, PlNode};
    use external_api::{TeardownTreeSet, PlainTreeWrapperAccess};
    use std::cmp;

    type Nd = PlNode<usize, ()>;
    type Tree = TreeWrapper<Nd>;


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
        let mut tree_mod = Tree::with_nodes(nodes);

        let mut output = Vec::with_capacity(tree_mod.size());

//        println!("tree={:?}, range=({}, {}), {}", &tree, from, to, &tree);
        let tree_orig = tree_mod.clone();
        tree_mod.delete_range(range.clone(), &mut output);

        let output = super::conv_from_tuple_vec(&mut output);
        delete_range_check(items.iter().filter(|&&x| x!=0).count(), range, output, tree_mod, &tree_orig);
    }

    #[test]
    fn delete_range_prebuilt() {
        test_prebuilt(&[1], 1..2);

        test_prebuilt(&[1], 1..1);

        test_prebuilt(&[1, 0, 2], 1..1);

        test_prebuilt(&[1, 0, 2], 2..2);

        test_prebuilt(&[3, 2, 0, 1], 1..3);

        test_prebuilt(&[3, 2, 4, 1], 1..3);

        test_prebuilt(&[3, 1, 4, 0, 2], 2..4);

        test_prebuilt(&[4, 2, 0, 1, 3], 3..4);


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
            delete_range_check(n, i..i+1, &mut output, tree_mod, &tree);
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
                delete_range_check(n, i..j, output, tree_mod, &tree);
            }
        }
    }

    fn delete_range_check(n: usize, range: Range<usize>, output: &mut Vec<usize>, tree_mod: Tree, tree_orig: &Tree) {
        let expected_range = cmp::max(1, range.start) .. cmp::min(n+1, range.end);

        assert_eq!(output, &expected_range.collect::<Vec<_>>(), "tree_orig={}", tree_orig);
        assert!(tree_mod.size() + output.len() == n, "tree'={:?}, tree={}, tree_mod={}, sz={}, output={:?}, n={}", tree_orig, tree_orig, tree_mod, tree_mod.size(), output, n);

        check_bst(&tree_mod, &output, tree_orig, 0);
        check_integrity(&tree_mod, &tree_orig);
    }





    quickcheck! {
        fn quickcheck_plain_(xs: Vec<usize>, rm: Range<usize>) -> bool {
            check_plain_tree(xs, rm)
        }
    }

    fn check_plain_tree(xs: Vec<usize>, rm: Range<usize>) -> bool {
        let rm = if rm.start <= rm.end { rm } else {rm.end .. rm.start};

        let tree = TeardownTreeSet::new(xs);
        check_tree(tree, rm)
    }

    fn check_tree(mut tree: TeardownTreeSet<usize>, rm: Range<usize>) -> bool {
        let tree: &mut Tree = tree.internal();
        let orig = tree.clone();

        let mut output = Vec::with_capacity(tree.size());
        tree.delete_range(rm.start .. rm.end, &mut output);

        check_bst(&tree, &output, &orig, 0);
        check_integrity(&tree, &orig);

        true
    }
}





#[cfg(test)]
mod test_interval {
    use std::ops::Range;
    use rand::{XorShiftRng, SeedableRng};
    use std::cmp;

    use base::{TreeWrapper, Node, TreeBase, parenti, lefti, righti};
    use base::validation::{check_bst, check_integrity, gen_tree_keys};
    use base::util::make_teardown_seq;
    use applied::interval::{Interval, IvNode, KeyInterval};
    use external_api::{IntervalTeardownTreeSet, IntervalTreeWrapperAccess};

    type Iv = KeyInterval<usize>;
    type IvTree = TreeWrapper<IvNode<Iv, ()>>;


    //---- quickcheck ------------------------------------------------------------------------------
    quickcheck! {
        fn quickcheck_interval_(xs: Vec<Range<usize>>, rm: Range<usize>) -> bool {
            let mut rng = XorShiftRng::from_seed([3, 1, 4, 15]);
            test_random_shape(xs, rm, &mut rng)
        }
    }

    fn test_random_shape(xs: Vec<Range<usize>>, rm: Range<usize>, rng: &mut XorShiftRng) -> bool {
        let mut intervals = xs.into_iter()
            .map(|r| if r.start<=r.end {
                Iv::new(r.start, r.end)
            } else {
                Iv::new(r.end, r.start)
            }
            )
            .collect::<Vec<_>>();
        intervals.sort();

        let tree = gen_tree(intervals, rng);

        let rm = if rm.start <= rm.end {
            Iv::new(rm.start, rm.end)
        } else {
            Iv::new(rm.end, rm.start)
        };
        let mut output = Vec::with_capacity(tree.size());
        check_tree(&mut IntervalTeardownTreeSet::from_internal(tree), rm, &mut output);
        true
    }

    fn test_shape(xs: Vec<Range<usize>>, rm: Range<usize>) {
        let nodes = xs.into_iter()
            .map(|r| if r.start<=r.end {
                    Some(IvNode::new(Iv::new(r.start, r.end), ()))
                } else {
                    None
                }
            )
            .collect::<Vec<_>>();

        let mut internal = IvTree::with_nodes(nodes);
        if internal.size() > 0 {
            init_maxb(&mut internal, 0);
        }

        let mut tree = IntervalTeardownTreeSet::from_internal(internal);
        let mut output = Vec::with_capacity(tree.size());
        check_tree(&mut tree, KeyInterval::from_range(&rm), &mut output);
    }


    fn gen_tree(items: Vec<Iv>, rng: &mut XorShiftRng) -> IvTree {
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
        IvTree::with_nodes(nodes)
    }

    fn check_tree(orig: &mut IntervalTeardownTreeSet<KeyInterval<usize>>, rm: Iv, output: &mut Vec<KeyInterval<usize>>) -> IntervalTeardownTreeSet<KeyInterval<usize>> {
        let mut tree = orig.clone();
        tree.delete_intersecting(&rm, output);

        {
            let (tree, orig): (&mut IvTree, &mut IvTree) = (tree.internal(), orig.internal());
            check_bst(&tree, &output, &orig, 0);
            check_integrity(&tree, &orig);
            check_output_intersects(&rm, &output);
            check_tree_doesnt_intersect(&rm, tree);

            assert!(output.len() + tree.size() == orig.size());

            if tree.size() > 0 {
                check_maxb(orig, tree, 0);
            }

            check_output_sorted(&output, orig, &rm);
        }

        tree
    }

    fn check_output_intersects(search: &Iv, output: &Vec<Iv>) {
        for iv in output.iter() {
            assert!(search.intersects(iv));
        }
    }

    fn check_tree_doesnt_intersect(search: &Iv, tree: &mut IvTree) {
        tree.traverse_inorder(0, &mut (), |this: &mut IvTree, _, idx| {
            assert!(!this.key(idx).intersects(search), "idx={}, key(idx)={:?}, search={:?}, tree={:?}, {}", idx, this.key(idx), search, this, this);
            false
        });
    }

    fn check_output_sorted(output: &Vec<Iv>, orig: &mut IvTree, rm: &Iv) {
        for i in 1..output.len() {
            assert!(output[i-1] <= output[i], "output={:?}, rm={:?}, orig={:?}, {}", output, rm, orig, orig);
        }
    }

    fn init_maxb(tree: &mut IvTree, idx: usize) -> usize {
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

    fn check_maxb(orig: &IvTree, tree: &IvTree, idx: usize) -> usize {
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


    #[test]
    fn prebuilt_random_shape() {
        let rng = &mut XorShiftRng::from_seed([3, 1, 4, 15]);

        full_teardown_n(5, 2);

        test_shape(vec![1..1, 0..2], 0..0);

        test_random_shape(vec![0..0], 0..0, rng);
        test_random_shape(vec![0..2, 1..1], 0..0, rng);
        test_random_shape(vec![0..0, 0..0, 0..1], 0..1, rng);
        test_random_shape(vec![0..0, 1..1, 2..2], 0..1, rng);

        test_random_shape(vec![1..1, 0..0, 0..0, 0..0], 0..1, rng);
        test_random_shape(vec![0..0, 1..1, 0..0, 0..0], 0..1, rng);
        test_random_shape(vec![0..0, 0..0, 1..1, 0..0], 0..1, rng);
        test_random_shape(vec![0..0, 0..0, 0..0, 1..1], 0..1, rng);
        test_random_shape(vec![1..1, 1..1, 1..1, 1..1], 0..1, rng);

        test_random_shape(vec![0..2, 1..2, 1..1, 1..2], 1..2, rng);
        test_random_shape(vec![0..2, 0..2, 2..0, 1..2, 0..2, 1..2, 0..2, 0..2, 1..0, 1..2], 1..2, rng);
        test_random_shape(vec![0..2, 1..1, 0..2, 0..2, 1..2, 1..2, 1..2, 0..2, 1..2, 0..2], 1..2, rng);
    }



    //---- non-quickcheck --------------------------------------------------------------------------
    fn full_teardown_n(n: usize, rm_items: usize) {
        let mut rng = XorShiftRng::from_seed([1, 2, 3, 4]);
        let elems: Vec<_> = (0..n).map(|x| (KeyInterval::new(x,x))).collect();
        let ranges: Vec<Range<usize>> = make_teardown_seq(n, rm_items, &mut rng);

        let mut orig = IntervalTeardownTreeSet::new(elems);
        let mut output = Vec::with_capacity(orig.size());

        for range in ranges.into_iter() {
            output.truncate(0);
            let rm = KeyInterval::from_range(&range);
            orig = check_tree(&mut orig, rm, &mut output);
        }
        assert!(orig.size() == 0);
    }


    #[test]
    fn test_full_teardown() {
        full_teardown_n(259, 3);
        full_teardown_n(1598, 21);
        full_teardown_n(65918, 7347);
        full_teardown_n(88165, 9664);
        full_teardown_n(196561, 81669);
        full_teardown_n(756198, 247787);
    }
}


#[cfg(test)]
fn conv_from_tuple_vec<K>(items: &mut Vec<(K, ())>) -> &mut Vec<K> {
    use std::mem;
    unsafe { mem::transmute(items) }
}
