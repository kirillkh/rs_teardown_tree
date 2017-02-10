pub mod interval_tree;
pub mod plain_tree;
pub mod interval;


use base::{Node, TreeDeref, TreeRepr};

pub trait AppliedTree<N: Node>: TreeDeref<N> + Sized {
    /// Constructs a new AppliedTree
    fn new(items: Vec<(N::K, N::V)>) -> Self {
        Self::with_repr(TreeRepr::new(items))
    }

    /// Constructs a new IvTree
    /// Note: the argument must be sorted!
    fn with_sorted(sorted: Vec<(N::K, N::V)>) -> Self {
        Self::with_repr(TreeRepr::with_sorted(sorted))
    }

    fn with_nodes(nodes: Vec<Option<N>>) -> Self {
        Self::with_repr(TreeRepr::with_nodes(nodes))
    }


    fn with_repr(repr: TreeRepr<N>) -> Self;

    unsafe fn with_shape(items: Vec<Option<(N::K, N::V)>>) -> Self;
}
