use base::{TreeBase, Node, lefti, righti};
use base::bulk_delete::DeleteRangeCache;
use std::fmt::{Debug, Formatter};
use std::fmt;
use std::mem;
use std::ptr;
use std::cmp::{max};
use std::ops::{Deref, DerefMut};
use std::marker::PhantomData;


pub trait Key: Ord+Clone {}

impl<T: Ord+Clone> Key for T {}

pub trait TreeReprAccess<N: Node>: Deref<Target=TreeRepr<N>> + DerefMut {}

#[derive(Clone)]
pub struct TreeRepr<N: Node> {
    pub data: Vec<N>,
    pub mask: Vec<bool>,
    pub size: usize,

    pub delete_range_cache: DeleteRangeCache,
}

#[derive(Clone)]
pub struct TreeWrapper<N: Node> {
    repr: TreeRepr<N>
}

impl<N: Node> TreeWrapper<N> {
    pub fn new(mut items: Vec<(N::K, N::V)>) -> TreeWrapper<N> {
        items.sort_by(|a, b| a.0.cmp(&b.0));
        Self::with_sorted(items)
    }


    /// Constructs a new TeardownTree<T>
    /// Note: the argument must be sorted!
    pub fn with_sorted(mut sorted: Vec<(N::K, N::V)>) -> TreeWrapper<N> {
        let size = sorted.len();

        let capacity = size;

        let mut data = Vec::with_capacity(capacity);
        unsafe { data.set_len(capacity); }

        let mask: Vec<bool> = vec![true; capacity];
        let height = Self::build(&mut sorted, 0, &mut data);
        unsafe { sorted.set_len(0); }
        let cache = DeleteRangeCache::new(height);
        TreeWrapper { repr: TreeRepr { data: data, mask: mask, size: size, delete_range_cache: cache }}
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
    pub fn with_nodes(mut nodes: Vec<Option<N>>) -> TreeWrapper<N> {
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
        TreeWrapper { repr: TreeRepr { data: data, mask: mask, size: size, delete_range_cache: cache } }
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

impl<N: Node> Deref for TreeWrapper<N> {
    type Target = TreeRepr<N>;

    fn deref(&self) -> &Self::Target {
        &self.repr
    }
}

impl<N: Node> DerefMut for TreeWrapper<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.repr
    }
}

impl<N: Node> Drop for TreeWrapper<N> {
    fn drop(&mut self) {
        self.drop_items();
        unsafe {
            self.data.set_len(0)
        }
    }
}


impl<N: Node> Debug for TreeWrapper<N> where N::K: Debug {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let mut nz: Vec<_> = self.mask.iter().enumerate()
            .rev()
            .skip_while(|&(_, flag)| !flag)
            .map(|(i, &flag)| match (self.node(i), flag) {
                (_, false) => String::from("0"),
                (ref node, true) => format!("{:?}", node.key)
            })
            .collect();
        nz.reverse();

        let _ = write!(fmt, "[size={}: ", self.size);
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

impl<N: Node> fmt::Display for TreeWrapper<N> where N::K: Debug {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        writeln!(fmt, "")?;
        let mut ancestors = vec![];
        self.fmt_subtree(fmt, 0, &mut ancestors)
    }
}


impl<N: Node> TreeWrapper<N> where N::K: Debug {
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
            writeln!(fmt, "{:?}", self.key(idx))?;

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
