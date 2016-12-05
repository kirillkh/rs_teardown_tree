//#![feature(specialization)]
//#![feature(unique)]
#![cfg_attr(feature = "unstable", feature(test))]

//extern crate test;
extern crate rand;

mod base;
mod slot_stack;
mod delete_range;
mod drivers;
mod unsafe_stack;

mod rust_bench;

pub use base::{Item, TeardownTree, TeardownTreeRefill, Node};




#[cfg(test)]
mod tests {
    use base::{TeardownTree, TeardownTreeInternal, Node};

    type Tree = TeardownTree<usize>;


    #[test]
    fn build() {
        Tree::new(vec![1]);
        Tree::new(vec![1, 2]);
        Tree::new(vec![1, 2, 3]);
        Tree::new(vec![1, 2, 3, 4]);
        Tree::new(vec![1, 2, 3, 4, 5]);
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
        let tree = Tree::new((1..n+1).collect::<Vec<_>>());
        delete_range_exhaustive_with_tree(tree);
    }


    fn test_prebuilt(items: &[usize], from_to: (usize, usize),
                     expect_tree: &[usize], expect_out: &[usize]) {

        let nodes: Vec<Option<Node<usize>>> = mk_prebuilt(items);
        let mut tree = Tree::with_nodes(nodes);
        let (from, to) = from_to;

        let mut output = Vec::with_capacity(tree.size());

        let mut expect = expect_out.to_vec();
        expect.sort();

        tree.delete_range(from, to, &mut output);
        let mut sorted_out = output.clone();
        sorted_out.sort();

        assert_eq!(format!("{:?}", &tree), format!("{:?}", expect_tree));
        assert_eq!(format!("{:?}", &sorted_out), format!("{:?}", expect));
    }

    #[test]
    fn delete_range_prebuilt() {
        test_prebuilt(&[1], (1,1),
                      &[], &[1]);

        test_prebuilt(&[1, 0, 2], (1,1),
                      &[2], &[1]);

        test_prebuilt(&[1, 0, 2], (2,2),
                      &[1], &[2]);

        test_prebuilt(&[3, 2, 4, 1], (1,3),
                      &[4], &[3,2,1]);

        test_prebuilt(&[3, 1, 4, 0, 2], (2,4),
                      &[1], &[3,4,2]);

        test_prebuilt(&[4, 2, 0, 1, 3], (3,4),
                      &[2, 1], &[4,3]);


        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], (1,1),
                      &[2, 0, 3, 0, 0, 0, 4], &[1]);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], (2,2),
                      &[1, 0, 3, 0, 0, 0, 4], &[2]);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], (3,3),
                      &[1, 0, 2, 0, 0, 0, 4], &[3]);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], (4,4),
                      &[1, 0, 2, 0, 0, 0, 3], &[4]);


        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], (1,1),
                      &[4, 3, 0, 2], &[1]);

        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], (2,2),
                      &[4, 3, 0, 1], &[2]);

        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], (3,3),
                      &[4, 2, 0, 1], &[3]);

        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], (4,4),
                      &[3, 2, 0, 1], &[4]);

        test_prebuilt(&[1, 0, 3, 0, 0, 2, 4], (1,2),
                      &[3, 0, 4], &[1, 2]);

        test_prebuilt(&[6, 4, 0, 1, 5, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3], (4,6),
                      &[3, 2, 0, 1], &[6,4,5]);
    }


    fn mk_prebuilt(items: &[usize]) -> Vec<Option<Node<usize>>> {
        let nodes: Vec<_> = items.iter().map(|&x| if x==0 {
            None
        } else {
            Some(Node { item: x })
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
        for i in 1..9 {
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
            let nodes: Vec<Option<Node<usize>>> = mk_prebuilt(items);
            let tree = Tree::with_nodes(nodes);
            check(tree);
        } else {
            let info = stack.pop().unwrap();
            let (lefti, righti) = (Tree::lefti(info.root_idx), Tree::righti(info.root_idx));
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
            debug_assert!(deleted.is_some());
            output.push(i);
            delete_range_check(n, i, i, &mut output, tree_mod, &tree);
        }
    }

    fn delete_range_exhaustive_with_tree(tree: Tree) {
        let n = tree.size();
        let mut output = Vec::with_capacity(n);
        for i in 1..n+1 {
            for j in i..n+1 {
                let mut tree_mod = tree.clone();
//                println!("tree={:?}, from={}, to={}", &tree, i, j);
                output.truncate(0);
                tree_mod.delete_range(i, j, &mut output);
                delete_range_check(n, i, j, &mut output, tree_mod, &tree);
            }
        }
    }

    fn delete_range_check(n: usize, from: usize, to: usize, output: &mut Vec<usize>, tree_mod: Tree, tree_orig: &Tree) {
        debug_assert!(output.len() == to-from+1, "tree'={:?}, tree={}, tree_mod={}, interval=({}, {}), expected output len={}, got: {:?}", tree_orig, tree_orig, tree_mod, from, to, to-from+1, output);
        debug_assert!(tree_mod.size() + output.len() == n, "tree'={:?}, tree={}, tree_mod={}, sz={}, output={:?}, n={}", tree_orig, tree_orig, tree_mod, tree_mod.size(), output, n);

        output.sort();
        assert_eq!(output, &(from..to+1).collect::<Vec<_>>());
        check_bst(&tree_mod, &output, tree_orig, 0);
    }

    fn check_bst(tree: &Tree, output: &Vec<usize>, tree_orig: &Tree, idx: usize) -> Option<(usize, usize)> {
        if tree.size() == 0 || !tree.is_null(idx) {
            return None;
        }

        let node = tree.node_opt(idx);
        if node.is_none() {
            return None;
        } else {
            let item = node.unwrap().item;
            let left = check_bst(tree, output, tree_orig, Tree::lefti(idx));
            let right = check_bst(tree, output, tree_orig, Tree::righti(idx));

            let min =
                if let Some((lmin, lmax)) = left {
                    debug_assert!(lmax < item, "tree_orig: {:?}, tree: {:?}, output: {:?}", tree_orig, tree, output);
                    lmin
                } else {
                    item
                };
            let max =
                if let Some((rmin, rmax)) = right {
                    debug_assert!(item < rmin, "tree_orig: {:?}, tree: {:?}, output: {:?}", tree_orig, tree, output);
                    rmax
                } else {
                    item
                };

            return Some((min, max));
        }
    }
}
