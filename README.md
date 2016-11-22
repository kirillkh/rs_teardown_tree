An implicit BST (binary search tree), written in Rust, that supports **bulk-delete** and fast **clone** operations.

The tree does not use any kind of autobalancing (at least at the moment) and does not support insert operation. Thus the
typical usage pattern is to build a master copy of the tree, then

1. clone the master copy to a new tree
2. tear the tree down with a sequence of bulk delete operations
3. rinse, repeat


-------
### Details

"Implicit" means that nodes do not store explicit pointers to their children. This is similar to how binary heaps work:
all nodes in the tree reside in an array, the root always at index 0, and given a node with index i, its left/right 
children are found at indices `2*i` and `2*i+1`. Thus no dynamic memory allocation or deallocation is done. This makes
it possible to implement a fast **clone** operation: instead of traversing the tree, allocating and copying each node
individually, we are able to allocate the whole array in a single call and efficiently copy the entire content.

As to **bulk-delete** operation, we use a custom algorithm running in `O(k + log n)` time, where k is the number of 
items deleted (and returned) and n is the initial size of the tree.
 
An exhaustive automated test for **bulk-delete** has been written and is found in `lib.rs`. I have tested all trees up 
to the size n=10.
    
    
    
**TODO 1**: benchmarks in comparison to typical AVL/RB/explicit-array-based (the last is the tough one!)

**TODO 2**: we currently allocate space for 4*n+3 items in order to avoid bounds checking. We can at least improve that 
            by a factor of 2 in the best case (complete tree). Also need to investigate doing bounds checking instead.
