mod interval_tree;
mod plain_tree;
pub mod interval;

pub use self::interval_tree::*;
pub use self::plain_tree::*;


use base::TeardownTreeInternal;

type Tree<T> =  TeardownTreeInternal<T>;

pub trait NodeVisitor<T: Ord> {
    fn visit(&mut self, tree: &mut Tree<T>, idx: usize);

    fn consume(&mut self, item: T);
    fn consume_unchecked(&mut self, item: T);
    fn consume_ptr(&mut self, src: *const T);
}

