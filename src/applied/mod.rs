pub mod interval_tree;
pub mod plain_tree;
pub mod interval;

//use base::TreeRepr;
//
//type Tree<K, V> =  TreeRepr<K, V>;
//
//pub trait NodeVisitor<K: Ord, V> {
//    fn visit(&mut self, tree: &mut Tree<K, V>, idx: usize);
//
//    fn consume(&mut self, item: K);
//    fn consume_unchecked(&mut self, item: K);
//    fn consume_ptr(&mut self, src: *const K);
//}
//
