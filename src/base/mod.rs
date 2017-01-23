mod slot_stack;
mod bulk_delete;
mod unsafe_stack;
mod base_repr;
mod node;
pub mod util;
pub mod drivers;

pub use self::slot_stack::*;
pub use self::bulk_delete::*;
pub use self::unsafe_stack::*;
pub use self::drivers::*;
pub use self::base_repr::*;
pub use self::node::*;

use std::ptr;


/// A fast way to refill the tree from a master copy; adds the requirement for T to implement Copy.
pub trait TeardownTreeRefill {
    fn refill(&mut self, master: &Self);
}



impl<N: Node> TeardownTreeRefill for TreeRepr<N> where N::K: Copy, N::V: Copy {
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




pub trait ItemFilter<K: Key> {
    #[inline(always)] fn accept(&mut self, key: &K) -> bool;
    #[inline(always)] fn is_noop() -> bool;
}

#[derive(Clone)]
pub struct NoopFilter;

impl<K: Key> ItemFilter<K> for NoopFilter {
    #[inline(always)] fn accept(&mut self, key: &K) -> bool {
        true
    }

    #[inline(always)] fn is_noop() -> bool {
        true
    }
}



#[cfg(test)]
pub mod validation {
    use rand::{Rng, XorShiftRng};
    use std::fmt::Debug;
    use base::{Key, TreeRepr, Node, lefti, righti, parenti};

    type Tree<N> = TreeRepr<N>;

    /// Validates the BST property.
    pub fn check_bst<'a, N: Node, U: Ord+Debug>(tree: &'a Tree<N>, output: &Vec<U>, tree_orig: &Tree<N>, idx: usize) -> Option<(&'a N::K, &'a N::K)>
        where N: Debug, N::K: Debug
    {
        let node = tree.node_opt(idx);
        if node.is_none() {
            return None;
        } else {
            let key = &node.unwrap().key;
            let left = check_bst(tree, output, tree_orig, lefti(idx));
            let right = check_bst(tree, output, tree_orig, righti(idx));

            let min =
                if let Some((lmin, lmax)) = left {
                    debug_assert!(lmax <= key, "lmax={:?}, key={:?}, tree_orig: {:?}, tree: {:?}, output: {:?}", lmax, key, tree_orig, tree, output);
                    lmin
                } else {
                    key
                };
            let max =
                if let Some((rmin, rmax)) = right {
                    debug_assert!(key <= rmin, "tree_orig: {:?}, tree: {:?}, output: {:?}", tree_orig, tree, output);
                    rmax
                } else {
                    key
                };

            return Some((min, max));
        }
    }

    /// Checks that there are no dangling items (the parent of every item marked as present is also marked as present).
    pub fn check_integrity<N: Node>(tree: &Tree<N>, tree_orig: &Tree<N>) where N: Debug {
        let mut noccupied = 0;

        for i in 0..tree.data.len() {
            if tree.mask[i] {
                debug_assert!(i == 0 || tree.mask[parenti(i)], "i={}, tree_orig: {:?}, {}, tree: {:?}, {}", i, tree_orig, tree_orig, tree, tree);
                noccupied += 1;
            }
        }

        debug_assert!(noccupied == tree.size());
    }


    pub fn gen_tree_keys<T: Key>(items: Vec<T>, rng: &mut XorShiftRng) -> Vec<Option<T>> {
        let mut shaped = vec![None; 1 << 18];
        gen_subtree_keys(&items, 0, &mut shaped, rng);

        let mut items = shaped.into_iter()
            .rev()
            .skip_while(|opt| opt.is_none())
            .collect::<Vec<_>>();
        items.reverse();
        items
    }

    fn gen_subtree_keys<T: Key>(items: &[T], idx: usize, output: &mut Vec<Option<T>>, rng: &mut XorShiftRng) {
        if items.len() == 0 {
            return;
        }

        // hack
        if idx >= output.len() {
            return;
        }

        let root = rng.gen_range(0, items.len());
        output[idx] = Some(items[root].clone());
        gen_subtree_keys(&items[..root], lefti(idx), output, rng);
        gen_subtree_keys(&items[root+1..], righti(idx), output, rng);
    }
}
