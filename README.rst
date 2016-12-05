=============
teardown_tree
=============

`API docs <https://docs.rs/teardown_tree/>`_

|crates|_

.. |crates| image:: http://meritbadge.herokuapp.com/teardown_tree
.. _crates: https://crates.io/crates/teardown_tree

A BST (binary search tree) written in Rust that supports efficient **teardown** scenarios, i.e. the typical usage
pattern is to build a master copy of the tree, then

1. **clone** the master copy to a new tree
2. tear the tree down with a series of **delete-range** operations (and do something with the retrieved items)
3. rinse, repeat

The tree does not use any kind of self-balancing and does not support insert operation.


-------
Details
-------

The tree is implicit -- meaning that nodes do not store explicit pointers to their children. This is similar to how
binary heaps work: all nodes in the tree reside in an array, the root always at index 0, and given a node with index i,
its left/right children are found at indices ``2*i+1`` and ``2*i+2``. Thus no dynamic memory allocation or deallocation is
done. This makes it possible to implement a fast **clone** operation: instead of traversing the tree, allocating and
copying each node individually, we are able to allocate the whole array in a single call and efficiently copy the entire
content.

As to **delete-range** operation, we use a custom algorithm running in ``O(k + log n)`` time, where k is the number of
items deleted (and returned) and n is the initial size of the tree. `Detailed description <https://github.com/kirillkh/rs_teardown_tree/blob/master/delete_range.md>`_.
 
An exhaustive automated test for **delete-range** has been written and is found in ``lib.rs``. I have tested all trees up
to the size n=10.


-----
Usage
-----

As a library
------------
| Add to your Cargo.toml:
|
|     ``[dependencies]``
|     ``teardown_tree = "0.4.5"``



To run the benchmarks
---------------------
1. Install Rust and Cargo (any recent version will do, stable or nightly).
2. ``git clone https://github.com/kirillkh/rs_teardown_tree.git``
3. ``cd rs_teardown_tree``
4. ``cargo run --release``



----------
Benchmarks
----------

I have so far only performed a very limited set of benchmarks, comparing
my own implementation (which is geared for a very specialized use case)
against the BTreeSet in Rust's standard library. Truth be told, the comparison
is unfair, considering that BTreeSet lacks a way to efficiently delete ranges
(it has an ``O(log n)`` ``split``, but not ``merge``, see `Rust #34666 <https://github.com/rust-lang/rust/issues/34666>`_). That
said, on my machine the whole clone/teardown sequence on a tree of 1,000,000
items (we clone the tree, then delete 1000 items at a time until the tree
is empty), is ~10 times faster with ``delete_range`` implementation than with
BTreeSet. It also uses 20% less memory (39 vs 50 MB for 1,000,000 u64 items).
You can see the rest of the benchmarks by compiling the project and running
the ``benchmarks`` binary.

