A BST (binary search tree) written in Rust that supports efficient **teardown** scenarios, i.e. the typical usage
pattern is to build a master copy of the tree, then

1. **clone** the master copy to a new tree
2. tear the tree down with a series of **delete-range** operations
3. rinse, repeat

The tree does not use any kind of self-balancing and does not support insert operation.


-------
### Details

The tree is implicit -- meaning that nodes do not store explicit pointers to their children. This is similar to how
binary heaps work: all nodes in the tree reside in an array, the root always at index 0, and given a node with index i,
its left/right children are found at indices `2*i` and `2*i+1`. Thus no dynamic memory allocation or deallocation is
done. This makes it possible to implement a fast **clone** operation: instead of traversing the tree, allocating and
copying each node individually, we are able to allocate the whole array in a single call and efficiently copy the entire
content.

As to **delete-range** operation, we use a custom algorithm running in `O(k + log n)` time, where k is the number of 
items deleted (and returned) and n is the initial size of the tree. [Detailed description][1].
 
An exhaustive automated test for **delete-range** has been written and is found in `lib.rs`. I have tested all trees up 
to the size n=10.

[1]: https://github.com/kirillkh/rs_teardown_tree/blob/master/delete_range.md