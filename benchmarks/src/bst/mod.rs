mod node;

use std::cell::UnsafeCell;
use std::mem;
use std::ops::{Range};
use std::ptr;

use self::node::Node;
use teardown_tree::Sink;
use teardown_tree::sink::{CountingSink};


pub struct BST<K: Ord, V> {
    root: UnsafeCell<Option<Box<Node<K, V>>>>,
    size: usize,
}

#[derive(new)]
pub struct ConsumeIter<K, V> {
    cur: Option<Box<Node<K, V>>>,
}

pub struct IntoIter<K, V> {
    inner: ConsumeIter<K, V>,
    remaining: usize
}



impl<K: Ord, V> BST<K, V> {
    pub fn new() -> BST<K, V> {
        BST { root: UnsafeCell::new(None), size: 0 }
    }

    pub fn size(&self) -> usize { self.size }
    pub fn is_empty(&self) -> bool { self.size() == 0 }

    pub fn into_iter(mut self) -> IntoIter<K, V> {
        IntoIter::new(self.root_mut().take(), self.size)
    }


    pub fn clear(&mut self) {
        let iter = IntoIter::new(self.root_mut().take(), self.size);
        for _ in iter {
            // ignore, drop the values (and the node)
        }
        self.size = 0;
    }


    pub fn with_sorted(mut sorted: Vec<(K, V)>) -> BST<K, V> {
        let size = sorted.len();
        BST {
            root: UnsafeCell::new(Self::build(&mut sorted)),
            size: size
        }
    }

    /// Finds the point to partition n keys for a nearly-complete binary tree
    /// http://stackoverflow.com/a/26896494/3646645
    fn build_select_root(n: usize) -> usize {
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

    /// Returns the height of the tree. This consumes the contents of `data`, so the caller must
    /// make sure the contents are never reused or dropped after this call returns.
    fn build(sorted: &mut [(K, V)]) -> Option<Box<Node<K, V>>> {
        match sorted.len() {
            0 => None,
            n => {
                let mid = Self::build_select_root(n);
                let left = Self::build(&mut sorted[..mid]);
                let right = Self::build(&mut sorted[mid+1..]);


                unsafe {
                    let p = sorted.get_unchecked(mid);
                    let (k, v) = ptr::read(p);
                    Some(Node::new(k, v, left, right))
                }
            }
        }
    }


    #[inline(never)]
    fn consume_tree(node: Option<Box<Node<K, V>>>, sink: &mut Sink<(K, V)>) {
        if let Some(node) = node {
            let (key, value, left, right) = node.decons();
            Self::consume_tree(left, sink);
            sink.consume((key, value));
            Self::consume_tree(right, sink);
        } else {
            return
        };
    }

//    fn consume_tree2(node: Option<Box<Node<K, V>>>, sink: &mut Sink<(K, V)>) {
//        let iter = ConsumeIter::new(node);
//        for item in iter {
//            sink.consume(item);
//        }
//    }
}


impl<K: Ord, V> BST<K, V> {
    pub fn delete_range<S: Sink<(K,V)>>(&mut self, range: Range<K>, sink: S) {
        let mut sink = CountingSink::new(sink);
        Self::delete_range_loop(range, self.root_mut(), &mut sink);
        self.size -= sink.count();
    }

    #[inline]
    fn delete_range_loop(range: Range<K>, mut node_opt: &mut Option<Box<Node<K, V>>>, sink: &mut Sink<(K,V)>) {
        loop {
            let (new_left, new_right): (Option<Box<Node<K,V>>>, _) =
                if let Some(node) = Self::node_mut(node_opt) {
                    if range.start <= node.key && node.key < range.end || node.key == range.start {
                        let mut node = node_opt.take().unwrap();
                        let new_left = Self::delete_range_left(&range, node.pop_left(), sink);
                        let right = node.pop_right();
                        sink.consume(node.into_tuple());
                        let new_right = Self::delete_range_right(&range, right, sink);

                        (new_left, new_right)
                    } else if node.key < range.start {
                        node_opt = &mut node.right;
                        continue;
                    } else {
                        node_opt = &mut node.left;
                        continue;
                    }
                } else {
                    break;
                };


            let replacement =
                if new_left.is_some() {
                    if new_right.is_some() {
                        let (new_left, mut new_root) = Self::delete_max(new_left.unwrap());
                        new_root.left = new_left;
                        new_root.right = new_right;
                        Some(new_root)
                    } else {
                        new_left
                    }
                } else {
                    new_right
                };

            *node_opt = replacement;
            break;
        }
    }

    fn root_mut(&mut self) -> &mut Option<Box<Node<K, V>>> {
        unsafe { &mut *self.root.get() }
    }
    fn root_ref(&self) -> &Option<Box<Node<K, V>>> {
        unsafe { &*self.root.get() }
    }


    fn node_mut<'a>(node_opt: &mut Option<Box<Node<K, V>>>) -> Option<&'a mut Box<Node<K,V>>> {
        unsafe {
            let opt: Option<&mut Box<Node<K,V>>> = node_opt.as_mut();
            mem::transmute(opt)
        }
    }


    #[inline(never)]
    fn delete_range_left(range: &Range<K>, node_opt: Option<Box<Node<K, V>>>,
                         sink: &mut Sink<(K,V)>) -> Option<Box<Node<K, V>>>
    {
        node_opt.map_or(None, |mut node| {
            if range.start <= node.key {
                debug_assert!(node.key < range.end || node.key == range.start);
                let left = Self::delete_range_left(range, node.pop_left(), sink);
                let right = node.pop_right();
                sink.consume(node.into_tuple());
                Self::consume_tree(right, sink);
                left
            } else {
                node.right = Self::delete_range_left(range, node.pop_right(), sink);
                Some(node)
            }
        })
    }

