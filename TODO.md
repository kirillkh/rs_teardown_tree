
1. detect when the subtree is fully covered by the interval and handle specially
1. detect when the subtree is not covered by the interval (we are only interested in filling min or max slots) and handle specially
1. specialize the delete_recurse function to 2 cases: height(left)<height(right) and the opposite
1. implement the DSW algorithm (for building the master tree)
1. try to use two arrays (Vec<T> and Vec<bool>) instead of Vec<Option<T>>
1. try to remove height entirely
1. benchmarks in comparison to typical AVL/RB/Treap/explicit-array-based implementation
1. we currently allocate space for 4*n+3 items in order to avoid bounds checking. We can at least improve that
   by a factor of 2 in the best case (complete tree). Also need to investigate doing bounds checking instead.
