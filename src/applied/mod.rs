pub mod interval_tree;
pub mod plain_tree;
pub mod interval;

use base::TreeRepr;

type Tree<T> =  TreeRepr<T>;

pub trait NodeVisitor<T: Ord> {
    fn visit(&mut self, tree: &mut Tree<T>, idx: usize);

    fn consume(&mut self, item: T);
    fn consume_unchecked(&mut self, item: T);
    fn consume_ptr(&mut self, src: *const T);
}