    #[inline(never)]
    fn delete_range_right(range: &Range<K>, node_opt: Option<Box<Node<K, V>>>,
                          sink: &mut Sink<(K,V)>) -> Option<Box<Node<K, V>>>
    {
        node_opt.map_or(None, |mut node| {
            if node.key < range.end || node.key == range.start {
                debug_assert!(node.key >= range.start);
                let (key, value, left, right) = node.decons();
                Self::consume_tree(left, sink);
                sink.consume((key, value));
                Self::delete_range_right(range, right, sink)
            } else {
                node.left = Self::delete_range_right(range, node.pop_left(), sink);
                Some(node)
            }
        })
    }

    fn delete_max(mut root: Box<Node<K,V>>) -> (Option<Box<Node<K,V>>>, Box<Node<K,V>>) {
        if let Some(right) = Self::node_mut(&mut root.right) {
            let max = {
                let mut parent: &mut Box<Node<K,V>> = &mut root;
                let mut node: &mut Box<Node<K,V>> = right;

                while let Some(right) = Self::node_mut(&mut node.right) {
                    parent = unsafe { mem::transmute(node) };
                    node = right;
                }

                let mut max: Box<Node<K,V>> = parent.pop_right().unwrap();
                parent.right = max.pop_left();
                max
            };
            (Some(root), max)
        } else {
            (root.pop_left(), root)
        }
    }
}






impl<K: Clone + Ord, V: Clone> Clone for BST<K, V> {
    fn clone(&self) -> BST<K, V> {
        BST {
            root: UnsafeCell::new(self.root_ref().clone()),
            size: self.size,
        }
    }
}

impl<K: Ord, V> Drop for BST<K, V> {
    fn drop(&mut self) {
        // Be sure to not recurse too deep on destruction
        self.clear();
    }
}





impl<K, V> Iterator for ConsumeIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<(K, V)> {
        let mut cur = match self.cur.take() {
            Some(cur) => cur,
            None => return None,
        };
        loop {
            match cur.pop_left() {
                Some(node) => {
                    let mut node = node;
                    cur.left = node.pop_right();
                    node.right = Some(cur);
                    cur = node;
                }

                None => {
                    self.cur = cur.pop_right();
                    // left and right fields are both None
                    let node = *cur;
                    return Some(node.into_tuple());
                }
            }
        }
    }
}



impl<K: Ord, V> IntoIter<K, V> {
    pub fn new(cur: Option<Box<Node<K,V>>>, remaining: usize) -> Self {
        IntoIter { inner: ConsumeIter::new(cur), remaining:remaining }
    }
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<(K, V)> {
        let item = self.inner.next();
        if item.is_some() {
            self.remaining -= 1;
        }
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {}



#[cfg(test)]
mod tests {
    use ::teardown_tree::sink::UncheckedVecRefSink;
    use super::*;

    fn build(elems: &[usize]) -> BST<usize, ()> {
        let elems = elems.iter()
                         .map(|&x| (x, ()))
                         .collect();
        BST::with_sorted(elems)
    }

    fn run_test_with_tree(tree: &mut BST<usize, ()>, copy: &mut BST<usize, ()>, range: Range<usize>) {
        let mut output = Vec::with_capacity(tree.size());
        {
            let sink = UncheckedVecRefSink::new(&mut output);
            copy.delete_range(range.clone(), sink);
        }

        let expected_output: Vec<_> =
            tree.clone().into_iter()
                .map(|(x, _)| x)
                .filter(|&x| range.start <= x && x < range.end || x == range.start)
                .collect();
        let output: Vec<_> = output.into_iter().map(|(x, _)| x).collect();

        assert_eq!(output, expected_output);
        copy.clone().into_iter().map(|(x, _)| assert!(x < range.start || range.end<=x && x != range.start)).collect::<Vec<_>>();
    }

    fn run_test(elems: &[usize], range: Range<usize>) {
        let mut tree = build(elems);
        let mut copy = tree.clone();
        run_test_with_tree(&mut tree, &mut copy, range);
    }

    #[test]
    fn bst_basic() {
        run_test(&[0], 0..1);
        run_test(&[0], 0..0);
        run_test(&[0], 1..1);

        run_test(&[0,1], 0..0);
        run_test(&[0,1], 0..1);
        run_test(&[0,1], 0..2);
        run_test(&[0,1], 1..2);
        run_test(&[0,1], 2..2);

        run_test(&[0,1,2], 0..0);
        run_test(&[0,1,2], 0..1);
        run_test(&[0,1,2], 0..2);
        run_test(&[0,1,2], 0..3);
        run_test(&[0,1,2], 1..1);
        run_test(&[0,1,2], 1..2);
        run_test(&[0,1,2], 1..3);
        run_test(&[0,1,2], 2..2);
        run_test(&[0,1,2], 2..3);
        run_test(&[0,1,2], 3..3);

        run_test(&[0,1,2,3,4,5,6,7,8,9,10,11], 10..20);

        run_test(&(0..100).collect::<Vec<_>>(), 80..90);

        let mut tree = build(&(0..100).collect::<Vec<_>>());
        let mut copy = tree.clone();
        let (tree, copy) = (&mut tree, &mut copy);
        run_test_with_tree(tree, copy, 10..20);
        run_test_with_tree(tree, copy, 50..60);
        run_test_with_tree(tree, copy, 20..30);
        run_test_with_tree(tree, copy, 40..50);
        run_test_with_tree(tree, copy, 90..100);
        run_test_with_tree(tree, copy, 30..40);
        run_test_with_tree(tree, copy, 70..80);
        run_test_with_tree(tree, copy, 60..70);
        run_test_with_tree(tree, copy, 80..90);
        run_test_with_tree(tree, copy, 0..10);
    }
}